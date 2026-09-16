use crate::generation::structure::placement::GlobalStructureCache;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use pumpkin_data::block_properties::is_air;
use pumpkin_data::chunk::DoublePerlinNoiseParameters;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::fluid::{Fluid, FluidState};
use pumpkin_data::structures::{
    Structure, StructureKeys, StructurePlacementType, StructureSet, WeightedEntry,
};
use pumpkin_data::tag::RegistryKey;
use pumpkin_data::{Block, BlockState, block_properties::blocks_movement, chunk::Biome};
use pumpkin_data::{BlockId, BlockStateId, tag};
use pumpkin_util::random::xoroshiro128::XoroshiroSplitter;
use pumpkin_util::random::{RandomImpl, get_large_feature_seed, legacy_rand::LegacyRand};
use pumpkin_util::{
    HeightMap,
    math::{block_box::BlockBox, position::BlockPos, vector3::Vector3},
    random::{RandomGenerator, get_decorator_seed, worldgen_random::WorldgenRandom},
};
use rustc_hash::FxHashMap;

use super::{
    GlobalRandomConfig, biome_coords,
    blender::{Blender, BlenderImpl},
    feature::placed_features::PLACED_FEATURES,
    feature_order::for_each_selected_feature,
    noise::router::{
        multi_noise_sampler::MultiNoiseSampler, proto_noise_router::DoublePerlinNoiseBuilder,
        surface_height_sampler::SurfaceHeightEstimateSampler,
    },
    positions::chunk_pos::{start_block_x, start_block_z},
    surface::{MaterialRuleContext, estimate_surface_height, terrain::SurfaceTerrainBuilder},
};
use crate::biome::{BiomeSupplier, MultiNoiseBiomeSupplier, end::TheEndBiomeSupplier};
use crate::chunk::format::LightContainer;
use crate::chunk::{ChunkData, ChunkHeightmapType, ChunkLight};
use crate::chunk_system::{StagedChunkEnum, generation_cache::SurfaceBiomeNeighborhood};
use crate::generation::height_limit::HeightLimitView;
use crate::generation::noise::aquifer_sampler::{FluidLevel, FluidLevelSamplerImpl};
use crate::generation::noise::perlin::DoublePerlinNoiseSampler;
use crate::generation::noise::router::density_volume::DensityVolume;
use crate::generation::noise::router::surface_height_sampler::SurfaceHeightSamplerBuilderOptions;
use crate::generation::noise::{CHUNK_DIM, ChunkNoiseGenerator, LAVA_BLOCK, WATER_BLOCK};
use crate::generation::section_coords::section_to_block;
use crate::generation::structure::lazily_generate_structure;
use crate::generation::structure::placement::should_generate_structure;
use crate::generation::structure::structures::{
    StructureGeneratorContext, StructureInstance, create_chunk_random,
};
use crate::generation::surface::rule::try_apply_material_rule;
use crate::{
    chunk::CHUNK_AREA,
    generation::{biome, positions::chunk_pos},
    world::{BlockAccessor, WorldPortalExt},
};
use pumpkin_data::tag::get_tag_ids;
use pumpkin_nbt::compound::NbtCompound;

use crate::generation::structure::template::BlockPlacer;
use crate::tick::{ScheduledTick, TickPriority};

enum ActiveSupplier {
    Overworld(MultiNoiseBiomeSupplier),
    Nether(MultiNoiseBiomeSupplier),
    End(TheEndBiomeSupplier),
}

pub trait GenerationCache: HeightLimitView + BlockAccessor {
    fn get_center_chunk_mut(&mut self) -> &mut ProtoChunk;
    fn get_center_chunk(&self) -> &ProtoChunk;

    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut ProtoChunk>;
    fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&ProtoChunk>;

    fn try_get_proto_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&ProtoChunk>;

    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId;
    fn get_fluid_and_fluid_state(&self, position: &Vector3<i32>) -> (Fluid, FluidState);
    fn set_block_state(&mut self, pos: &Vector3<i32>, block_state: &BlockState);
    fn add_block_entity(&mut self, pos: &Vector3<i32>, nbt: NbtCompound);
    fn top_motion_blocking_block_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn top_motion_blocking_block_no_leaves_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn get_top_y(&self, heightmap: &HeightMap, x: i32, z: i32) -> i32;
    fn top_block_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn top_block_wg_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn ocean_floor_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn ocean_floor_wg_height_exclusive(&self, x: i32, z: i32) -> i32;
    fn get_world_seed(&self) -> u64;
    fn is_air(&self, local_pos: &Vector3<i32>) -> bool;
    fn get_biome_for_terrain_gen(&self, x: i32, y: i32, z: i32) -> &'static Biome;
    fn get_blending_data(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<&crate::generation::blender::blending_data::BlendingData>;
    fn get_sea_level(&self) -> i32 {
        63
    }
}

const AIR_BLOCK: Block = Block::AIR;

pub struct StandardChunkFluidLevelSampler {
    top_fluid: FluidLevel,
    bottom_fluid: FluidLevel,
    bottom_y: i32,
}

impl StandardChunkFluidLevelSampler {
    #[must_use]
    pub fn new(top_fluid: FluidLevel, bottom_fluid: FluidLevel) -> Self {
        let bottom_y = top_fluid
            .max_y_exclusive()
            .min(bottom_fluid.max_y_exclusive());
        Self {
            top_fluid,
            bottom_fluid,
            bottom_y,
        }
    }
}

impl FluidLevelSamplerImpl for StandardChunkFluidLevelSampler {
    fn get_fluid_level(&self, _x: i32, y: i32, _z: i32) -> &FluidLevel {
        if y < self.bottom_y {
            &self.bottom_fluid
        } else {
            &self.top_fluid
        }
    }
}

pub struct ProtoChunk {
    pub x: i32,
    pub z: i32,
    pub world_seed: u64,
    pub default_block: &'static BlockState,
    biome_mixer_seed: i64,
    pub(crate) flat_block_map: Box<[BlockStateId]>,
    pub flat_biome_map: Box<[u8]>,
    pub biome_mask: [u64; 4],
    pub flat_surface_height_map: [i16; CHUNK_AREA],
    pub flat_ocean_floor_height_map: [i16; CHUNK_AREA],
    pub flat_motion_blocking_height_map: [i16; CHUNK_AREA],
    pub flat_motion_blocking_no_leaves_height_map: [i16; CHUNK_AREA],
    pub structure_starts: FxHashMap<StructureKeys, StructureInstance>,
    pub emissive_sections: u32,

    height: u16,
    bottom_y: i8,
    generation_height: u16,
    generation_bottom_y: i8,
    /// Precomputed `height * CHUNK_DIM`: the stride between consecutive x-columns
    /// in `flat_block_map`. Cached once at construction so `local_pos_to_block_index`
    /// (called from every `get_block_state`/`set_block_state`) doesn't redo this
    /// multiplication on every single block access.
    column_stride: usize,
    /// Precomputed `height >> 2` (chunk height in biome-quart units), cached for the
    /// same reason as `column_stride` but for `local_biome_pos_to_biome_index`.
    biome_height: usize,
    pub stage: StagedChunkEnum,
    pub light: ChunkLight,
    pub carving_mask: crate::generation::carver::mask::CarvingMask,
    pub blending_data: Option<crate::generation::blender::blending_data::BlendingData>,
    pub pending_block_entities: Vec<NbtCompound>,
    pub pending_structure_entities: Vec<NbtCompound>,
    pub fluid_ticks: Vec<ScheduledTick<&'static Fluid>>,
}

pub struct TerrainCache {
    pub terrain_builder: SurfaceTerrainBuilder,
    pub surface_noise: DoublePerlinNoiseSampler,
    pub secondary_noise: DoublePerlinNoiseSampler,
    pub noise_samplers: [std::sync::OnceLock<DoublePerlinNoiseSampler>; DoublePerlinNoiseParameters::COUNT],
}

impl TerrainCache {
    #[must_use]
    pub fn from_random(random_config: &GlobalRandomConfig) -> Self {
        let random = &random_config.base_random_deriver;
        let terrain_builder = SurfaceTerrainBuilder::new(random);
        let surface_noise = DoublePerlinNoiseBuilder::get_noise_sampler_for_id(
            &random_config.base_random_deriver,
            &DoublePerlinNoiseParameters::SURFACE,
        );
        let secondary_noise = DoublePerlinNoiseBuilder::get_noise_sampler_for_id(
            &random_config.base_random_deriver,
            &DoublePerlinNoiseParameters::SURFACE_SECONDARY,
        );
        const INIT_LOCK: std::sync::OnceLock<DoublePerlinNoiseSampler> = std::sync::OnceLock::new();
        Self {
            terrain_builder,
            surface_noise,
            secondary_noise,
            noise_samplers: [INIT_LOCK; DoublePerlinNoiseParameters::COUNT],
        }
    }

    #[inline]
    #[must_use]
    pub fn get_noise_sampler(
        &self,
        params: &DoublePerlinNoiseParameters,
        random_deriver: &XoroshiroSplitter,
    ) -> &DoublePerlinNoiseSampler {
        if params.id < DoublePerlinNoiseParameters::COUNT {
            self.noise_samplers[params.id].get_or_init(|| {
                DoublePerlinNoiseBuilder::get_noise_sampler_for_id(random_deriver, params)
            })
        } else {
            &self.surface_noise
        }
    }
}

pub static STRUCT_REF_PROF_ENABLED: AtomicBool = AtomicBool::new(false);
pub static PROF_CANDIDATE_MATH_NS: AtomicU64 = AtomicU64::new(0);
pub static PROF_SHOULD_GENERATE_NS: AtomicU64 = AtomicU64::new(0);
pub static PROF_COMPUTE_START_NS: AtomicU64 = AtomicU64::new(0);
pub static PROF_BBOX_NS: AtomicU64 = AtomicU64::new(0);
pub static PROF_TOTAL_NS: AtomicU64 = AtomicU64::new(0);
pub static PROF_SET_TIMES_NS: [AtomicU64; 20] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];
pub static PROF_SET_CALLS: [AtomicU64; 20] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];
pub static PROF_SET_HITS: [AtomicU64; 20] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];
pub static PROF_JIGSAW_COUNT: AtomicU64 = AtomicU64::new(0);
pub static PROF_JIGSAW_NS: AtomicU64 = AtomicU64::new(0);

pub fn enable_structure_ref_profiling(enable: bool) {
    STRUCT_REF_PROF_ENABLED.store(enable, Ordering::Relaxed);
}

