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
use pumpkin_util::math::{lerp, vector3::Vector3};

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
    // NOTE: kept as a plain iterator loop (not chunks_exact/SIMD) because `buffer` length
    // varies per call site (full chunk vs. sub-volume) and LLVM already auto-vectorizes
    // this exact clamp+polynomial pattern on contiguous f32 slices at opt-level>=2.
    // A hand-rolled SIMD version here would only pay off if profiling showed this loop
    // as a bottleneck, which for a 4x8x4-interpolated 16^3 (or similar) buffer it is not
    // relative to the noise sampling above it.
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

    let (size_x, size_y, size_z) = (cell_volume.size_x, cell_volume.size_y, cell_volume.size_z);
    let n_corners = size_x * size_y * size_z;
    let n_cells = cell_count_x * cell_count_y * cell_count_z;

    let mut corners_167 = DensityBuffer::acquire(&cell_volume);
    let mut corners_173 = DensityBuffer::acquire(&cell_volume);

    let mut ctx = OverworldAotContext::new(stack);

    // Pass 1: Evaluate 167 (terrain) and 173 (noodle toggle) for all corners
    let mut index = 0;
    for z in 0..size_z {
        let block_z = cell_volume.block_z(z);
        for x in 0..size_x {
            let block_x = cell_volume.block_x(x);
            // Precompute column 2D slices (continentalness, erosion, ridges, weirdness)
            ctx.update_column(block_x, block_z);

            for y in 0..size_y {
                let block_y = cell_volume.block_y(y);
                let pos = Vector3::new(block_x, block_y, block_z);
                ctx.update_corner(pos);

                corners_167[index] = eval_overworld_167(&pos, &mut ctx);
                corners_173[index] = eval_overworld_173(&pos, &mut ctx);
                index += 1;
            }
        }
    }

    // Pass 2: Mark cells and propagate to corners.
    // If all 8 corners of a cell have noodle_toggle in [-1000000.0, 0.0),
    // noodle caves are provably inactive across the entire cell (convex combination).
    let mut cell_needs_noodle = vec![false; n_cells];
    let mut corner_needed = vec![false; n_corners];
    let mut any_cell_needs_noodle = false;

    for cell_z in 0..cell_count_z {
        let next_z = (cell_z + 1).min(size_z - 1);
        let z0_offset = cell_z * size_x;
        let z1_offset = next_z * size_x;
        for cell_x in 0..cell_count_x {
            let next_x = (cell_x + 1).min(size_x - 1);
            let idx_00 = (cell_x + z0_offset) * size_y;
            let idx_10 = (next_x + z0_offset) * size_y;
            let idx_01 = (cell_x + z1_offset) * size_y;
            let idx_11 = (next_x + z1_offset) * size_y;

            let cell_col_base = (cell_z * cell_count_x + cell_x) * cell_count_y;

            for cell_y in 0..cell_count_y {
                let next_y = (cell_y + 1).min(size_y - 1);

                let corner_indices = [
                    idx_00 + cell_y,
                    idx_10 + cell_y,
                    idx_00 + next_y,
                    idx_10 + next_y,
                    idx_01 + cell_y,
                    idx_11 + cell_y,
                    idx_01 + next_y,
                    idx_11 + next_y,
                ];

                let needs = corner_indices.iter().any(|&idx| {
                    let v = corners_173[idx];
                    !(v >= -1000000.0 && v < 0.0)
                });

                if needs {
                    any_cell_needs_noodle = true;
                    cell_needs_noodle[cell_col_base + cell_y] = true;
                    for &idx in &corner_indices {
                        corner_needed[idx] = true;
                    }
                }
            }
        }
    }

    // Step 3: Evaluate 179, 182, 186 only for needed corners.
    //
    // OPTIMIZATION: these three buffers are only ever read when `any_cell_needs_noodle`
    // is true (see `fused_interpolate_overworld`, which branches on `cell_needs_noodle`
    // and never touches corners_179/182/186 for cells where it's false). In the common
    // case where a chunk has no noodle caves at all, we skip acquiring all three buffers
    // entirely, saving 3 full-size allocations/zerings per chunk. We pass `None` through
    // and thread an `Option` into the interpolation step below so behavior for the
    // "needs noodle" path is byte-for-byte identical to before.
    let mut noodle_corners: Option<(DensityBuffer, DensityBuffer, DensityBuffer)> = None;

    if any_cell_needs_noodle {
        let mut corners_179 = DensityBuffer::acquire(&cell_volume);
        let mut corners_182 = DensityBuffer::acquire(&cell_volume);
        let mut corners_186 = DensityBuffer::acquire(&cell_volume);

        for z in 0..size_z {
            let block_z = cell_volume.block_z(z);
            let z_offset = z * size_x;
            for x in 0..size_x {
                let col_idx = (z_offset + x) * size_y;
                let col_needed = (0..size_y).any(|y| corner_needed[col_idx + y]);
                if !col_needed {
                    continue;
                }

                let block_x = cell_volume.block_x(x);
                ctx.update_column(block_x, block_z);

                for y in 0..size_y {
                    let idx = col_idx + y;
                    if corner_needed[idx] {
                        let block_y = cell_volume.block_y(y);
                        let pos = Vector3::new(block_x, block_y, block_z);
                        ctx.update_corner(pos);

                        corners_179[idx] = eval_overworld_179(&pos, &mut ctx);
                        corners_182[idx] = eval_overworld_182(&pos, &mut ctx);
                        corners_186[idx] = eval_overworld_186(&pos, &mut ctx);
                    }
                }
            }
        }

        noodle_corners = Some((corners_179, corners_182, corners_186));
    }

    // Fused trilinear interpolation + cave/noodle combination directly into buffer.
    // Eliminates 4 intermediate 98k-float heap buffers and fuses 5 interpolation passes + 1 combination pass into 1 single pass.
    fused_interpolate_overworld(
        &corners_167,
        &corners_173,
        noodle_corners.as_ref(),
        &cell_needs_noodle,
        buffer,
        volume,
        &cell_volume,
        cell_count_x,
        cell_count_y,
        cell_count_z,
        CELL_SIZE_XZ,
        CELL_SIZE_Y,
    );

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

