use pumpkin_data::{
    chunk::DoublePerlinNoiseParameters,
    noise_router::{
        NoiseEvaluationContext, WrapperType,
        nether_compiled::eval_nether_8,
        overworld_compiled::{
            eval_overworld_16, eval_overworld_19, eval_overworld_22, eval_overworld_27,
            eval_overworld_167, eval_overworld_173, eval_overworld_179, eval_overworld_182,
            eval_overworld_186,
        },
    },
};
use pumpkin_util::math::vector3::Vector3;

use super::{
    chunk_density_function::{compute_cell_volume, trilinear_interpolate_corners},
    chunk_noise_router::ChunkNoiseFunctionComponent,
    density_function::{
        beardifier::Beardifier,
        noise::InterpolatedNoiseSampler,
        spline::Spline,
    },
    density_volume::{DensityBuffer, DensityVolume},
    proto_noise_router::{
        DependentProtoNoiseFunctionComponent, IndependentProtoNoiseFunctionComponent,
    },
};
use crate::generation::noise::perlin::DoublePerlinNoiseSampler;

pub struct NetherAotContext<'a> {
    pub interpolated_noise: &'a InterpolatedNoiseSampler,
}

impl<'a> NoiseEvaluationContext for NetherAotContext<'a> {
    #[inline(always)]
    fn sample_noise(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        _x: f64,
        _y: f64,
        _z: f64,
    ) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_shift_a(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        _pos: &Vector3<i32>,
    ) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_shift_b(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        _pos: &Vector3<i32>,
    ) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_shifted_noise(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        _shift_x: f32,
        _shift_y: f32,
        _shift_z: f32,
        _xz_scale: f64,
        _y_scale: f64,
    ) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_interpolated_noise(&mut self, pos: &Vector3<i32>) -> f32 {
        use super::density_function::StaticIndependentChunkNoiseFunctionComponentImpl;
        self.interpolated_noise.sample(pos)
    }