pub fn dump_and_reset_structure_ref_profiling(name: &str) {
    let total_ns = PROF_TOTAL_NS.swap(0, Ordering::Relaxed);
    let cand_ns = PROF_CANDIDATE_MATH_NS.swap(0, Ordering::Relaxed);
    let should_ns = PROF_SHOULD_GENERATE_NS.swap(0, Ordering::Relaxed);
    let comp_ns = PROF_COMPUTE_START_NS.swap(0, Ordering::Relaxed);
    let bbox_ns = PROF_BBOX_NS.swap(0, Ordering::Relaxed);
    let jigsaw_cnt = PROF_JIGSAW_COUNT.swap(0, Ordering::Relaxed);
    let jigsaw_ns = PROF_JIGSAW_NS.swap(0, Ordering::Relaxed);

    println!("--- [{}] StructureReferences Breakdown ---", name);
    println!("  Total time inside set_structure_references: {:.3} ms", total_ns as f64 / 1_000_000.0);
    println!("    - Candidate Chunk Math:     {:.3} ms", cand_ns as f64 / 1_000_000.0);
    println!("    - should_generate_structure: {:.3} ms", should_ns as f64 / 1_000_000.0);
    println!("    - compute_structure_start:   {:.3} ms (Jigsaw invocations: {}, Jigsaw time: {:.3} ms)",
        comp_ns as f64 / 1_000_000.0, jigsaw_cnt, jigsaw_ns as f64 / 1_000_000.0);
    println!("    - Bounding Box & Collector:  {:.3} ms", bbox_ns as f64 / 1_000_000.0);
    println!("  Per Structure Set Profile:");
    for (i, &name) in StructureSet::NAMES.iter().enumerate() {
        let set_time = PROF_SET_TIMES_NS[i].swap(0, Ordering::Relaxed);
        let calls = PROF_SET_CALLS[i].swap(0, Ordering::Relaxed);
        let hits = PROF_SET_HITS[i].swap(0, Ordering::Relaxed);
        if set_time > 0 || calls > 0 {
            println!("    - {:<20}: {:8.3} ms | candidates: {:4} | hits: {:2}",
                name, set_time as f64 / 1_000_000.0, calls, hits);
        }
    }
    println!("--------------------------------------------------");
}

impl ProtoChunk {
    #[must_use]
    pub const fn get_sea_level(&self) -> i32 {
        63
    }

    #[cfg(test)]
    pub(crate) fn has_structure(&self, key: StructureKeys) -> bool {
        self.structure_starts.contains_key(&key)
    }

    #[must_use]
    pub fn new(x: i32, z: i32, generator: &super::generator::WorldGenerator) -> Self {
        let dimension = generator.dimension();
        let height = dimension.height as u16;
        let bottom_y = dimension.min_y as i8;
        let section_count = (height as usize) / 16;

        let (generation_height, generation_bottom_y) = match generator {
            super::generator::WorldGenerator::Noise(noise_gen) => {
                let shape = noise_gen
                    .settings
                    .shape
                    .trim_height(bottom_y, (dimension.min_y + dimension.height) as u16);
                (shape.height, shape.min_y)
            }
            super::generator::WorldGenerator::Flat(_)
            | super::generator::WorldGenerator::Custom(_) => (height, bottom_y),
        };

        let default_block = match generator {
            super::generator::WorldGenerator::Noise(noise_gen) => noise_gen.default_block,
            super::generator::WorldGenerator::Flat(_) => Block::AIR.default_state,
            super::generator::WorldGenerator::Custom(custom_gen) => custom_gen.default_block(),
        };
        let biome_mixer_seed = match generator {
            super::generator::WorldGenerator::Noise(noise_gen) => noise_gen.biome_mixer_seed,
            super::generator::WorldGenerator::Flat(flat_gen) => {
                crate::biome::hash_seed(flat_gen.seed)
            }
            super::generator::WorldGenerator::Custom(custom_gen) => custom_gen.biome_mixer_seed(),
        };

        let default_heightmap = [i16::MIN; CHUNK_AREA];
        let world_seed = generator.seed();
            let default_biome_id = Biome::PLAINS.id;
            let mut initial_mask = [0u64; 4];
            initial_mask[(default_biome_id >> 6) as usize] |= 1u64 << (default_biome_id & 63);
            let column_stride = height as usize * CHUNK_DIM as usize;
            let biome_height = height as usize >> 2;
            Self {
                x,
                z,
                world_seed,
                default_block,
                biome_mixer_seed,
                flat_block_map: vec![BlockStateId::AIR; CHUNK_AREA * height as usize]
                    .into_boxed_slice(),
                flat_biome_map: vec![
                    default_biome_id;
                    biome_coords::from_block(CHUNK_DIM as i32) as usize
                        * biome_coords::from_block(CHUNK_DIM as i32) as usize
                        * biome_coords::from_block(height as i32) as usize
                ]
                .into_boxed_slice(),
                biome_mask: initial_mask,
                flat_surface_height_map: default_heightmap,
                flat_ocean_floor_height_map: default_heightmap,
                flat_motion_blocking_height_map: default_heightmap,
                flat_motion_blocking_no_leaves_height_map: default_heightmap,
                structure_starts: FxHashMap::default(),
                emissive_sections: 0,
                height,
                bottom_y,
                generation_height,
                generation_bottom_y,
                column_stride,
                biome_height,
                stage: StagedChunkEnum::Empty,
                light: ChunkLight {
                    sky_light: (0..section_count)
                        .map(|_| LightContainer::new_empty(0))
                        .collect(),
                    block_light: (0..section_count)
                        .map(|_| LightContainer::new_empty(0))
                        .collect(),
                },
                carving_mask: crate::generation::carver::mask::CarvingMask::new(
                    height as i32,
                    bottom_y as i32,
                ),
                blending_data: None,
                pending_block_entities: Vec::new(),
                pending_structure_entities: Vec::new(),
                fluid_ticks: Vec::new(),
            }
        }

    #[inline]
    pub fn update_biome_mask(&mut self) {
        let mut mask = [0u64; 4];
        for &id in &self.flat_biome_map[..] {
            mask[(id >> 6) as usize] |= 1u64 << (id & 63);
        }
        self.biome_mask = mask;
    }

    /// Create a fresh `ProtoChunk` at `(x, z)` inheriting all dimension/seed
    /// metadata from an existing chunk (without re-deriving from `WorldGenerator`).
    ///
    /// Used by `StageCache` to clone biome-stage metadata before running
    /// structure-start generation on the cloned chunk.
    #[must_use]
    pub(crate) fn new_raw(x: i32, z: i32, template: &Self) -> Self {
        use crate::chunk::format::LightContainer;
        let height = template.height;
        let bottom_y = template.bottom_y;
        let section_count = (height as usize) / 16;
        let default_heightmap = [i16::MIN; CHUNK_AREA];
        Self {
            x,
            z,
            world_seed: template.world_seed,
            default_block: template.default_block,
            biome_mixer_seed: template.biome_mixer_seed,
            flat_block_map: vec![
                pumpkin_data::BlockStateId::AIR;
                CHUNK_AREA * height as usize
            ]
            .into_boxed_slice(),
            flat_biome_map: template.flat_biome_map.clone(),
            biome_mask: template.biome_mask,
            flat_surface_height_map: default_heightmap,
            flat_ocean_floor_height_map: default_heightmap,
            flat_motion_blocking_height_map: default_heightmap,
            flat_motion_blocking_no_leaves_height_map: default_heightmap,
            structure_starts: rustc_hash::FxHashMap::default(),
            emissive_sections: template.emissive_sections,
            height,
            bottom_y,
            generation_height: template.generation_height,
            generation_bottom_y: template.generation_bottom_y,
            column_stride: template.column_stride,
            biome_height: template.biome_height,
            stage: StagedChunkEnum::Biomes,
            light: ChunkLight {
                sky_light: (0..section_count)
                    .map(|_| LightContainer::new_empty(0))
                    .collect(),
                block_light: (0..section_count)
                    .map(|_| LightContainer::new_empty(0))
                    .collect(),
            },
            carving_mask: crate::generation::carver::mask::CarvingMask::new(
                height as i32,
                bottom_y as i32,
            ),
            blending_data: None,
            pending_block_entities: Vec::new(),
            pending_structure_entities: Vec::new(),
            fluid_ticks: Vec::new(),
        }
    }

    /// Copy the completed `structure_starts` map from `src` into `self`.
    ///
    /// Used by `StageCache` to transfer the read-only structure data computed on
    /// a cached chunk into a fresh mutable chunk that will continue through
    /// StructureReferences and later stages.
    pub(crate) fn copy_structure_starts_from(&mut self, src: &Self) {
        self.structure_starts.clone_from(&src.structure_starts);
    }

    #[inline]
    #[must_use]
    pub fn structure_starts(&self) -> &FxHashMap<StructureKeys, StructureInstance> {
        &self.structure_starts
    }

    #[must_use]
    pub fn from_chunk_data(
        chunk_data: &ChunkData,
        generator: &super::generator::WorldGenerator,
    ) -> Self {
        let mut proto_chunk = Self::new(chunk_data.x, chunk_data.z, generator);

        proto_chunk.light = chunk_data
            .light_engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        proto_chunk
            .blending_data
            .clone_from(&chunk_data.blending_data);
        proto_chunk.emissive_sections = 0;

        let section_data = &chunk_data.section;
        let heightmap_data = chunk_data
            .heightmap
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let block_sections_guard = section_data
            .block_sections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let biome_sections_guard = section_data
            .biome_sections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        for (section_idx, block_palette) in block_sections_guard.iter().enumerate() {
            let section_base_y = section_idx as i32 * 16;

            if section_base_y >= proto_chunk.height() as i32 {
                continue;
            }

            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..16 {
                        let block_state_id = block_palette.get(x, y, z);
                        let block_state = BlockState::from_id(block_state_id);
                        let absolute_y = section_base_y + y as i32 + section_data.min_y;

                        proto_chunk.set_block_state(x as i32, absolute_y, z as i32, block_state);
                    }
                }
            }