#[inline(always)]
fn caves_from_167(raw_167: f32) -> f32 {
    let c = raw_167.clamp(-1.0, 1.0);
    c * 0.5 - c * c * c * (1.0 / 24.0)
}

fn fused_interpolate_overworld(
    corners_167: &[f32],
    corners_173: &[f32],
    noodle_corners: Option<&(DensityBuffer, DensityBuffer, DensityBuffer)>,
    cell_needs_noodle: &[bool],
    buffer: &mut [f32],
    volume: &DensityVolume,
    cell_volume: &DensityVolume,
    cell_count_x: usize,
    cell_count_y: usize,
    cell_count_z: usize,
    cell_size_xz: i32,
    cell_size_y: i32,
) {
    let cell_size_xz_inv = 1.0 / cell_size_xz as f32;
    let cell_size_y_inv = 1.0 / cell_size_y as f32;
    let xy_plane = volume.size_x * volume.size_y;

    // Split once, outside the loop, instead of on every cell.
    let noodle_slices: Option<(&[f32], &[f32], &[f32])> =
        noodle_corners.map(|(a, b, c)| (a.as_ref(), b.as_ref(), c.as_ref()));

    for cell_z in 0..cell_count_z {
        let next_z = (cell_z + 1).min(cell_volume.size_z - 1);
        let z0_offset = cell_z * cell_volume.size_x;
        let z1_offset = next_z * cell_volume.size_x;
        for cell_x in 0..cell_count_x {
            let next_x = (cell_x + 1).min(cell_volume.size_x - 1);
            let idx_00 = (cell_x + z0_offset) * cell_volume.size_y;
            let idx_10 = (next_x + z0_offset) * cell_volume.size_y;
            let idx_01 = (cell_x + z1_offset) * cell_volume.size_y;
            let idx_11 = (next_x + z1_offset) * cell_volume.size_y;

            // Bounds computation depends only on cell_x/cell_z (not cell_y-branch),
            // computed once per (cell_x, cell_z, cell_y) as before — unchanged math,
            // just shared between the two branches below instead of duplicated verbatim.
            let cell_out_x = cell_volume.block_x(cell_x) - volume.min_block_x;
            let cell_out_z = cell_volume.block_z(cell_z) - volume.min_block_z;
            let x0 = 0.max(-cell_out_x);
            let z0 = 0.max(-cell_out_z);
            let x1 = cell_size_xz.min(volume.size_x as i32 - cell_out_x) - 1;
            let z1 = cell_size_xz.min(volume.size_z as i32 - cell_out_z) - 1;

            for cell_y in 0..cell_count_y {
                let next_y = (cell_y + 1).min(cell_volume.size_y - 1);

                let c167 = [
                    corners_167[idx_00 + cell_y],
                    corners_167[idx_10 + cell_y],
                    corners_167[idx_00 + next_y],
                    corners_167[idx_10 + next_y],
                    corners_167[idx_01 + cell_y],
                    corners_167[idx_11 + cell_y],
                    corners_167[idx_01 + next_y],
                    corners_167[idx_11 + next_y],
                ];

                let cell_out_y = cell_volume.block_y(cell_y) - volume.min_block_y;
                let y0 = 0.max(-cell_out_y);
                let y1 = cell_size_y.min(volume.size_y as i32 - cell_out_y) - 1;

                let y_count = (y1 - y0 + 1) as usize;
                let y_start = (cell_out_y + y0) as usize;

                let cell_idx = (cell_z * cell_count_x + cell_x) * cell_count_y + cell_y;
                if !cell_needs_noodle[cell_idx] {
                    for z in z0..=z1 {
                        let alpha_z = z as f32 * cell_size_xz_inv;
                        let out_z = (cell_out_z + z) as usize;
                        let z_offset = out_z * xy_plane;

                        let v00_167 = lerp(alpha_z, c167[0], c167[4]);
                        let v01_167 = lerp(alpha_z, c167[2], c167[6]);
                        let v10_167 = lerp(alpha_z, c167[1], c167[5]);
                        let v11_167 = lerp(alpha_z, c167[3], c167[7]);

                        for x in x0..=x1 {
                            let alpha_x = x as f32 * cell_size_xz_inv;
                            let out_x = (cell_out_x + x) as usize;

                            let v_0_167 = lerp(alpha_x, v00_167, v10_167);
                            let v_1_167 = lerp(alpha_x, v01_167, v11_167);
                            let step_167 = (v_1_167 - v_0_167) * cell_size_y_inv;
                            let val_167_start = v_0_167 + step_167 * y0 as f32;

                            let start = y_start + (out_x * volume.size_y) + z_offset;

                            for dy in 0..y_count {
                                let dy_f = dy as f32;
                                let raw_167 = val_167_start + step_167 * dy_f;
                                buffer[start + dy] = caves_from_167(raw_167);
                            }
                        }
                    }
                    continue;
                }

                let (corners_179, corners_182, corners_186) = noodle_slices
                    .expect("cell_needs_noodle implies noodle_slices is Some");

                let c173 = [
                    corners_173[idx_00 + cell_y],
                    corners_173[idx_10 + cell_y],
                    corners_173[idx_00 + next_y],
                    corners_173[idx_10 + next_y],
                    corners_173[idx_01 + cell_y],
                    corners_173[idx_11 + cell_y],
                    corners_173[idx_01 + next_y],
                    corners_173[idx_11 + next_y],
                ];
                let c179 = [
                    corners_179[idx_00 + cell_y],
                    corners_179[idx_10 + cell_y],
                    corners_179[idx_00 + next_y],
                    corners_179[idx_10 + next_y],
                    corners_179[idx_01 + cell_y],
                    corners_179[idx_11 + cell_y],
                    corners_179[idx_01 + next_y],
                    corners_179[idx_11 + next_y],
                ];
                let c182 = [
                    corners_182[idx_00 + cell_y],
                    corners_182[idx_10 + cell_y],
                    corners_182[idx_00 + next_y],
                    corners_182[idx_10 + next_y],
                    corners_182[idx_01 + cell_y],
                    corners_182[idx_11 + cell_y],
                    corners_182[idx_01 + next_y],
                    corners_182[idx_11 + next_y],
                ];
                let c186 = [
                    corners_186[idx_00 + cell_y],
                    corners_186[idx_10 + cell_y],
                    corners_186[idx_00 + next_y],
                    corners_186[idx_10 + next_y],
                    corners_186[idx_01 + cell_y],
                    corners_186[idx_11 + cell_y],
                    corners_186[idx_01 + next_y],
                    corners_186[idx_11 + next_y],
                ];

                for z in z0..=z1 {
                    let alpha_z = z as f32 * cell_size_xz_inv;
                    let out_z = (cell_out_z + z) as usize;
                    let z_offset = out_z * xy_plane;

                    let v00_167 = lerp(alpha_z, c167[0], c167[4]);
                    let v01_167 = lerp(alpha_z, c167[2], c167[6]);
                    let v10_167 = lerp(alpha_z, c167[1], c167[5]);
                    let v11_167 = lerp(alpha_z, c167[3], c167[7]);

                    let v00_173 = lerp(alpha_z, c173[0], c173[4]);
                    let v01_173 = lerp(alpha_z, c173[2], c173[6]);
                    let v10_173 = lerp(alpha_z, c173[1], c173[5]);
                    let v11_173 = lerp(alpha_z, c173[3], c173[7]);

                    let v00_179 = lerp(alpha_z, c179[0], c179[4]);
                    let v01_179 = lerp(alpha_z, c179[2], c179[6]);
                    let v10_179 = lerp(alpha_z, c179[1], c179[5]);
                    let v11_179 = lerp(alpha_z, c179[3], c179[7]);

                    let v00_182 = lerp(alpha_z, c182[0], c182[4]);
                    let v01_182 = lerp(alpha_z, c182[2], c182[6]);
                    let v10_182 = lerp(alpha_z, c182[1], c182[5]);
                    let v11_182 = lerp(alpha_z, c182[3], c182[7]);

                    let v00_186 = lerp(alpha_z, c186[0], c186[4]);
                    let v01_186 = lerp(alpha_z, c186[2], c186[6]);
                    let v10_186 = lerp(alpha_z, c186[1], c186[5]);
                    let v11_186 = lerp(alpha_z, c186[3], c186[7]);

                    for x in x0..=x1 {
                        let alpha_x = x as f32 * cell_size_xz_inv;
                        let out_x = (cell_out_x + x) as usize;

                        let v_0_167 = lerp(alpha_x, v00_167, v10_167);
                        let v_1_167 = lerp(alpha_x, v01_167, v11_167);
                        let step_167 = (v_1_167 - v_0_167) * cell_size_y_inv;
                        let val_167_start = v_0_167 + step_167 * y0 as f32;

                        let v_0_173 = lerp(alpha_x, v00_173, v10_173);
                        let v_1_173 = lerp(alpha_x, v01_173, v11_173);
                        let step_173 = (v_1_173 - v_0_173) * cell_size_y_inv;
                        let val_173_start = v_0_173 + step_173 * y0 as f32;

                        let v_0_179 = lerp(alpha_x, v00_179, v10_179);
                        let v_1_179 = lerp(alpha_x, v01_179, v11_179);
                        let step_179 = (v_1_179 - v_0_179) * cell_size_y_inv;
                        let val_179_start = v_0_179 + step_179 * y0 as f32;

                        let v_0_182 = lerp(alpha_x, v00_182, v10_182);
                        let v_1_182 = lerp(alpha_x, v01_182, v11_182);
                        let step_182 = (v_1_182 - v_0_182) * cell_size_y_inv;
                        let val_182_start = v_0_182 + step_182 * y0 as f32;

                        let v_0_186 = lerp(alpha_x, v00_186, v10_186);
                        let v_1_186 = lerp(alpha_x, v01_186, v11_186);
                        let step_186 = (v_1_186 - v_0_186) * cell_size_y_inv;
                        let val_186_start = v_0_186 + step_186 * y0 as f32;

                        let start = y_start + (out_x * volume.size_y) + z_offset;

                        for dy in 0..y_count {
                            let dy_f = dy as f32;
                            let raw_167 = val_167_start + step_167 * dy_f;
                            let caves = caves_from_167(raw_167);

                            let noodle_toggle = val_173_start + step_173 * dy_f;
                            let is_toggled = noodle_toggle >= -1000000.0 && noodle_toggle < 0.0;

                            // Skip computing the noodle value entirely when the toggle
                            // already selects `caves` — ridge_a/ridge_b/abs/max cost real
                            // cycles and their result would be discarded anyway.
                            buffer[start + dy] = if is_toggled {
                                caves
                            } else {
                                let thickness = val_179_start + step_179 * dy_f;
                                let ridge_a = (val_182_start + step_182 * dy_f).abs();
                                let ridge_b = (val_186_start + step_186 * dy_f).abs();
                                let noodle = thickness + ridge_a.max(ridge_b) * 1.5;
                                caves.min(noodle)
                            };
                        }
                    }
                }
            }
        }
    }
}