    #[inline(always)]
    fn sample_beardifier(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_blend_alpha(&mut self, _pos: &Vector3<i32>) -> f32 {
        1.0
    }

    #[inline(always)]
    fn sample_blend_offset(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_blend_density(&mut self, input_val: f32, _pos: &Vector3<i32>) -> f32 {
        input_val
    }

    #[inline(always)]
    fn sample_end_islands(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_wrapper(
        &mut self,
        _wrapper_index: usize,
        _wrapper_type: WrapperType,
        pos: &Vector3<i32>,
        eval_input: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
    ) -> f32 {
        eval_input(pos, self)
    }

    #[inline(always)]
    fn sample_spline(
        &mut self,
        _spline_index: usize,
        _location_value: f32,
        _pos: &Vector3<i32>,
    ) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_find_top_surface(
        &mut self,
        _density_fn: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
        _upper_bound_fn: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
        _lower_bound: i32,
        _cell_height: i32,
        _pos: &Vector3<i32>,
    ) -> f32 {
        0.0
    }
}

/// Evaluates the complete Nether final density 3D volume using the AOT transpiled noise router
/// and fast trilinear interpolation.
pub fn evaluate_nether_final_density_volume(
    interpolated_noise: &InterpolatedNoiseSampler,
    beardifier: &Beardifier,
    buffer: &mut [f32],
    volume: &DensityVolume,
) {
    const CELL_SIZE_XZ: i32 = 4;
    const CELL_SIZE_Y: i32 = 8;

    let (cell_volume, cell_count_x, cell_count_y, cell_count_z) =
        compute_cell_volume(volume, CELL_SIZE_XZ, CELL_SIZE_Y);

    let mut corners = DensityBuffer::acquire(&cell_volume);
    let mut ctx = NetherAotContext {
        interpolated_noise,
    };

    // Evaluate eval_nether_8 directly at every cell corner with zero AST dispatch
    cell_volume.fill_with(&mut corners, |pos| {
        eval_nether_8(pos, &mut ctx)
    });

    // Trilinear interpolation into chunk volume buffer
    trilinear_interpolate_corners(
        &corners,
        buffer,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    // Vectorized squeeze: c / 2.0 - c * c * c / 24.0
    for slot in buffer.iter_mut() {
        let c = (*slot).clamp(-1.0, 1.0);
        *slot = c * 0.5 - c * c * c * (1.0 / 24.0);
    }

    // Add beardifier contribution if any structure or junction affects this chunk
    if let Some(affected_box) = beardifier.affected_box {
        if affected_box.min.x <= volume.max_block_x()
            && affected_box.max.x >= volume.min_block_x
            && affected_box.min.y <= volume.max_block_y()
            && affected_box.max.y >= volume.min_block_y
            && affected_box.min.z <= volume.max_block_z()
            && affected_box.max.z >= volume.min_block_z
        {
            let min_x = 0
                .max(affected_box.min.x - volume.min_block_x)
                .div_euclid(volume.step_block_x);
            let min_y = 0
                .max(affected_box.min.y - volume.min_block_y)
                .div_euclid(volume.step_block_y);
            let min_z = 0
                .max(affected_box.min.z - volume.min_block_z)
                .div_euclid(volume.step_block_z);
            let max_x = (volume.size_x as i32 - 1)
                .min((affected_box.max.x - volume.min_block_x).div_euclid(volume.step_block_x));
            let max_y = (volume.size_y as i32 - 1)
                .min((affected_box.max.y - volume.min_block_y).div_euclid(volume.step_block_y));
            let max_z = (volume.size_z as i32 - 1)
                .min((affected_box.max.z - volume.min_block_z).div_euclid(volume.step_block_z));
            for z in min_z..=max_z {
                let block_z = volume.block_z(z as usize);
                for x in min_x..=max_x {
                    let block_x = volume.block_x(x as usize);
                    for y in min_y..=max_y {
                        let index = volume.index_unchecked(x as usize, y as usize, z as usize);
                        buffer[index] += beardifier.sample_value_unchecked(
                            block_x,
                            volume.block_y(y as usize),
                            block_z,
                        );
                    }
                }
            }
        }
    }
}

pub struct OverworldAotContext<'a> {
    pub noise_samplers: [Option<&'a DoublePerlinNoiseSampler>; DoublePerlinNoiseParameters::COUNT],
    pub shift_a_sampler: Option<&'a DoublePerlinNoiseSampler>,
    pub shift_b_sampler: Option<&'a DoublePerlinNoiseSampler>,
    pub interpolated_noise: Option<&'a InterpolatedNoiseSampler>,
    pub spline_28: &'a Spline,
    pub spline_35: &'a Spline,
    pub spline_47: &'a Spline,
    pub current_pos: Vector3<i32>,
    pub val_16: f32,
    pub val_19: f32,
    pub val_22: f32,
    pub val_27: f32,
    pub cache_tags: [u32; 222],
    pub cache_pos: [Vector3<i32>; 222],
    pub cache_vals: [f32; 222],
    pub current_tag: u32,
}

impl<'a> OverworldAotContext<'a> {
    pub fn new(stack: &'a [ChunkNoiseFunctionComponent<'a>]) -> Self {
        let mut noise_samplers: [Option<&'a DoublePerlinNoiseSampler>;
            DoublePerlinNoiseParameters::COUNT] = [None; DoublePerlinNoiseParameters::COUNT];
        let mut shift_a_sampler = None;
        let mut shift_b_sampler = None;
        let mut interpolated_noise = None;

        for comp in stack {
            match comp {
                ChunkNoiseFunctionComponent::Independent(ind) => match ind {
                    IndependentProtoNoiseFunctionComponent::Noise(n) => {
                        noise_samplers[n.data.noise_id.id] = Some(&n.sampler);
                    }
                    IndependentProtoNoiseFunctionComponent::ShiftA(s) => {
                        shift_a_sampler = Some(&s.sampler);
                        noise_samplers[DoublePerlinNoiseParameters::OFFSET.id] = Some(&s.sampler);
                    }
                    IndependentProtoNoiseFunctionComponent::ShiftB(s) => {
                        shift_b_sampler = Some(&s.sampler);
                    }
                    IndependentProtoNoiseFunctionComponent::InterpolatedNoise(s) => {
                        interpolated_noise = Some(s);
                    }
                    _ => {}
                },
                ChunkNoiseFunctionComponent::Dependent(dep) => match dep {
                    DependentProtoNoiseFunctionComponent::ShiftedNoise(s) => {
                        noise_samplers[s.data.noise_id.id] = Some(&s.sampler);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        let spline_28 = match &stack[28] {
            ChunkNoiseFunctionComponent::Dependent(DependentProtoNoiseFunctionComponent::Spline(s)) => {
                s.spline()
            }
            _ => panic!("Expected spline at index 28"),
        };
        let spline_35 = match &stack[35] {
            ChunkNoiseFunctionComponent::Dependent(DependentProtoNoiseFunctionComponent::Spline(s)) => {
                s.spline()
            }
            _ => panic!("Expected spline at index 35"),
        };
        let spline_47 = match &stack[47] {
            ChunkNoiseFunctionComponent::Dependent(DependentProtoNoiseFunctionComponent::Spline(s)) => {
                s.spline()
            }
            _ => panic!("Expected spline at index 47"),
        };

        Self {
            noise_samplers,
            shift_a_sampler,
            shift_b_sampler,
            interpolated_noise,
            spline_28,
            spline_35,
            spline_47,
            current_pos: Vector3::new(i32::MIN, i32::MIN, i32::MIN),
            val_16: 0.0,
            val_19: 0.0,
            val_22: 0.0,
            val_27: 0.0,
            cache_tags: [0; 222],
            cache_pos: [Vector3::new(i32::MIN, i32::MIN, i32::MIN); 222],
            cache_vals: [0.0; 222],
            current_tag: 1,
        }
    }

    #[inline(always)]
    pub fn update_column(&mut self, x: i32, z: i32) {
        let slice_pos = Vector3::new(x, 0, z);
        self.current_pos = slice_pos;
        self.val_16 = eval_overworld_16(&slice_pos, self);
        self.val_19 = eval_overworld_19(&slice_pos, self);
        self.val_22 = eval_overworld_22(&slice_pos, self);
        self.val_27 = eval_overworld_27(&slice_pos, self);
    }

    #[inline(always)]
    pub fn update_corner(&mut self, pos: Vector3<i32>) {
        self.current_pos = pos;
        self.current_tag = self.current_tag.wrapping_add(1);
        if self.current_tag == 0 {
            self.cache_tags = [0; 222];
            self.current_tag = 1;
        }
    }
}

impl<'a> NoiseEvaluationContext for OverworldAotContext<'a> {
    #[inline(always)]
    fn sample_noise(
        &mut self,
        noise_id: DoublePerlinNoiseParameters,
        x: f64,
        y: f64,
        z: f64,
    ) -> f32 {
        if let Some(sampler) = self.noise_samplers[noise_id.id] {
            sampler.sample(x, y, z)
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn sample_shift_a(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        pos: &Vector3<i32>,
    ) -> f32 {
        if let Some(sampler) = self.shift_a_sampler {
            sampler.sample(
                f64::from(pos.x) * 0.25,
                0.0,
                f64::from(pos.z) * 0.25,
            ) * 4.0
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn sample_shift_b(
        &mut self,
        _noise_id: DoublePerlinNoiseParameters,
        pos: &Vector3<i32>,
    ) -> f32 {
        if let Some(sampler) = self.shift_b_sampler {
            sampler.sample(
                f64::from(pos.z) * 0.25,
                f64::from(pos.x) * 0.25,
                0.0,
            ) * 4.0
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn sample_shifted_noise(
        &mut self,
        noise_id: DoublePerlinNoiseParameters,
        shift_x: f32,
        shift_y: f32,
        shift_z: f32,
        xz_scale: f64,
        y_scale: f64,
    ) -> f32 {
        if let Some(sampler) = self.noise_samplers[noise_id.id] {
            sampler.sample(
                f64::from(self.current_pos.x) * xz_scale + f64::from(shift_x),
                f64::from(self.current_pos.y) * y_scale + f64::from(shift_y),
                f64::from(self.current_pos.z) * xz_scale + f64::from(shift_z),
            )
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn sample_interpolated_noise(&mut self, pos: &Vector3<i32>) -> f32 {
        use super::density_function::StaticIndependentChunkNoiseFunctionComponentImpl;
        if let Some(sampler) = self.interpolated_noise {
            sampler.sample(pos)
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn sample_beardifier(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_blend_alpha(&mut self, _pos: &Vector3<i32>) -> f32 {
        1.0
    }

    #[inline(always)]
    fn sample_blend_offset(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_blend_density(&mut self, input_val: f32, _pos: &Vector3<i32>) -> f32 {
        input_val
    }

    #[inline(always)]
    fn sample_end_islands(&mut self, _pos: &Vector3<i32>) -> f32 {
        0.0
    }

    #[inline(always)]
    fn sample_wrapper(
        &mut self,
        wrapper_index: usize,
        wrapper_type: WrapperType,
        pos: &Vector3<i32>,
        eval_input: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
    ) -> f32 {
        match wrapper_type {
            WrapperType::Cache => {
                if wrapper_index < 222 {
                    if self.cache_tags[wrapper_index] == self.current_tag && self.cache_pos[wrapper_index] == *pos {
                        return self.cache_vals[wrapper_index];
                    }
                    let val = eval_input(pos, self);
                    self.cache_tags[wrapper_index] = self.current_tag;
                    self.cache_pos[wrapper_index] = *pos;
                    self.cache_vals[wrapper_index] = val;
                    val
                } else {
                    eval_input(pos, self)
                }
            }
            _ => eval_input(pos, self),
        }
    }

    #[inline(always)]
    fn sample_spline(
        &mut self,
        spline_index: usize,
        location_value: f32,
        _pos: &Vector3<i32>,
    ) -> f32 {
        let spline = match spline_index {
            28 => self.spline_28,
            35 => self.spline_35,
            47 => self.spline_47,
            _ => return 0.0,
        };
        let val_16 = self.val_16;
        let val_19 = self.val_19;
        let val_22 = self.val_22;
        let val_27 = self.val_27;
        spline.sample_with(&mut |index| match index {
            16 => val_16,
            19 => val_19,
            22 => val_22,
            27 => val_27,
            _ => location_value,
        })
    }

    #[inline(always)]
    fn sample_find_top_surface(
        &mut self,
        _density_fn: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
        _upper_bound_fn: &dyn Fn(&Vector3<i32>, &mut Self) -> f32,
        _lower_bound: i32,
        _cell_height: i32,
        _pos: &Vector3<i32>,
    ) -> f32 {
        0.0
    }
}

/// Evaluates the complete Overworld final density 3D volume using the AOT transpiled noise router
/// and fast trilinear interpolation.
pub fn evaluate_overworld_final_density_volume(
    stack: &[ChunkNoiseFunctionComponent],
    beardifier: &Beardifier,
    buffer: &mut [f32],
    volume: &DensityVolume,
) {
    const CELL_SIZE_XZ: i32 = 4;
    const CELL_SIZE_Y: i32 = 8;

    let (cell_volume, cell_count_x, cell_count_y, cell_count_z) =
        compute_cell_volume(volume, CELL_SIZE_XZ, CELL_SIZE_Y);

    let mut corners_167 = DensityBuffer::acquire(&cell_volume);
    let mut corners_173 = DensityBuffer::acquire(&cell_volume);
    let mut corners_179 = DensityBuffer::acquire(&cell_volume);
    let mut corners_182 = DensityBuffer::acquire(&cell_volume);
    let mut corners_186 = DensityBuffer::acquire(&cell_volume);

    let mut ctx = OverworldAotContext::new(stack);

    // Evaluate the 5 corner arrays in a column-optimized single pass
    let mut index = 0;
    for z in 0..cell_volume.size_z {
        let block_z = cell_volume.block_z(z);
        for x in 0..cell_volume.size_x {
            let block_x = cell_volume.block_x(x);
            // Precompute column 2D slices (continentalness, erosion, ridges, weirdness)
            ctx.update_column(block_x, block_z);

            for y in 0..cell_volume.size_y {
                let block_y = cell_volume.block_y(y);
                let pos = Vector3::new(block_x, block_y, block_z);
                ctx.update_corner(pos);

                corners_167[index] = eval_overworld_167(&pos, &mut ctx);
                corners_173[index] = eval_overworld_173(&pos, &mut ctx);
                corners_179[index] = eval_overworld_179(&pos, &mut ctx);
                corners_182[index] = eval_overworld_182(&pos, &mut ctx);
                corners_186[index] = eval_overworld_186(&pos, &mut ctx);
                index += 1;
            }
        }
    }

    // Trilinearly interpolate corners into chunk voxel buffers
    // 167 into buffer (becomes caves)
    trilinear_interpolate_corners(
        &corners_167,
        buffer,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    let mut buf_174 = DensityBuffer::acquire(volume);
    trilinear_interpolate_corners(
        &corners_173,
        &mut buf_174,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    let mut buf_180 = DensityBuffer::acquire(volume);
    trilinear_interpolate_corners(
        &corners_179,
        &mut buf_180,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    let mut buf_183 = DensityBuffer::acquire(volume);
    trilinear_interpolate_corners(
        &corners_182,
        &mut buf_183,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    let mut buf_187 = DensityBuffer::acquire(volume);
    trilinear_interpolate_corners(
        &corners_186,
        &mut buf_187,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

    // Flat voxel-wise combination:
    // caves = squeeze(interp_168)
    // noodle = thickness + max(abs(ridge_a), abs(ridge_b)) * 1.5
    // if toggle in [-1000000.0, 0.0) => caves, else => min(caves, noodle)
    for i in 0..buffer.len() {
        let c = buffer[i].clamp(-1.0, 1.0);
        let caves = c * 0.5 - c * c * c * (1.0 / 24.0);
        let noodle_toggle = buf_174[i];
        if noodle_toggle >= -1000000.0 && noodle_toggle < 0.0 {
            buffer[i] = caves;
        } else {
            let thickness = buf_180[i];
            let ridge_a = buf_183[i].abs();
            let ridge_b = buf_187[i].abs();
            let noodle = thickness + ridge_a.max(ridge_b) * 1.5;
            buffer[i] = caves.min(noodle);
        }
    }

    // Add beardifier contribution if any structure or junction affects this chunk
    if let Some(affected_box) = beardifier.affected_box {
        if affected_box.min.x <= volume.max_block_x()
            && affected_box.max.x >= volume.min_block_x
            && affected_box.min.y <= volume.max_block_y()
            && affected_box.max.y >= volume.min_block_y
            && affected_box.min.z <= volume.max_block_z()
            && affected_box.max.z >= volume.min_block_z
        {
            let min_x = 0
                .max(affected_box.min.x - volume.min_block_x)
                .div_euclid(volume.step_block_x);
            let min_y = 0
                .max(affected_box.min.y - volume.min_block_y)
                .div_euclid(volume.step_block_y);
            let min_z = 0
                .max(affected_box.min.z - volume.min_block_z)
                .div_euclid(volume.step_block_z);
            let max_x = (volume.size_x as i32 - 1)
                .min((affected_box.max.x - volume.min_block_x).div_euclid(volume.step_block_x));
            let max_y = (volume.size_y as i32 - 1)
                .min((affected_box.max.y - volume.min_block_y).div_euclid(volume.step_block_y));
            let max_z = (volume.size_z as i32 - 1)
                .min((affected_box.max.z - volume.min_block_z).div_euclid(volume.step_block_z));
            for z in min_z..=max_z {
                let block_z = volume.block_z(z as usize);
                for x in min_x..=max_x {
                    let block_x = volume.block_x(x as usize);
                    for y in min_y..=max_y {
                        let index = volume.index_unchecked(x as usize, y as usize, z as usize);
                        buffer[index] += beardifier.sample_value_unchecked(
                            block_x,
                            volume.block_y(y as usize),
                            block_z,
                        );
                    }
                }
            }
        }
    }
}