            if let Some(biome_palette) = biome_sections_guard.get(section_idx) {
                for x in 0..4 {
                    for y in 0..4 {
                        for z in 0..4 {
                            let biome_id = biome_palette.get(x, y, z);
                            let biome_y_idx = (section_idx * 4) + y;
                            let index = proto_chunk.local_biome_pos_to_biome_index(
                                x as i32,
                                biome_y_idx as i32,
                                z as i32,
                            );
                            proto_chunk.flat_biome_map[index] = biome_id;
                        }
                    }
                }
            }
        }
        drop(block_sections_guard);
        drop(biome_sections_guard);
        proto_chunk.update_biome_mask();

        for z in 0..16 {
            for x in 0..16 {
                let index = Self::local_position_to_height_map_index(x, z);

                proto_chunk.flat_motion_blocking_height_map[index] = heightmap_data.get(
                    ChunkHeightmapType::MotionBlocking,
                    x,
                    z,
                    section_data.min_y,
                ) as i16;

                proto_chunk.flat_motion_blocking_no_leaves_height_map[index] = heightmap_data.get(
                    ChunkHeightmapType::MotionBlockingNoLeaves,
                    x,
                    z,
                    section_data.min_y,
                )
                    as i16;

                proto_chunk.flat_surface_height_map[index] =
                    heightmap_data.get(ChunkHeightmapType::WorldSurface, x, z, section_data.min_y)
                        as i16;
            }
        }

        let saved_stage = StagedChunkEnum::from(chunk_data.status);
        proto_chunk.stage = saved_stage;
        if let super::generator::WorldGenerator::Noise(generator) = generator
            && (StagedChunkEnum::StructureStart..StagedChunkEnum::Features).contains(&saved_stage)
        {
            // Structure starts and references are currently transient proto-chunk data.
            // Rebuild them when resuming a partially generated chunk so structures that
            // cross chunk boundaries are not truncated at the unload boundary.
            proto_chunk.stage = StagedChunkEnum::Biomes;
            proto_chunk.set_structure_starts(generator);
            if saved_stage >= StagedChunkEnum::StructureReferences {
                proto_chunk.set_structure_references(generator);
            }
            proto_chunk.stage = saved_stage;
        }
        proto_chunk
    }

    #[inline]
    #[must_use]
    pub const fn stage_id(&self) -> u8 {
        self.stage as u8
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub const fn bottom_y(&self) -> i8 {
        self.bottom_y
    }

    #[must_use]
    pub const fn generation_height(&self) -> u16 {
        self.generation_height
    }

    #[must_use]
    pub const fn generation_bottom_y(&self) -> i8 {
        self.generation_bottom_y
    }

    pub fn add_block_entity(&mut self, nbt: NbtCompound) {
        self.pending_block_entities.push(nbt);
    }

    pub fn take_pending_block_entities(&mut self) -> Vec<NbtCompound> {
        std::mem::take(&mut self.pending_block_entities)
    }

    pub fn add_structure_entity(&mut self, nbt: NbtCompound) {
        self.pending_structure_entities.push(nbt);
    }

    pub(crate) fn take_pending_structure_entities(&mut self) -> Vec<NbtCompound> {
        std::mem::take(&mut self.pending_structure_entities)
    }

    pub fn schedule_fluid_tick(&mut self, x: i32, y: i32, z: i32, fluid: &'static Fluid) {
        self.fluid_ticks.push(ScheduledTick {
            delay: 0,
            priority: TickPriority::Normal,
            position: BlockPos::new(x, y, z),
            value: fluid,
        });
    }

    fn maybe_update_surface_height_map(&mut self, index: usize, y: i16) {
        let current_height = self.flat_surface_height_map[index];
        self.flat_surface_height_map[index] = current_height.max(y);
    }

    fn maybe_update_ocean_floor_height_map(&mut self, index: usize, y: i16) {
        let current_height = self.flat_ocean_floor_height_map[index];
        self.flat_ocean_floor_height_map[index] = current_height.max(y);
    }

    fn maybe_update_motion_blocking_height_map(&mut self, index: usize, y: i16) {
        let current_height = self.flat_motion_blocking_height_map[index];
        self.flat_motion_blocking_height_map[index] = current_height.max(y);
    }

    fn maybe_update_motion_blocking_no_leaves_height_map(&mut self, index: usize, y: i16) {
        let current_height = self.flat_motion_blocking_no_leaves_height_map[index];
        self.flat_motion_blocking_no_leaves_height_map[index] = current_height.max(y);
    }

    #[must_use]
    pub const fn get_top_y(&self, heightmap: &HeightMap, x: i32, z: i32) -> i32 {
        match heightmap {
            HeightMap::WorldSurfaceWg | HeightMap::WorldSurface => {
                self.top_block_height_exclusive(x, z)
            }
            HeightMap::OceanFloorWg | HeightMap::OceanFloor => {
                self.ocean_floor_height_exclusive(x, z)
            }
            HeightMap::MotionBlocking => self.top_motion_blocking_block_height_exclusive(x, z),
            HeightMap::MotionBlockingNoLeaves => {
                self.top_motion_blocking_block_no_leaves_height_exclusive(x, z)
            }
        }
    }

    #[must_use]
    pub const fn top_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let index = Self::local_position_to_height_map_index(x & 15, z & 15);
        self.flat_surface_height_map[index] as i32 + 1
    }

    #[must_use]
    pub const fn top_block_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        self.top_block_height_exclusive(x, z)
    }

    #[must_use]
    pub const fn ocean_floor_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let index = Self::local_position_to_height_map_index(x & 15, z & 15);
        self.flat_ocean_floor_height_map[index] as i32 + 1
    }

    #[must_use]
    pub const fn ocean_floor_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        self.ocean_floor_height_exclusive(x, z)
    }

    #[must_use]
    pub const fn top_motion_blocking_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let index = Self::local_position_to_height_map_index(x & 15, z & 15);
        self.flat_motion_blocking_height_map[index] as i32 + 1
    }

    #[must_use]
    pub const fn top_motion_blocking_block_no_leaves_height_exclusive(
        &self,
        x: i32,
        z: i32,
    ) -> i32 {
        let index = Self::local_position_to_height_map_index(x & 15, z & 15);
        self.flat_motion_blocking_no_leaves_height_map[index] as i32 + 1
    }

    #[inline(always)]
    const fn local_position_to_height_map_index(x: i32, z: i32) -> usize {
        x as usize * CHUNK_DIM as usize + z as usize
    }

    #[inline(always)]
    const fn local_pos_to_block_index(&self, x: i32, y: i32, z: i32) -> usize {
        // Uses the precomputed `column_stride` (== height * CHUNK_DIM) instead of
        // recomputing that multiplication on every single block access - this is
        // the single hottest indexing function in chunk gen (every get/set of a
        // block state goes through it).
        self.column_stride * x as usize + CHUNK_DIM as usize * y as usize + z as usize
    }

    #[inline(always)]
    #[must_use]
    pub const fn local_biome_pos_to_biome_index(&self, x: i32, y: i32, z: i32) -> usize {
        self.biome_height * biome_coords::from_block(CHUNK_DIM as i32) as usize * x as usize
            + biome_coords::from_block(CHUNK_DIM as i32) as usize * y as usize
            + z as usize
    }

    #[inline(always)]
    #[must_use]
    pub fn is_air(&self, local_pos: &Vector3<i32>) -> bool {
        is_air(self.get_block_state(local_pos))
    }

    #[inline(always)]
    #[must_use]
    pub fn get_block_state_raw(&self, x: i32, y: i32, z: i32) -> BlockStateId {
        let index = self.local_pos_to_block_index(x, y, z);
        self.flat_block_map[index]
    }

    #[inline(always)]
    #[must_use]
    pub fn get_block_state(&self, local_pos: &Vector3<i32>) -> BlockStateId {
        let local_y = local_pos.y - self.bottom_y() as i32;
        if local_y < 0 || local_y >= self.height() as i32 {
            return Block::VOID_AIR.default_state.id;
        }
        self.get_block_state_raw(local_pos.x & 15, local_y, local_pos.z & 15)
    }

    #[inline]
    pub fn set_block_state(&mut self, x: i32, y: i32, z: i32, block_state: &BlockState) {
        let local_x = x & 15;
        let local_y = y - self.bottom_y() as i32;
        let local_z = z & 15;

        if local_y < 0 || local_y >= self.height() as i32 {
            return;
        }
        if !block_state.is_air() {
            if block_state.luminance > 0 {
                let sec = (local_y >> 4) as u32;
                self.emissive_sections |= 1 << sec;
            }
            let index = Self::local_position_to_height_map_index(local_x, local_z);
            let y = y as i16;
            self.maybe_update_surface_height_map(index, y);
            let block = BlockId::from_state_id(block_state.id);

            let blocks_movement = blocks_movement(block_state, block);
            if blocks_movement {
                self.maybe_update_ocean_floor_height_map(index, y);
            }
            if blocks_movement || block_state.is_liquid() {
                self.maybe_update_motion_blocking_height_map(index, y);
                if !block.has_tag(tag::Block::MINECRAFT_LEAVES) {
                    {
                        self.maybe_update_motion_blocking_no_leaves_height_map(index, y);
                    }
                }
            }
        }

        let index = self.local_pos_to_block_index(local_x, local_y, local_z);
        self.flat_block_map[index] = block_state.id;
    }

    #[inline]
    #[must_use]
    pub fn get_biome(&self, x: i32, y: i32, z: i32) -> &'static Biome {
        Biome::from_id(self.get_biome_id(x, y, z)).unwrap_or(&Biome::PLAINS)
    }

    #[inline(always)]
    #[must_use]
    pub fn get_biome_id(&self, x: i32, y: i32, z: i32) -> u8 {
        let local_y = (y - biome_coords::from_block(self.bottom_y() as i32))
            .clamp(0, self.biome_height as i32 - 1);
        let index = self.local_biome_pos_to_biome_index(
            x & 3,
            local_y,
            z & 3,
        );
        self.flat_biome_map[index]
    }

    pub fn step_to_biomes(&mut self, generator: &super::generator::VanillaGenerator) {
        debug_assert_eq!(self.stage, StagedChunkEnum::Empty);
        let mut multi_noise_sampler =
            MultiNoiseSampler::generate(&generator.base_router.multi_noise);
        self.populate_biomes(generator, &mut multi_noise_sampler);
        self.stage = StagedChunkEnum::Biomes;
    }

    #[expect(clippy::too_many_lines)]
    pub fn step_to_noise(&mut self, generator: &super::generator::VanillaGenerator) {
        debug_assert_eq!(self.stage, StagedChunkEnum::StructureReferences);
        let settings = generator.settings;
        let generation_shape = &settings.shape;
        let start_x = start_block_x(self.x);
        let start_z = start_block_z(self.z);

        let sampler = StandardChunkFluidLevelSampler::new(
            FluidLevel::new(
                settings.sea_level,
                Block::from_state_id(settings.default_fluid.id),
            ),
            FluidLevel::new(-54, &Block::LAVA),
        );

        let (beardifier_structures, beardifier_junctions, affected_box) =
            if self.structure_starts.is_empty() {
                (Vec::new(), Vec::new(), None)
            } else {
                let mut beardifier_structures = Vec::new();
                let mut beardifier_junctions = Vec::new();
                let mut any_piece_bounding_box: Option<BlockBox> = None;

                let chunk_start_x = self.start_block_x();
                let chunk_start_z = self.start_block_z();

                for (key, instance) in &self.structure_starts {
                    let structure = pumpkin_data::structures::Structure::get(key);
                    let terrain_adaptation = structure.terrain_adaptation;

                    // Vanilla strictly skips filtering Beardifier parts if adaptation is None early-on
                    if terrain_adaptation == pumpkin_data::structures::TerrainAdaptation::None {
                        continue;
                    }

                    let collector = match instance {
                        StructureInstance::Start(pos) => &pos.collector,
                        StructureInstance::Reference(collector) => collector,
                    };

                    let collector = collector
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    for piece in &collector.pieces {
                        let bounding_box = piece.get_structure_piece().bounding_box;

                        // Match `piece.isCloseToChunk(chunkPos, 12)`
                        // Validates if an expansion 12 blocks out covers the chunk borders
                        if !bounding_box.intersects_raw_xz(
                            chunk_start_x - 12,
                            chunk_start_z - 12,
                            chunk_start_x + 15 + 12,
                            chunk_start_z + 15 + 12,
                        ) {
                            continue;
                        }

                        let mut ground_level_delta = 0;

                        if let Some(jigsaw_piece) = piece.as_any().downcast_ref::<crate::generation::structure::structures::jigsaw::PoolElementStructurePiece>() {
                            // Java only adds to rigids if projection is RIGID
                            if jigsaw_piece.projection == crate::generation::structure::structures::jigsaw::JigsawProjection::Rigid {
                                ground_level_delta = jigsaw_piece.ground_level_delta;
                                any_piece_bounding_box = any_piece_bounding_box.map_or(Some(bounding_box), |mut b| {
                                    b.encompass(&bounding_box);
                                    Some(b)
                                });

                                beardifier_structures.push(
                                    crate::generation::noise::router::density_function::beardifier::BeardifierStructure {
                                        bounding_box,
                                        terrain_adaptation,
                                        ground_level_delta,
                                    },
                                );
                            }

                            for j in &jigsaw_piece.junctions {
                                let j_x = j.source_x;
                                let j_z = j.source_z;
                                // Junction bounds filter (match vanilla proximity checks)
                                if j_x > chunk_start_x - 12
                                    && j_z > chunk_start_z - 12
                                    && j_x < chunk_start_x + 15 + 12
                                    && j_z < chunk_start_z + 15 + 12
                                {
                                    beardifier_junctions.push(
                                        crate::generation::noise::router::density_function::beardifier::BeardifierJunction {
                                            x: j_x,
                                            ground_y: j.source_ground_y,
                                            z: j_z,
                                        },
                                    );
                                    let junction_box = BlockBox::from_pos(BlockPos::new(j_x, j.source_ground_y, j_z));
                                    any_piece_bounding_box = any_piece_bounding_box.map_or(Some(junction_box), |mut b| {
                                        b.encompass(&junction_box);
                                        Some(b)
                                    });
                                }
                            }
                        } else {
                            any_piece_bounding_box = any_piece_bounding_box.map_or(Some(bounding_box), |mut b| {
                                b.encompass(&bounding_box);
                                Some(b)
                            });

                            beardifier_structures.push(
                                crate::generation::noise::router::density_function::beardifier::BeardifierStructure {
                                    bounding_box,
                                    terrain_adaptation,
                                    ground_level_delta,
                                },
                            );
                        }
                    }
                }
                (
                    beardifier_structures,
                    beardifier_junctions,
                    any_piece_bounding_box.map(|b| b.expand(24, 24, 24)),
                )
            };

        // Passed the newly mapped beardifier structures & junctions arrays independently!
        let mut noise_sampler = ChunkNoiseGenerator::new(
            &generator.base_router.noise,
            &generator.random_config,
            DensityVolume::with_block_step(
                CHUNK_DIM as usize,
                generation_shape.height as usize,
                CHUNK_DIM as usize,
                start_x,
                i32::from(generation_shape.min_y),
                start_z,
            ),
            generation_shape,
            sampler,
            settings.aquifers_enabled,
            settings.ore_veins_enabled,
            beardifier_structures,
            beardifier_junctions,
            affected_box,
        );

        let surface_config = SurfaceHeightSamplerBuilderOptions::new(
            generation_shape.min_y as i32,
            generation_shape.max_y() as i32,
            generation_shape.vertical_cell_block_count() as usize,
        );
        let mut surface_height_estimate_sampler = SurfaceHeightEstimateSampler::generate(
            &generator.base_router.surface_estimator,
            &surface_config,
        );
        self.populate_noise(
            generator,
            &mut noise_sampler,
            &generator.random_config.ore_random_deriver,
            &mut surface_height_estimate_sampler,
        );

        self.stage = StagedChunkEnum::Noise;
    }

    pub(crate) fn step_to_surface(
        &mut self,
        generator: &super::generator::VanillaGenerator,
        surface_biomes: &SurfaceBiomeNeighborhood,
    ) {
        debug_assert_eq!(self.stage, StagedChunkEnum::Noise);
        let generation_shape = &generator.settings.shape;
        let surface_config = SurfaceHeightSamplerBuilderOptions::new(
            generation_shape.min_y as i32,
            generation_shape.max_y() as i32,
            generation_shape.vertical_cell_block_count() as usize,
        );
        let mut surface_height_estimate_sampler = SurfaceHeightEstimateSampler::generate(
            &generator.base_router.surface_estimator,
            &surface_config,
        );

        self.build_surface(
            generator,
            surface_biomes,
            &mut surface_height_estimate_sampler,
        );
        self.stage = StagedChunkEnum::Surface;
    }

    pub fn step_to_carvers(&mut self, generator: &super::generator::VanillaGenerator) {
        debug_assert_eq!(self.stage, StagedChunkEnum::Surface);
        super::carver::carve(self, generator);

        self.stage = StagedChunkEnum::Carvers;
    }

    pub fn populate_biomes(
        &mut self,
        generator: &super::generator::VanillaGenerator,
        multi_noise_sampler: &mut MultiNoiseSampler,
    ) {
        let dimension = &generator.dimension;
        let active_supplier = if dimension == &Dimension::THE_END {
            ActiveSupplier::End(TheEndBiomeSupplier)
        } else if dimension == &Dimension::THE_NETHER {
            ActiveSupplier::Nether(MultiNoiseBiomeSupplier::NETHER)
        } else {
            ActiveSupplier::Overworld(MultiNoiseBiomeSupplier::OVERWORLD)
        };
        let base_supplier: &dyn BiomeSupplier = match &active_supplier {
            ActiveSupplier::End(s) => s,
            ActiveSupplier::Nether(s) | ActiveSupplier::Overworld(s) => s,
        };
        let blender = Blender::empty();
        let blender_supplier;
        let biome_supplier: &dyn BiomeSupplier = if self.blending_data.is_some() {
            blender_supplier = blender.get_biome_supplier(base_supplier);
            &blender_supplier
        } else {
            base_supplier
        };
        let min_y = self.bottom_y();

        let start_block_x = start_block_x(self.x);
        let start_block_z = start_block_z(self.z);

        let total_biome_y = biome_coords::from_block(self.height() as i32) as usize;
        let biomes_per_section = biome_coords::from_block(CHUNK_DIM as i32) as usize;

        if *dimension == Dimension::THE_END {
            let start_biome_x = biome_coords::from_block(start_block_x);
            let start_biome_z = biome_coords::from_block(start_block_z);
            let min_biome_y = biome_coords::from_block(min_y as i32);
            for x in 0..biomes_per_section as i32 {
                for z in 0..biomes_per_section as i32 {
                    for by in 0..total_biome_y as i32 {
                        let biome = biome_supplier.biome(
                            start_biome_x + x,
                            min_biome_y + by,
                            start_biome_z + z,
                            multi_noise_sampler,
                        );
                        let index = self.local_biome_pos_to_biome_index(x, by, z);
                        self.flat_biome_map[index] = biome.id;
                    }
                }
            }
            self.update_biome_mask();
            return;
        }

        let biome_step = biome_coords::to_block(1);

        if *dimension == Dimension::OVERWORLD && self.blending_data.is_none() {
            let col_volume = DensityVolume::new(
                biomes_per_section,
                1,
                biomes_per_section,
                start_block_x,
                0,
                start_block_z,
                biome_step,
                biome_step,
                biome_step,
            );
            let depth_volume = DensityVolume::new(
                1,
                total_biome_y,
                1,
                start_block_x,
                min_y as i32,
                start_block_z,
                biome_step,
                biome_step,
                biome_step,
            );
            let (col_buffers, depth_buf) =
                multi_noise_sampler.fill_overworld_column_cached(&col_volume, &depth_volume);

            let b_temp = &col_buffers[0];
            let b_humid = &col_buffers[1];
            let b_cont = &col_buffers[2];
            let b_erosion = &col_buffers[3];
            let b_ridges = &col_buffers[4];

            for z in 0..biomes_per_section {
                for x in 0..biomes_per_section {
                    let col_idx_2d = x + z * biomes_per_section;
                    let t_long = crate::biome::multi_noise::to_long(b_temp[col_idx_2d]);
                    let h_long = crate::biome::multi_noise::to_long(b_humid[col_idx_2d]);
                    let c_long = crate::biome::multi_noise::to_long(b_cont[col_idx_2d]);
                    let e_long = crate::biome::multi_noise::to_long(b_erosion[col_idx_2d]);
                    let r_long = crate::biome::multi_noise::to_long(b_ridges[col_idx_2d]);

                    let mut last_depth = i64::MAX;
                    let mut last_biome = &pumpkin_data::chunk::Biome::PLAINS;

                    for by in 0..total_biome_y {
                        let d_long = crate::biome::multi_noise::to_long(depth_buf[by]);
                        let biome = if d_long == last_depth {
                            last_biome
                        } else {
                            let point_list = [t_long, h_long, c_long, e_long, d_long, r_long, 0];
                            let b = biome_supplier.biome_from_point(point_list, multi_noise_sampler);
                            last_depth = d_long;
                            last_biome = b;
                            b
                        };
                        let index = self.local_biome_pos_to_biome_index(x as i32, by as i32, z as i32);
                        self.flat_biome_map[index] = biome.id;
                    }
                }
            }
            self.update_biome_mask();
            return;
        }

        let buffers = multi_noise_sampler.fill_volume(DensityVolume::new(
            biomes_per_section,
            total_biome_y,
            biomes_per_section,
            start_block_x,
            min_y as i32,
            start_block_z,
            biome_step,
            biome_step,
            biome_step,
        ));

        let b0 = &buffers[0];
        let b1 = &buffers[1];
        let b2 = &buffers[2];
        let b3 = &buffers[3];
        let b4 = &buffers[4];
        let b5 = &buffers[5];

        for z in 0..biomes_per_section {
            for x in 0..biomes_per_section {
                let mut last_point_list = [i64::MAX; 7];
                let mut last_biome = &pumpkin_data::chunk::Biome::PLAINS;

                let col_idx = (x + z * biomes_per_section) * total_biome_y;

                for by in 0..total_biome_y {
                    let idx = col_idx + by;
                    let point_list = [
                        crate::biome::multi_noise::to_long(b0[idx]),
                        crate::biome::multi_noise::to_long(b1[idx]),
                        crate::biome::multi_noise::to_long(b2[idx]),
                        crate::biome::multi_noise::to_long(b3[idx]),
                        crate::biome::multi_noise::to_long(b4[idx]),
                        crate::biome::multi_noise::to_long(b5[idx]),
                        0,
                    ];
                    let biome = if point_list == last_point_list {
                        last_biome
                    } else {
                        let b = biome_supplier.biome_from_point(point_list, multi_noise_sampler);
                        last_point_list = point_list;
                        last_biome = b;
                        b
                    };
                    let index = self.local_biome_pos_to_biome_index(x as i32, by as i32, z as i32);
                    self.flat_biome_map[index] = biome.id;
                }
            }
        }
        self.update_biome_mask();
    }

    pub fn populate_noise(
        &mut self,
        generator: &super::generator::VanillaGenerator,
        noise_sampler: &mut ChunkNoiseGenerator,
        ore_random_deriver: &XoroshiroSplitter,
        surface_height_estimate_sampler: &mut SurfaceHeightEstimateSampler,
    ) {
        let volume = *noise_sampler.volume();
        let densities = noise_sampler.sample_density();
        let chunk_height = self.height() as usize;
        let bottom_y = self.bottom_y() as i32;

        let default_state_id = generator.default_block.id;
        let default_luminance = generator.default_block.luminance;
        let air_state_id = Block::AIR.default_state.id;

        let sky_skip_y = noise_sampler
            .get_skip_sampling_above_y(surface_height_estimate_sampler)
            .map(|skip_y| (skip_y + 1).max(generator.settings.sea_level));

        for z in 0..volume.size_z {
            let block_z = volume.block_z(z);
            for x in 0..volume.size_x {
                let block_x = volume.block_x(x);
                let hm_index = Self::local_position_to_height_map_index(x as i32, z as i32);
                let col_base = self.column_stride * x + z;
                let col_density_offset = (x + z * volume.size_x) * volume.size_y;
                let mut surface_found = false;
                let mut ocean_floor_found = false;
                let mut motion_blocking_found = false;
                let mut motion_no_leaves_found = false;
                let mut all_hm_found = false;

                let mut block_idx = col_base + CHUNK_DIM as usize * (volume.size_y.saturating_sub(1));
                for y in (0..volume.size_y).rev() {
                    let block_y = volume.block_y(y);
                    let index = col_density_offset + y;
                    let density = densities.density[index];
                    let vein = if block_y >= crate::generation::noise::ore_sampler::vein_type::MIN_Y
                        && block_y <= crate::generation::noise::ore_sampler::vein_type::MAX_Y
                    {
                        densities.vein_sample(index)
                    } else {
                        None
                    };

                    if all_hm_found && density > 0.0 && vein.is_none() {
                        let local_y = block_y - bottom_y;
                        if local_y >= 0 && (local_y as usize) < chunk_height {
                            self.flat_block_map[block_idx] = default_state_id;
                            if default_luminance > 0 {
                                let sec = (local_y >> 4) as u32;
                                self.emissive_sections |= 1 << sec;
                            }
                        }
                        if block_idx >= CHUNK_DIM as usize {
                            block_idx -= CHUNK_DIM as usize;
                        }
                        continue;
                    }
                    let block_state = if density > 0.0 && vein.is_none() {
                        generator.default_block
                    } else if density <= 0.0 && sky_skip_y.is_some_and(|sy| block_y >= sy) {
                        Block::AIR.default_state
                    } else {
                        noise_sampler
                            .sample_block_state(
                                ore_random_deriver,
                                &Vector3::new(block_x, block_y, block_z),
                                density,
                                vein.as_ref(),
                                surface_height_estimate_sampler,
                            )
                            .unwrap_or(generator.default_block)
                    };

                    let local_y = block_y - bottom_y;
                    if local_y >= 0 && (local_y as usize) < chunk_height {
                        self.flat_block_map[block_idx] = block_state.id;
                        if block_state.luminance > 0 {
                            let sec = (local_y >> 4) as u32;
                            self.emissive_sections |= 1 << sec;
                        }

                        if block_state.id != air_state_id {
                            let y_i16 = block_y as i16;
                            if !surface_found {
                                self.flat_surface_height_map[hm_index] = y_i16;
                                surface_found = true;
                            }
                            if !all_hm_found {
                                if block_state.id == default_state_id {
                                    if !ocean_floor_found {
                                        self.flat_ocean_floor_height_map[hm_index] = y_i16;
                                        ocean_floor_found = true;
                                    }
                                    if !motion_blocking_found {
                                        self.flat_motion_blocking_height_map[hm_index] = y_i16;
                                        motion_blocking_found = true;
                                    }
                                    if !motion_no_leaves_found {
                                        self.flat_motion_blocking_no_leaves_height_map[hm_index] = y_i16;
                                        motion_no_leaves_found = true;
                                    }
                                    all_hm_found = true;
                                } else {
                                    let block = BlockId::from_state_id(block_state.id);
                                    let blocks_mov = blocks_movement(block_state, block);
                                    if blocks_mov && !ocean_floor_found {
                                        self.flat_ocean_floor_height_map[hm_index] = y_i16;
                                        ocean_floor_found = true;
                                    }
                                    let is_liquid = block_state.is_liquid();
                                    if (blocks_mov || is_liquid) && !motion_blocking_found {
                                        self.flat_motion_blocking_height_map[hm_index] = y_i16;
                                        motion_blocking_found = true;
                                    }
                                    if (blocks_mov || is_liquid)
                                        && !block.has_tag(tag::Block::MINECRAFT_LEAVES)
                                        && !motion_no_leaves_found
                                    {
                                        self.flat_motion_blocking_no_leaves_height_map[hm_index] = y_i16;
                                        motion_no_leaves_found = true;
                                    }
                                    if surface_found
                                        && ocean_floor_found
                                        && motion_blocking_found
                                        && motion_no_leaves_found
                                    {
                                        all_hm_found = true;
                                    }
                                }
                            }
                        }
                    }
                    if block_idx >= CHUNK_DIM as usize {
                        block_idx -= CHUNK_DIM as usize;
                    }
                }
            }
        }
    }

    pub fn spawn_mobs<T: GenerationCache>(cache: &mut T, block_registry: &dyn WorldPortalExt) {
        let chunk = cache.get_center_chunk();
        if chunk.stage >= StagedChunkEnum::Spawn {
            return;
        }
        debug_assert_eq!(chunk.stage, StagedChunkEnum::Lighting);

        let biome = chunk.get_terrain_gen_biome(
            section_to_block(chunk.x),
            chunk.bottom_y() as i32 + chunk.height() as i32 - 1,
            section_to_block(chunk.z),
        );
        let x = chunk.x;
        let z = chunk.z;

        block_registry.spawn_mobs_for_chunk_generation(cache, biome, x, z);

        let entities = cache
            .get_center_chunk_mut()
            .take_pending_structure_entities();
        block_registry.spawn_structure_entities(entities);

        cache.get_center_chunk_mut().stage = StagedChunkEnum::Spawn;
    }

    #[must_use]
    pub fn get_terrain_gen_biome_id(&self, x: i32, y: i32, z: i32) -> u8 {
        let biome_pos = self.get_terrain_gen_biome_pos(x, y, z);

        self.get_biome_id(biome_pos.x, biome_pos.y, biome_pos.z)
    }

    #[must_use]
    pub(crate) fn get_terrain_gen_biome_pos(&self, x: i32, y: i32, z: i32) -> Vector3<i32> {
        biome::get_biome_blend(
            self.bottom_y(),
            self.height(),
            self.biome_mixer_seed,
            x,
            y,
            z,
        )
    }

    pub(crate) fn get_terrain_gen_biome_id_from_neighborhood(
        &self,
        surface_biomes: &SurfaceBiomeNeighborhood,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<u8> {
        let biome_pos = self.get_terrain_gen_biome_pos(x, y, z);

        if biome_pos.x >> 2 == self.x && biome_pos.z >> 2 == self.z {
            return Some(self.get_biome_id(biome_pos.x, biome_pos.y, biome_pos.z));
        }

        surface_biomes
            .get_biome_id(biome_pos.x, biome_pos.y, biome_pos.z)
            .or_else(|| Some(self.get_biome_id(biome_pos.x, biome_pos.y, biome_pos.z)))
    }

    #[must_use]
    pub fn get_terrain_gen_biome(&self, x: i32, y: i32, z: i32) -> &'static Biome {
        Biome::from_id(self.get_terrain_gen_biome_id(x, y, z)).unwrap_or(&Biome::PLAINS)
    }

    #[expect(clippy::too_many_lines)]
    #[expect(
        clippy::panic,
        reason = "surface scheduling guarantees a complete 3x3 biome neighborhood"
    )]
    pub(crate) fn build_surface(
        &mut self,
        generator: &super::generator::VanillaGenerator,
        surface_biomes: &SurfaceBiomeNeighborhood,
        surface_height_estimate_sampler: &mut SurfaceHeightEstimateSampler,
    ) {
        let start_x = chunk_pos::start_block_x(self.x);
        let start_z = chunk_pos::start_block_z(self.z);
        let min_y = self.bottom_y();

        let settings = generator.settings;
        let random_config = &generator.random_config;
        let terrain_cache = &generator.terrain_cache;

        let random = &random_config.base_random_deriver;
        let mut context = MaterialRuleContext::new(
            self.generation_bottom_y(),
            self.generation_height(),
            random,
            &terrain_cache.terrain_builder,
            &terrain_cache.surface_noise,
            &terrain_cache.secondary_noise,
            settings.sea_level,
        )
        .with_terrain_cache(terrain_cache);

        let min_y_i32 = min_y as i32;
        let is_overworld = generator.dimension == Dimension::OVERWORLD;
        let has_sulfur_caves = surface_biomes.contains_biome(Biome::SULFUR_CAVES.id);

        let (bedrock_spl, deepslate_spl) = if is_overworld && !has_sulfur_caves {
            let b_spl = random
                .from_lo_and_hi(13544455532117611141u64, 14185350335435586452u64)
                .next_splitter();
            let d_spl = random
                .from_lo_and_hi(10411719568726253007u64, 14964796469053385315u64)
                .next_splitter();
            (Some(b_spl), Some(d_spl))
        } else {
            (None, None)
        };
        let bedrock_state_id = Block::BEDROCK.default_state.id;
        let deepslate_state_id = Block::DEEPSLATE.default_state.id;
        let default_state_id = self.default_block.id;

        for local_x in 0..16 {
            for local_z in 0..16 {
                let x = start_x + local_x;
                let z = start_z + local_z;
                let col_block_idx = self.column_stride * local_x as usize + local_z as usize;

                let mut top_block = self.top_block_height_exclusive(local_x, local_z);

                let biome_y = if settings.legacy_random_source {
                    0
                } else {
                    top_block
                };

                let this_biome = self
                    .get_terrain_gen_biome_id_from_neighborhood(surface_biomes, x, biome_y, z)
                    .unwrap_or_else(|| {
                        self.get_biome_id(x >> 2, biome_y >> 2, z >> 2)
                    });
                if this_biome == Biome::ERODED_BADLANDS {
                    terrain_cache
                        .terrain_builder
                        .place_badlands_pillar(self, x, z, top_block);

                    top_block = self.top_block_height_exclusive(local_x, local_z);
                }

                context.init_horizontal(x, z);

                let surface_estimate = if is_overworld {
                    estimate_surface_height(&mut context, surface_height_estimate_sampler)
                } else {
                    i32::MIN
                };

                if let (Some(b_spl), Some(d_spl)) = (&bedrock_spl, &deepslate_spl) {
                    let split_y = surface_estimate.min(8);
                    let mut stone_depth_above = 0;
                    let mut min = i32::MAX;
                    let mut fluid_height = i32::MIN;

                    for y in (split_y.max(min_y_i32)..top_block).rev() {
                        let local_y = y - min_y_i32;
                        let block_idx = col_block_idx + CHUNK_DIM as usize * local_y as usize;
                        let state = BlockState::from_id(self.flat_block_map[block_idx]);
                        if state.is_air() {
                            stone_depth_above = 0;
                            fluid_height = i32::MIN;
                            continue;
                        }
                        if state.is_liquid() {
                            if fluid_height == i32::MIN {
                                fluid_height = y + 1;
                            }
                            continue;
                        }
                        if (8..surface_estimate).contains(&y) && state.id == default_state_id {
                            stone_depth_above += 1;
                            continue;
                        }
                        if min >= y {
                            let shift = min_y << 4;
                            min = shift as i32;

                            for search_y in ((min_y_i32 - 1)..y).rev() {
                                if search_y < min_y_i32 {
                                    min = search_y + 1;
                                    break;
                                }

                                let local_search_y = search_y - min_y_i32;
                                let block_id = BlockId::from_state_id(
                                    self.flat_block_map
                                        [col_block_idx + CHUNK_DIM as usize * local_search_y as usize],
                                );

                                if !(block_id != AIR_BLOCK
                                    && block_id != WATER_BLOCK
                                    && block_id != LAVA_BLOCK)
                                {
                                    min = search_y + 1;
                                    break;
                                }
                            }
                        }

                        stone_depth_above += 1;
                        let stone_depth_below = y - min + 1;
                        context.init_vertical(stone_depth_above, stone_depth_below, y, fluid_height);

                        if state.id == default_state_id {
                            let biome_id = self
                                .get_terrain_gen_biome_id_from_neighborhood(
                                    surface_biomes,
                                    context.block_pos_x,
                                    context.block_pos_y,
                                    context.block_pos_z,
                                )
                                .unwrap_or_else(|| {
                                    self.get_biome_id(
                                        context.block_pos_x >> 2,
                                        context.block_pos_y >> 2,
                                        context.block_pos_z >> 2,
                                    )
                                });
                            context.biome = Biome::from_id(biome_id).unwrap_or(&Biome::PLAINS);
                            let new_state = try_apply_material_rule(
                                generator.surface_rule,
                                self,
                                &mut context,
                                surface_height_estimate_sampler,
                            );

                            if let Some(state) = new_state {
                                self.set_block_state(x, y, z, state);
                            }
                        }
                    }

                    let bedrock_bottom = min_y_i32;
                    let bedrock_top = min_y_i32 + 5;
                    let phase2_top = split_y.min(top_block);
                    for y in (min_y_i32..phase2_top).rev() {
                        let local_y = y - min_y_i32;
                        let block_idx = col_block_idx + CHUNK_DIM as usize * local_y as usize;
                        let state_id = self.flat_block_map[block_idx];
                        if state_id == default_state_id {
                            if y > 0 {
                                let mut rand = d_spl.split_pos(x, y, z);
                                let mapped = pumpkin_util::math::map(y as f32, 0.0, 8.0, 1.0, 0.0);
                                if rand.next_f32() < mapped {
                                    self.flat_block_map[block_idx] = deepslate_state_id;
                                }
                            } else if y >= bedrock_top {
                                self.flat_block_map[block_idx] = deepslate_state_id;
                            } else if y <= bedrock_bottom {
                                self.flat_block_map[block_idx] = bedrock_state_id;
                            } else {
                                let mut rand = b_spl.split_pos(x, y, z);
                                let mapped = pumpkin_util::math::map(
                                    y as f32,
                                    bedrock_bottom as f32,
                                    bedrock_top as f32,
                                    1.0,
                                    0.0,
                                );
                                if rand.next_f32() < mapped {
                                    self.flat_block_map[block_idx] = bedrock_state_id;
                                } else {
                                    self.flat_block_map[block_idx] = deepslate_state_id;
                                }
                            }
                        }
                    }
                } else {
                    let mut stone_depth_above = 0;
                    let mut min = i32::MAX;
                    let mut fluid_height = i32::MIN;
                    for y in (min_y_i32..top_block).rev() {
                        let local_y = y - min_y_i32;
                        let state = BlockState::from_id(
                            self.flat_block_map[col_block_idx + CHUNK_DIM as usize * local_y as usize],
                        );
                        if state.is_air() {
                            stone_depth_above = 0;
                            fluid_height = i32::MIN;
                            continue;
                        }
                        if state.is_liquid() {
                            if fluid_height == i32::MIN {
                                fluid_height = y + 1;
                            }
                            continue;
                        }
                        if min >= y {
                            let shift = min_y << 4;
                            min = shift as i32;

                            for search_y in ((min_y_i32 - 1)..y).rev() {
                                if search_y < min_y_i32 {
                                    min = search_y + 1;
                                    break;
                                }

                                let local_search_y = search_y - min_y_i32;
                                let block_id = BlockId::from_state_id(
                                    self.flat_block_map
                                        [col_block_idx + CHUNK_DIM as usize * local_search_y as usize],
                                );

                                if !(block_id != AIR_BLOCK
                                    && block_id != WATER_BLOCK
                                    && block_id != LAVA_BLOCK)
                                {
                                    min = search_y + 1;
                                    break;
                                }
                            }
                        }

                        stone_depth_above += 1;
                        let stone_depth_below = y - min + 1;
                        context.init_vertical(stone_depth_above, stone_depth_below, y, fluid_height);

                        if state.id == self.default_block.id {
                            if is_overworld && !has_sulfur_caves && (8..surface_estimate).contains(&y) {
                                continue;
                            }
                            let biome_id = self
                                .get_terrain_gen_biome_id_from_neighborhood(
                                    surface_biomes,
                                    context.block_pos_x,
                                    context.block_pos_y,
                                    context.block_pos_z,
                                )
                                .unwrap_or_else(|| {
                                    self.get_biome_id(
                                        context.block_pos_x >> 2,
                                        context.block_pos_y >> 2,
                                        context.block_pos_z >> 2,
                                    )
                                });
                            context.biome = Biome::from_id(biome_id).unwrap_or(&Biome::PLAINS);
                            let new_state = try_apply_material_rule(
                                generator.surface_rule,
                                self,
                                &mut context,
                                surface_height_estimate_sampler,
                            );

                            if let Some(state) = new_state {
                                self.set_block_state(x, y, z, state);
                            }
                        }
                    }
                }
                if this_biome == Biome::FROZEN_OCEAN || this_biome == Biome::DEEP_FROZEN_OCEAN {
                    let surface_estimate =
                        estimate_surface_height(&mut context, surface_height_estimate_sampler);

                    terrain_cache.terrain_builder.place_iceberg(
                        self,
                        Biome::from_id(this_biome).unwrap_or(&Biome::PLAINS),
                        x,
                        z,
                        surface_estimate,
                        top_block,
                        settings.sea_level,
                        &random_config.base_random_deriver,
                    );
                }
            }
        }
    }

    pub fn generate_features_and_structure<T: GenerationCache>(
        cache: &mut T,
        block_registry: &dyn WorldPortalExt,
        random_config: &GlobalRandomConfig,
    ) {
        let (center_x, center_z, min_y, generation_min_y, generation_height) = {
            let chunk = cache.get_center_chunk();
            (
                chunk.x,
                chunk.z,
                chunk.bottom_y() as i32,
                chunk.generation_bottom_y(),
                chunk.generation_height(),
            )
        };

        // Vanilla gathers every biome stored in the 3x3 chunk neighborhood before selecting the
        // globally ordered feature set.
        // Bitmask aggregation: 36 64-bit OR operations replace scanning 13,824 elements across 9 chunks.
        let mut combined_mask = [0u64; 4];
        for chunk_x in center_x - 1..=center_x + 1 {
            for chunk_z in center_z - 1..=center_z + 1 {
                if let Some(chunk) = cache.get_chunk(chunk_x, chunk_z) {
                    combined_mask[0] |= chunk.biome_mask[0];
                    combined_mask[1] |= chunk.biome_mask[1];
                    combined_mask[2] |= chunk.biome_mask[2];
                    combined_mask[3] |= chunk.biome_mask[3];
                }
            }
        }

        let mut possible_biomes = Vec::with_capacity(32);
        for (i, mut mask) in combined_mask.into_iter().enumerate() {
            while mask != 0 {
                let bit = mask.trailing_zeros();
                let biome_id = ((i as u8) << 6) | (bit as u8);
                possible_biomes.push(biome_id);
                mask &= mask - 1;
            }
        }

        let start_block_x = chunk_pos::start_block_x(center_x);
        let start_block_z = chunk_pos::start_block_z(center_z);
        let origin_pos = BlockPos::new(start_block_x, min_y, start_block_z);

        let population_seed =
            WorldgenRandom::get_population_seed(random_config.seed, start_block_x, start_block_z);

        let mut tasks_by_step: [Vec<_>; 11] = Default::default();
        {
            let center_chunk = cache.get_center_chunk();
            let center_x = center_chunk.x;
            let center_z = center_chunk.z;
            let start_x = chunk_pos::start_block_x(center_x);
            let start_z = chunk_pos::start_block_z(center_z);
            let end_x = start_x + 15;
            let end_z = start_z + 15;

            for (id, instance) in &center_chunk.structure_starts {
                let s = Structure::get(id);
                let step = s.step.ordinal();
                if step < 11 {
                    match instance {
                        StructureInstance::Start(pos) => tasks_by_step[step].push(pos.collector.clone()),
                        StructureInstance::Reference(collector) => {
                            let collector_arc = collector.clone();
                            if !tasks_by_step[step].iter().any(|t| Arc::ptr_eq(t, &collector_arc)) {
                                tasks_by_step[step].push(collector_arc);
                            }
                        }
                    }
                }
            }

            let radius = 8;
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if dx == 0 && dz == 0 {
                        continue;
                    }

                    let neighbor_x = center_x + dx;
                    let neighbor_z = center_z + dz;

                    if let Some(neighbor) = cache.try_get_proto_chunk(neighbor_x, neighbor_z) {
                        for (id, instance) in &neighbor.structure_starts {
                            let s = Structure::get(id);
                            let step = s.step.ordinal();
                            if step < 11 {
                                match instance {
                                    StructureInstance::Start(pos) => {
                                        if pos
                                            .get_bounding_box()
                                            .intersects_raw_xz(start_x, start_z, end_x, end_z)
                                        {
                                            let collector_arc = pos.collector.clone();
                                            if !tasks_by_step[step].iter().any(|t| Arc::ptr_eq(t, &collector_arc)) {
                                                tasks_by_step[step].push(collector_arc);
                                            }
                                        }
                                    }
                                    StructureInstance::Reference(collector) => {
                                        let collector_arc = collector.clone();
                                        if !tasks_by_step[step].iter().any(|t| Arc::ptr_eq(t, &collector_arc)) {
                                            tasks_by_step[step].push(collector_arc);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let world_seed = random_config.seed as i64;
        for step in 0..11 {
            let tasks = std::mem::take(&mut tasks_by_step[step]);
            if !tasks.is_empty() {
                let mut keyed_tasks: Vec<_> = tasks
                    .into_iter()
                    .map(|collector_arc| {
                        let bbox = collector_arc
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .get_bounding_box();
                        let key = (bbox.min.x, bbox.min.y, bbox.min.z, bbox.max.x, bbox.max.z);
                        (key, collector_arc)
                    })
                    .collect();
                keyed_tasks.sort_by_key(|(key, _)| *key);

                let decorator_seed = get_decorator_seed(population_seed, 0, step as u64);
                let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

                let chunk = cache.get_center_chunk_mut();
                for (_, collector_arc) in keyed_tasks {
                    let mut collector = collector_arc
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    collector.generate_in_chunk(chunk, block_registry, &mut random, world_seed);
                }
            }

            for_each_selected_feature(&possible_biomes, step, |global_index, feature_enum| {
                if let Some(feature) = PLACED_FEATURES.get(&feature_enum) {
                    let decorator_seed =
                        get_decorator_seed(population_seed, global_index as u64, step as u64);
                    let mut random =
                        RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

                    feature.generate(
                        cache,
                        block_registry,
                        generation_min_y,
                        generation_height,
                        feature_enum,
                        &mut random,
                        origin_pos,
                    );
                }
            });
        }

        cache.get_center_chunk_mut().stage = StagedChunkEnum::Features;
    }

    #[must_use]
    pub fn get_allowed_biomes(set: &StructureSet) -> Vec<u16> {
        let mut allowed_biomes = Vec::new();
        for entry in set.structures {
            let structure = Structure::get(&entry.structure);
            if let Some(biomes) = get_tag_ids(
                RegistryKey::WorldgenBiome,
                structure
                    .biomes
                    .strip_prefix('#')
                    .unwrap_or(structure.biomes),
            ) {
                allowed_biomes.extend_from_slice(biomes);
            }
        }
        allowed_biomes
    }

    pub fn set_structure_starts(&mut self, generator: &super::generator::VanillaGenerator) {
        debug_assert_eq!(self.stage, StagedChunkEnum::Biomes);
        let random_config = &generator.random_config;
        let settings = generator.settings;
        let global_cache = &generator.global_structure_cache;
        let calculator = &generator.structure_calculator;

        let seed = random_config.seed;

        let mut height_sampler = None;

        for &i in &generator.dimension_structure_sets {
            let set = &StructureSet::ALL[i];
            let allowed_biomes = &generator.structure_allowed_biomes[&i];

            if !should_generate_structure(
                &set.placement,
                calculator,
                self.x,
                self.z,
                global_cache,
                self,
                allowed_biomes,
            ) {
                continue;
            }

            let sampler = height_sampler.get_or_insert_with(|| {
                crate::generation::structure::height_sampler::NoiseHeightSampler::new(generator)
            });

            if set.structures.len() == 1 {
                if let Some(entry) = set.structures.first() {
                    self.try_set_structure_start(
                        global_cache,
                        settings.sea_level,
                        entry,
                        generator,
                        sampler,
                    );
                }
                continue;
            }

            let mut candidates = set.structures.to_vec();
            let large_feature_seed = get_large_feature_seed(seed, self.x, self.z);
            let mut random = LegacyRand::from_seed(large_feature_seed);

            let mut total_weight: u32 = candidates.iter().map(|e| e.weight).sum();

            while !candidates.is_empty() {
                let mut roll = random.next_bounded_i32(total_weight as i32);
                let mut selected_idx = 0;

                for (i, entry) in candidates.iter().enumerate() {
                    roll -= entry.weight as i32;
                    if roll < 0 {
                        selected_idx = i;
                        break;
                    }
                }

                let selected_entry = &candidates[selected_idx];

                if self.try_set_structure_start(
                    global_cache,
                    settings.sea_level,
                    selected_entry,
                    generator,
                    sampler,
                ) {
                    break;
                }

                let failed_entry = candidates.remove(selected_idx);
                total_weight -= failed_entry.weight;
            }
        }
        self.stage = StagedChunkEnum::StructureStart;
    }

    fn try_set_structure_start(
        &mut self,
        global_cache: &GlobalStructureCache,
        sea_level: i32,
        entry: &WeightedEntry,
        generator: &super::generator::VanillaGenerator,
        height_sampler: &mut dyn crate::generation::structure::structures::HeightSampler,
    ) -> bool {
        if entry.structure == StructureKeys::Monument {
            let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
            let center_x = chunk_pos::get_center_x(self.x);
            let center_z = chunk_pos::get_center_z(self.z);
            let start_y = height_sampler.estimate_ocean_floor_height(center_x, center_z);
            if !crate::generation::structure::structures::ocean_monument::has_valid_biomes(
                &MultiNoiseBiomeSupplier::OVERWORLD,
                &mut sampler,
                self.x,
                self.z,
                sea_level,
                start_y,
            ) {
                return false;
            }
        }

        let chunk_x = self.x;
        let chunk_z = self.z;
        let seed = generator.random_config.seed as i64;
        let chunk_min_y = self.bottom_y() as i32;
        let position =
            global_cache.get_or_compute_structure_start(entry.structure, chunk_x, chunk_z, || {
                let structure = Structure::get(&entry.structure);
                let dimension = &generator.dimension;
                let active_supplier = if *dimension == Dimension::THE_END {
                    ActiveSupplier::End(TheEndBiomeSupplier)
                } else if *dimension == Dimension::THE_NETHER {
                    ActiveSupplier::Nether(MultiNoiseBiomeSupplier::NETHER)
                } else {
                    ActiveSupplier::Overworld(MultiNoiseBiomeSupplier::OVERWORLD)
                };

                let base_supplier: &dyn BiomeSupplier = match &active_supplier {
                    ActiveSupplier::End(s) => s,
                    ActiveSupplier::Nether(s) | ActiveSupplier::Overworld(s) => s,
                };
                let blender = Blender::empty();
                let biome_supplier = blender.get_biome_supplier(base_supplier);
                let mut multi_noise_sampler =
                    MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                let mut height_sampler =
                    crate::generation::structure::height_sampler::NoiseHeightSampler::new(generator);

                let context = StructureGeneratorContext {
                    seed,
                    chunk_x,
                    chunk_z,
                    random: create_chunk_random(seed, chunk_x, chunk_z),
                    sea_level,
                    min_y: chunk_min_y,
                    height_sampler: Some(&mut height_sampler),
                    structure_key: Some(entry.structure),
                };
                lazily_generate_structure(
                    &entry.structure,
                    structure,
                    context,
                    &biome_supplier,
                    &mut multi_noise_sampler,
                )
            });

        if let Some(pos) = position {
            self.structure_starts
                .insert(entry.structure, StructureInstance::Start(pos));
            return true;
        }
        false
    }

    #[inline]
    pub const fn structure_set_max_chunk_radius(set_index: usize) -> i32 {
        match set_index {
            0 => 8,  // ancient_cities (116 blocks max range)
            1 => 1,  // buried_treasures (1 block footprint)
            2 => 2,  // desert_pyramids (21x21 blocks)
            3 => 8,  // end_cities (large towers/bridges up to 8 chunks)
            4 => 2,  // igloos (10x10 blocks)
            5 => 2,  // jungle_temples (15x15 blocks)
            6 => 8,  // mineshafts (corridors up to 8 chunks)
            7 => 8,  // nether_complexes (fortress bridges up to 8 chunks)
            8 => 2,  // nether_fossils (small fossil pieces)
            9 => 4,  // ocean_monuments (58x58 blocks, radius 29 blocks)
            10 => 3, // ocean_ruins (~40 blocks)
            11 => 4, // pillager_outposts (watchtower + tents up to 48 blocks)
            12 => 2, // ruined_portals (~25 blocks)
            13 => 2, // shipwrecks (~28 blocks)
            14 => 8, // strongholds (labyrinth up to 8 chunks)
            15 => 2, // swamp_huts (9x9 blocks)
            16 => 3, // trail_ruins (~40 blocks)
            17 => 6, // trial_chambers (80 blocks max distance)
            18 => 6, // villages (80 blocks max distance)
            19 => 7, // woodland_mansions (96x96 blocks)
            _ => 8,
        }
    }

    #[expect(clippy::too_many_lines)]
    pub fn set_structure_references(&mut self, generator: &super::generator::VanillaGenerator) {
        debug_assert_eq!(self.stage, StagedChunkEnum::StructureStart);
        let prof = STRUCT_REF_PROF_ENABLED.load(Ordering::Relaxed);
        let fn_start = if prof { Some(std::time::Instant::now()) } else { None };

        let random_config = &generator.random_config;
        let settings = generator.settings;
        let dimension = &generator.dimension;
        let noise_router = &generator.base_router;
        let global_cache = &generator.global_structure_cache;
        let calculator = &generator.structure_calculator;

        let start_x = chunk_pos::start_block_x(self.x);
        let start_z = chunk_pos::start_block_z(self.z);
        let end_x = start_x + 15;
        let end_z = start_z + 15;

        let seed = random_config.seed as i64;

        let active_supplier = if *dimension == Dimension::THE_END {
            ActiveSupplier::End(TheEndBiomeSupplier)
        } else if *dimension == Dimension::THE_NETHER {
            ActiveSupplier::Nether(MultiNoiseBiomeSupplier::NETHER)
        } else {
            ActiveSupplier::Overworld(MultiNoiseBiomeSupplier::OVERWORLD)
        };

        let base_supplier: &dyn BiomeSupplier = match &active_supplier {
            ActiveSupplier::End(s) => s,
            ActiveSupplier::Nether(s) | ActiveSupplier::Overworld(s) => s,
        };
        let blender = Blender::empty();
        let biome_supplier = blender.get_biome_supplier(base_supplier);
        let mut multi_noise_sampler = MultiNoiseSampler::generate(&noise_router.multi_noise);

        let mut height_sampler =
            crate::generation::structure::height_sampler::NoiseHeightSampler::new(generator);

        let mut references = Vec::new();
        // Constant across every chunk in the dimension, so hoist it out of the loop
        // and out of the (cached) structure-start computation below.
        let chunk_min_y = self.bottom_y() as i32;

        for &set_index in &generator.dimension_structure_sets {
            let set = &StructureSet::ALL[set_index];
            let set_start = if prof { Some(std::time::Instant::now()) } else { None };
            // RandomSpread always yields exactly 9 candidates (a 3x3 region scan) and
            // ConcentricRings only a handful; reserving capacity up front avoids the
            // 2-3 reallocations `Vec::new()` would otherwise trigger while filling
            // this, for every structure set (~20), on every single chunk generated.
            let mut candidate_chunks = Vec::with_capacity(9);

            let t_cand = if prof { Some(std::time::Instant::now()) } else { None };
            match &set.placement.placement_type {
                StructurePlacementType::RandomSpread(spread) => {
                    let region_x = pumpkin_util::math::floor_div(self.x, spread.spacing);
                    let region_z = pumpkin_util::math::floor_div(self.z, spread.spacing);

                    for rx in (region_x - 1)..=(region_x + 1) {
                        for rz in (region_z - 1)..=(region_z + 1) {
                            candidate_chunks.push(
                                crate::generation::structure::placement::get_structure_chunk_in_region(
                                    spread,
                                    seed,
                                    rx,
                                    rz,
                                    set.placement.salt,
                                )
                            );
                        }
                    }
                }
                StructurePlacementType::ConcentricRings(rings) => {
                    let min_dist_chunks = (rings.distance * 4 - (rings.distance * 5 / 4) - 8).max(0);
                    let max_radius = Self::structure_set_max_chunk_radius(set_index);
                    let safe_dist = (min_dist_chunks - max_radius).max(0);
                    if self.x.saturating_mul(self.x) + self.z.saturating_mul(self.z) >= safe_dist * safe_dist {
                        let allowed_biomes = Self::get_allowed_biomes(set);
                        let strongholds = global_cache.get_or_calculate_strongholds(
                            seed,
                            rings,
                            self,
                            &allowed_biomes,
                        );
                        for &(cx, cz) in strongholds {
                            if (cx - self.x).abs() <= max_radius && (cz - self.z).abs() <= max_radius {
                                candidate_chunks.push((cx, cz));
                            }
                        }
                    }
                }
            }
            if prof {
                if let Some(tc) = t_cand {
                    PROF_CANDIDATE_MATH_NS.fetch_add(tc.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
            }

            let max_radius = Self::structure_set_max_chunk_radius(set_index);
            for (candidate_chunk_x, candidate_chunk_z) in candidate_chunks {
                if (candidate_chunk_x - self.x).abs() > max_radius
                    || (candidate_chunk_z - self.z).abs() > max_radius
                {
                    continue;
                }

                if prof {
                    PROF_SET_CALLS[set_index].fetch_add(1, Ordering::Relaxed);
                }

                let t_should = if prof { Some(std::time::Instant::now()) } else { None };
                let generates = should_generate_structure(
                    &set.placement,
                    calculator,
                    candidate_chunk_x,
                    candidate_chunk_z,
                    global_cache,
                    self,
                    &generator.structure_allowed_biomes[&set_index],
                );
                if prof {
                    if let Some(ts) = t_should {
                        PROF_SHOULD_GENERATE_NS.fetch_add(ts.elapsed().as_nanos() as u64, Ordering::Relaxed);
                    }
                }

                if !generates {
                    continue;
                }

                // Early biome rejection: verify if the candidate chunk has any allowed biomes for this structure set.
                // We scan all 16 quart coordinates (4x4) of the candidate chunk and break immediately on the first match.
                let allowed_biomes = &generator.structure_allowed_biomes[&set_index];
                let base_bx = candidate_chunk_x * 4;
                let base_bz = candidate_chunk_z * 4;
                let test_by = biome_coords::from_block(if *dimension == Dimension::THE_NETHER {
                    32
                } else if set_index == 0 {
                    -27 // ancient_cities
                } else {
                    settings.sea_level
                });

                let mut has_allowed_biome = false;
                'biome_check: for dx in 0..4 {
                    for dz in 0..4 {
                        let b = biome_supplier.biome(base_bx + dx, test_by, base_bz + dz, &mut multi_noise_sampler).id as u16;
                        if allowed_biomes.contains(&b) {
                            has_allowed_biome = true;
                            break 'biome_check;
                        }
                    }
                }
                if !has_allowed_biome {
                    continue;
                }

                if prof {
                    PROF_SET_HITS[set_index].fetch_add(1, Ordering::Relaxed);
                }

                for entry in set.structures {
                    let structure = Structure::get(&entry.structure);

                    let t_comp = if prof { Some(std::time::Instant::now()) } else { None };
                    let start_data = global_cache.get_or_compute_structure_start(
                        entry.structure,
                        candidate_chunk_x,
                        candidate_chunk_z,
                        || {
                            let context = StructureGeneratorContext {
                                seed,
                                chunk_x: candidate_chunk_x,
                                chunk_z: candidate_chunk_z,
                                random: create_chunk_random(
                                    seed,
                                    candidate_chunk_x,
                                    candidate_chunk_z,
                                ),
                                sea_level: settings.sea_level,
                                min_y: chunk_min_y,
                                height_sampler: Some(&mut height_sampler),
                                structure_key: Some(entry.structure),
                            };
                            lazily_generate_structure(
                                &entry.structure,
                                structure,
                                context,
                                &biome_supplier,
                                &mut multi_noise_sampler,
                            )
                        },
                    );
                    if prof {
                        if let Some(tc) = t_comp {
                            PROF_COMPUTE_START_NS.fetch_add(tc.elapsed().as_nanos() as u64, Ordering::Relaxed);
                        }
                    }

                    let t_bb = if prof { Some(std::time::Instant::now()) } else { None };
                    if let Some(start_data) = start_data
                        && start_data
                            .get_bounding_box()
                            .intersects_raw_xz(start_x, start_z, end_x, end_z)
                    {
                        references.push((entry.structure, start_data.collector.clone()));
                        if prof {
                            if let Some(tbb) = t_bb {
                                PROF_BBOX_NS.fetch_add(tbb.elapsed().as_nanos() as u64, Ordering::Relaxed);
                            }
                        }
                        break;
                    }
                    if prof {
                        if let Some(tbb) = t_bb {
                            PROF_BBOX_NS.fetch_add(tbb.elapsed().as_nanos() as u64, Ordering::Relaxed);
                        }
                    }
                }
            }

            if prof {
                if let Some(ts) = set_start {
                    PROF_SET_TIMES_NS[set_index].fetch_add(ts.elapsed().as_nanos() as u64, Ordering::Relaxed);
                }
            }
        }

        for (key, pos) in references {
            self.structure_starts
                .entry(key)
                .or_insert_with(|| StructureInstance::Reference(pos));
        }

        if prof {
            if let Some(tf) = fn_start {
                PROF_TOTAL_NS.fetch_add(tf.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }
        }

        self.stage = StagedChunkEnum::StructureReferences;
    }

    const fn start_cell_x(&self, horizontal_cell_block_count: i32) -> i32 {
        self.start_block_x() / horizontal_cell_block_count
    }

    const fn start_cell_z(&self, horizontal_cell_block_count: i32) -> i32 {
        self.start_block_z() / horizontal_cell_block_count
    }

    const fn start_block_x(&self) -> i32 {
        start_block_x(self.x)
    }

    const fn start_block_z(&self) -> i32 {
        start_block_z(self.z)
    }
}

impl BlockAccessor for ProtoChunk {
    #[inline]
    fn get_block(&self, position: &BlockPos) -> &'static Block {
        self.get_block_state(&position.0).to_block()
    }

    #[inline]
    fn get_block_state(&self, position: &BlockPos) -> &'static BlockState {
        self.get_block_state(&position.0).to_state()
    }

    #[inline]
    fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
        self.get_block_state(&position.0)
    }

    #[inline]
    fn get_block_and_state(&self, position: &BlockPos) -> (&'static Block, &'static BlockState) {
        let id = self.get_block_state(&position.0);
        BlockState::from_id_with_block(id)
    }
}

impl BlockPlacer for ProtoChunk {
    #[inline]
    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId {
        self.get_block_state(pos)
    }

    #[inline]
    fn set_block_state(&mut self, pos: &Vector3<i32>, state: &BlockState) {
        Self::set_block_state(self, pos.x, pos.y, pos.z, state);
    }

    #[inline]
    fn add_block_entity(&mut self, nbt: NbtCompound) {
        self.add_block_entity(nbt);
    }
}

impl GenerationCache for ProtoChunk {
    fn get_center_chunk_mut(&mut self) -> &mut ProtoChunk {
        self
    }
    fn get_center_chunk(&self) -> &ProtoChunk {
        self
    }
    fn get_chunk_mut(&mut self, cx: i32, cz: i32) -> Option<&mut ProtoChunk> {
        (cx == self.x && cz == self.z).then_some(self)
    }
    fn get_chunk(&self, cx: i32, cz: i32) -> Option<&ProtoChunk> {
        (cx == self.x && cz == self.z).then_some(self)
    }
    fn try_get_proto_chunk(&self, cx: i32, cz: i32) -> Option<&ProtoChunk> {
        self.get_chunk(cx, cz)
    }
    #[inline]
    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId {
        Self::get_block_state(self, pos)
    }
    fn get_fluid_and_fluid_state(&self, _pos: &Vector3<i32>) -> (Fluid, FluidState) {
        (
            Fluid::EMPTY,
            FluidState {
                height: 0.0,
                level: 0,
                is_empty: true,
                blast_resistance: 0.0,
                block_state_id: BlockStateId::AIR,
                is_still: false,
                is_source: false,
                falling: false,
            },
        )
    }
    #[inline]
    fn set_block_state(&mut self, pos: &Vector3<i32>, block_state: &BlockState) {
        Self::set_block_state(self, pos.x, pos.y, pos.z, block_state);
    }
    fn add_block_entity(&mut self, _pos: &Vector3<i32>, nbt: NbtCompound) {
        self.add_block_entity(nbt);
    }
    #[inline]
    fn top_motion_blocking_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::top_motion_blocking_block_height_exclusive(self, x, z)
    }
    #[inline]
    fn top_motion_blocking_block_no_leaves_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::top_motion_blocking_block_no_leaves_height_exclusive(self, x, z)
    }
    #[inline]
    fn get_top_y(&self, heightmap: &HeightMap, x: i32, z: i32) -> i32 {
        Self::get_top_y(self, heightmap, x, z)
    }
    #[inline]
    fn top_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::top_block_height_exclusive(self, x, z)
    }
    #[inline]
    fn top_block_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::top_block_wg_height_exclusive(self, x, z)
    }
    #[inline]
    fn ocean_floor_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::ocean_floor_height_exclusive(self, x, z)
    }
    #[inline]
    fn ocean_floor_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        Self::ocean_floor_wg_height_exclusive(self, x, z)
    }
    fn get_world_seed(&self) -> u64 {
        self.world_seed
    }
    #[inline]
    fn is_air(&self, local_pos: &Vector3<i32>) -> bool {
        self.is_air(local_pos)
    }
    #[inline]
    fn get_biome_for_terrain_gen(&self, x: i32, y: i32, z: i32) -> &'static Biome {
        Self::get_terrain_gen_biome(self, x, y, z)
    }
    fn get_blending_data(
        &self,
        _cx: i32,
        _cz: i32,
    ) -> Option<&crate::generation::blender::blending_data::BlendingData> {
        None
    }
    fn get_sea_level(&self) -> i32 {
        Self::get_sea_level(self)
    }
}