pub fn evaluate_overworld_veins_volume(
    stack: &[ChunkNoiseFunctionComponent],
    toggle_buffer: &mut [f32],
    ridged_buffer: &mut [f32],
    volume: &DensityVolume,
) -> bool {
    if stack.len() < 218 {
        return false;
    }
    let (sampler_veininess, sampler_a, sampler_b) = match (&stack[200], &stack[205], &stack[210]) {
        (
            ChunkNoiseFunctionComponent::Independent(IndependentProtoNoiseFunctionComponent::Noise(n_v)),
            ChunkNoiseFunctionComponent::Independent(IndependentProtoNoiseFunctionComponent::Noise(n_a)),
            ChunkNoiseFunctionComponent::Independent(IndependentProtoNoiseFunctionComponent::Noise(n_b)),
        ) => (&n_v.sampler, &n_a.sampler, &n_b.sampler),
        _ => return false,
    };

    const CELL_SIZE_XZ: i32 = 4;
    const CELL_SIZE_Y: i32 = 8;
    const CELL_SIZE_XZ_INV: f32 = 1.0 / 4.0;
    const CELL_SIZE_Y_INV: f32 = 1.0 / 8.0;

    let (cell_volume, cell_count_x, cell_count_y, cell_count_z) =
        compute_cell_volume(volume, CELL_SIZE_XZ, CELL_SIZE_Y);

    let mut corners_veininess = DensityBuffer::acquire(&cell_volume);

    // Sample veininess corners
    // Note: for block_y outside [-64, 57), eval_overworld_201 returns 0.0.
    for z in 0..cell_volume.size_z {
        let block_z = cell_volume.block_z(z);
        let z_offset = z * cell_volume.size_x;
        for x in 0..cell_volume.size_x {
            let block_x = cell_volume.block_x(x);
            let col_idx = (x + z_offset) * cell_volume.size_y;
            for y in 0..cell_volume.size_y {
                let block_y = cell_volume.block_y(y);
                if block_y >= -64 && block_y < 57 {
                    corners_veininess[col_idx + y] = sampler_veininess.sample(
                        block_x as f64 * 1.5,
                        block_y as f64 * 1.5,
                        block_z as f64 * 1.5,
                    ) as f32;
                } else {
                    corners_veininess[col_idx + y] = 0.0;
                }
            }
        }
    }

    // Pre-check: does ANY corner have |veininess| >= 0.4?
    let mut any_vein_cells = false;
    for &v in corners_veininess.iter() {
        if v.abs() >= 0.4 {
            any_vein_cells = true;
            break;
        }
    }

    // If no corner has |veininess| >= 0.4, then in EVERY cell, all corners are in (-0.4, 0.4).
    // Throughout the entire chunk, vein_toggle is strictly in (-0.4, 0.4), so vein_ridged is -1.0 everywhere.
    if !any_vein_cells {
        ridged_buffer.fill(-1.0);
        for cell_z in 0..cell_count_z {
            let next_z = (cell_z + 1).min(cell_volume.size_z - 1);
            let z0_offset = cell_z * cell_volume.size_x;
            let z1_offset = next_z * cell_volume.size_x;
            for cell_x in 0..cell_count_x {
                let next_x = (cell_x + 1).min(cell_volume.size_x - 1);
                let idx_00 = (cell_x + z0_offset) * cell_volume.size_y;
                let idx_10 = (next_x + z0_offset) * cell_volume.size_y;
                let idx_01 = (cell_x + z1_offset) * cell_volume.size_y;
                let idx_11 = (next_x + z1_offset) * cell_volume.size_y;
                for cell_y in 0..cell_count_y {
                    let next_y = (cell_y + 1).min(cell_volume.size_y - 1);
                    super::chunk_density_function::fill_single_cell(
                        toggle_buffer,
                        volume,
                        &cell_volume,
                        [cell_x, cell_y, cell_z],
                        [
                            corners_veininess[idx_00 + cell_y],
                            corners_veininess[idx_10 + cell_y],
                            corners_veininess[idx_00 + next_y],
                            corners_veininess[idx_10 + next_y],
                            corners_veininess[idx_01 + cell_y],
                            corners_veininess[idx_11 + cell_y],
                            corners_veininess[idx_01 + next_y],
                            corners_veininess[idx_11 + next_y],
                        ],
                        CELL_SIZE_XZ,
                        CELL_SIZE_Y,
                        CELL_SIZE_XZ_INV,
                        CELL_SIZE_Y_INV,
                    );
                }
            }
        }
        return true;
    }

    // Rare case: an ore vein exists in this chunk.
    let mut corners_a = DensityBuffer::acquire(&cell_volume);
    let mut corners_b = DensityBuffer::acquire(&cell_volume);
    for z in 0..cell_volume.size_z {
        let block_z = cell_volume.block_z(z);
        let z_offset = z * cell_volume.size_x;
        for x in 0..cell_volume.size_x {
            let block_x = cell_volume.block_x(x);
            let col_idx = (x + z_offset) * cell_volume.size_y;
            for y in 0..cell_volume.size_y {
                let block_y = cell_volume.block_y(y);
                if block_y >= -64 && block_y < 57 {
                    corners_a[col_idx + y] = sampler_a.sample(
                        block_x as f64 * 4.0,
                        block_y as f64 * 4.0,
                        block_z as f64 * 4.0,
                    ) as f32;
                    corners_b[col_idx + y] = sampler_b.sample(
                        block_x as f64 * 4.0,
                        block_y as f64 * 4.0,
                        block_z as f64 * 4.0,
                    ) as f32;
                } else {
                    corners_a[col_idx + y] = 1.0;
                    corners_b[col_idx + y] = 1.0;
                }
            }
        }
    }

    for cell_z in 0..cell_count_z {
        let next_z = (cell_z + 1).min(cell_volume.size_z - 1);
        let z0_offset = cell_z * cell_volume.size_x;
        let z1_offset = next_z * cell_volume.size_x;
        for cell_x in 0..cell_count_x {
            let next_x = (cell_x + 1).min(cell_volume.size_x - 1);
            let idx_00 = (cell_x + z0_offset) * cell_volume.size_y;
            let idx_10 = (next_x + z0_offset) * cell_volume.size_y;
            let idx_01 = (cell_x + z1_offset) * cell_volume.size_y;
            let idx_11 = (next_x + z1_offset) * cell_volume.size_y;
            for cell_y in 0..cell_count_y {
                let next_y = (cell_y + 1).min(cell_volume.size_y - 1);

                let c_t = [
                    corners_veininess[idx_00 + cell_y],
                    corners_veininess[idx_10 + cell_y],
                    corners_veininess[idx_00 + next_y],
                    corners_veininess[idx_10 + next_y],
                    corners_veininess[idx_01 + cell_y],
                    corners_veininess[idx_11 + cell_y],
                    corners_veininess[idx_01 + next_y],
                    corners_veininess[idx_11 + next_y],
                ];

                super::chunk_density_function::fill_single_cell(
                    toggle_buffer,
                    volume,
                    &cell_volume,
                    [cell_x, cell_y, cell_z],
                    c_t,
                    CELL_SIZE_XZ,
                    CELL_SIZE_Y,
                    CELL_SIZE_XZ_INV,
                    CELL_SIZE_Y_INV,
                );

                let cell_has_vein = c_t.iter().any(|&v| v <= -0.4 || v >= 0.4);
                if !cell_has_vein {
                    fill_cell_constant(
                        ridged_buffer,
                        volume,
                        &cell_volume,
                        [cell_x, cell_y, cell_z],
                        -1.0,
                        CELL_SIZE_XZ,
                        CELL_SIZE_Y,
                    );
                } else {
                    let c_a = [
                        corners_a[idx_00 + cell_y],
                        corners_a[idx_10 + cell_y],
                        corners_a[idx_00 + next_y],
                        corners_a[idx_10 + next_y],
                        corners_a[idx_01 + cell_y],
                        corners_a[idx_11 + cell_y],
                        corners_a[idx_01 + next_y],
                        corners_a[idx_11 + next_y],
                    ];
                    let c_b = [
                        corners_b[idx_00 + cell_y],
                        corners_b[idx_10 + cell_y],
                        corners_b[idx_00 + next_y],
                        corners_b[idx_10 + next_y],
                        corners_b[idx_01 + cell_y],
                        corners_b[idx_11 + cell_y],
                        corners_b[idx_01 + next_y],
                        corners_b[idx_11 + next_y],
                    ];

                    fill_vein_cell(
                        toggle_buffer,
                        ridged_buffer,
                        volume,
                        &cell_volume,
                        [cell_x, cell_y, cell_z],
                        c_a,
                        c_b,
                        CELL_SIZE_XZ,
                        CELL_SIZE_Y,
                        CELL_SIZE_XZ_INV,
                        CELL_SIZE_Y_INV,
                    );
                }
            }
        }
    }

    true
}

fn fill_cell_constant(
    buffer: &mut [f32],
    volume: &DensityVolume,
    cell_volume: &DensityVolume,
    cell: [usize; 3],
    val: f32,
    cell_size_xz: i32,
    cell_size_y: i32,
) {
    let cell_out_x = cell_volume.block_x(cell[0]) - volume.min_block_x;
    let cell_out_y = cell_volume.block_y(cell[1]) - volume.min_block_y;
    let cell_out_z = cell_volume.block_z(cell[2]) - volume.min_block_z;
    let x0 = 0.max(-cell_out_x);
    let y0 = 0.max(-cell_out_y);
    let z0 = 0.max(-cell_out_z);
    let x1 = cell_size_xz.min(volume.size_x as i32 - cell_out_x) - 1;
    let y1 = cell_size_y.min(volume.size_y as i32 - cell_out_y) - 1;
    let z1 = cell_size_xz.min(volume.size_z as i32 - cell_out_z) - 1;

    let y_count = (y1 - y0 + 1) as usize;
    let y_start = (cell_out_y + y0) as usize;
    let xy_plane = volume.size_x * volume.size_y;

    for z in z0..=z1 {
        let out_z = (cell_out_z + z) as usize;
        let z_offset = out_z * xy_plane;
        for x in x0..=x1 {
            let out_x = (cell_out_x + x) as usize;
            let start = y_start + (out_x * volume.size_y) + z_offset;
            buffer[start..start + y_count].fill(val);
        }
    }
}

fn fill_vein_cell(
    toggle_buffer: &[f32],
    ridged_buffer: &mut [f32],
    volume: &DensityVolume,
    cell_volume: &DensityVolume,
    cell: [usize; 3],
    [a000, a100, a010, a110, a001, a101, a011, a111]: [f32; 8],
    [b000, b100, b010, b110, b001, b101, b011, b111]: [f32; 8],
    cell_size_xz: i32,
    cell_size_y: i32,
    cell_size_xz_inv: f32,
    cell_size_y_inv: f32,
) {
    let cell_out_x = cell_volume.block_x(cell[0]) - volume.min_block_x;
    let cell_out_y = cell_volume.block_y(cell[1]) - volume.min_block_y;
    let cell_out_z = cell_volume.block_z(cell[2]) - volume.min_block_z;
    let x0 = 0.max(-cell_out_x);
    let y0 = 0.max(-cell_out_y);
    let z0 = 0.max(-cell_out_z);
    let x1 = cell_size_xz.min(volume.size_x as i32 - cell_out_x) - 1;
    let y1 = cell_size_y.min(volume.size_y as i32 - cell_out_y) - 1;
    let z1 = cell_size_xz.min(volume.size_z as i32 - cell_out_z) - 1;

    let y_count = (y1 - y0 + 1) as usize;
    let y_start = (cell_out_y + y0) as usize;
    let xy_plane = volume.size_x * volume.size_y;

    for z in z0..=z1 {
        let alpha_z = z as f32 * cell_size_xz_inv;
        let out_z = (cell_out_z + z) as usize;
        let z_offset = out_z * xy_plane;

        let a00 = lerp(alpha_z, a000, a001);
        let a01 = lerp(alpha_z, a010, a011);
        let a10 = lerp(alpha_z, a100, a101);
        let a11 = lerp(alpha_z, a110, a111);

        let b00 = lerp(alpha_z, b000, b001);
        let b01 = lerp(alpha_z, b010, b011);
        let b10 = lerp(alpha_z, b100, b101);
        let b11 = lerp(alpha_z, b110, b111);

        for x in x0..=x1 {
            let alpha_x = x as f32 * cell_size_xz_inv;
            let out_x = (cell_out_x + x) as usize;

            let a_0 = lerp(alpha_x, a00, a10);
            let a_1 = lerp(alpha_x, a01, a11);
            let step_a = (a_1 - a_0) * cell_size_y_inv;
            let a_start = a_0 + step_a * y0 as f32;

            let b_0 = lerp(alpha_x, b00, b10);
            let b_1 = lerp(alpha_x, b01, b11);
            let step_b = (b_1 - b_0) * cell_size_y_inv;
            let b_start = b_0 + step_b * y0 as f32;

            let start = y_start + (out_x * volume.size_y) + z_offset;

            for dy in 0..y_count {
                let dy_f = dy as f32;
                let t = toggle_buffer[start + dy];
                if t >= -0.4 && t < 0.4 {
                    ridged_buffer[start + dy] = -1.0;
                } else {
                    let val_a = a_start + step_a * dy_f;
                    let val_b = b_start + step_b * dy_f;
                    ridged_buffer[start + dy] = 0.08 - val_a.abs().max(val_b.abs());
                }
            }
        }
    }
}