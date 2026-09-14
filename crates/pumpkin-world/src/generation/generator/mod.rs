use pumpkin_data::BlockState;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::material_rule::MaterialRule;
use pumpkin_data::noise_router::{
    END_BASE_NOISE_ROUTER, NETHER_BASE_NOISE_ROUTER, OVERWORLD_BASE_NOISE_ROUTER,
};
use pumpkin_data::noise_settings::NoiseSettings;

use super::noise::router::proto_noise_router::ProtoNoiseRouters;
use crate::generation::proto_chunk::TerrainCache;
use crate::generation::{GlobalRandomConfig, Seed};

pub mod biome_finder;
pub mod structure_finder;

pub trait GeneratorInit {
    fn new(seed: Seed, dimension: Dimension) -> Self;
}

use pumpkin_data::structures::{StructurePlacementCalculator, StructureSet};
use rustc_hash::FxHashMap;

use std::sync::Arc;

use crate::chunk_system::StagedChunkEnum;
use crate::generation::proto_chunk::ProtoChunk;

pub mod flat;

#[derive(Clone, Debug)]
pub struct FlatLayer {
    pub block: String,
    pub height: i32,
}

pub trait CustomChunkGenerator: Send + Sync {
    fn dimension(&self) -> &Dimension;
    fn seed(&self) -> u64;
    fn default_block(&self) -> &'static BlockState {
        pumpkin_data::Block::AIR.default_state
    }
    fn biome_mixer_seed(&self) -> i64 {
        0
    }
    fn global_structure_cache(
        &self,
    ) -> Option<&crate::generation::structure::placement::GlobalStructureCache> {
        None
    }

    fn step_to_biomes(&self, chunk: &mut ProtoChunk) {
        chunk.stage = StagedChunkEnum::Biomes;
    }

    fn step_to_noise(&self, chunk: &mut ProtoChunk) {
        chunk.stage = StagedChunkEnum::Noise;
    }

    fn step_to_surface(&self, chunk: &mut ProtoChunk) {
        chunk.stage = StagedChunkEnum::Surface;
    }

    fn step_to_carvers(&self, chunk: &mut ProtoChunk) {
        chunk.stage = StagedChunkEnum::Carvers;
    }

    fn step_to_features(
        &self,
        cache: &mut crate::chunk_system::generation_cache::Cache,
        _block_registry: &dyn crate::world::WorldPortalExt,
    ) {
        let mid = ((cache.size * cache.size) >> 1) as usize;
        cache.chunks[mid].get_proto_chunk_mut().stage = StagedChunkEnum::Features;
    }

    fn set_structure_starts(&self, _chunk: &mut ProtoChunk) {}
    fn set_structure_references(&self, _chunk: &mut ProtoChunk) {}
    fn is_in_structure_bounds(
        &self,
        _key: pumpkin_data::structures::StructureKeys,
        _pos: &pumpkin_util::math::position::BlockPos,
    ) -> bool {
        false
    }
}

pub enum WorldGenerator {
    Noise(Box<VanillaGenerator>),
    Flat(Box<flat::FlatGenerator>),
    Custom(Arc<dyn CustomChunkGenerator>),
}

impl WorldGenerator {
    #[must_use]
    pub fn dimension(&self) -> &Dimension {
        match self {
            Self::Noise(noise_gen) => &noise_gen.dimension,
            Self::Flat(flat_gen) => &flat_gen.dimension,
            Self::Custom(custom_gen) => custom_gen.dimension(),
        }
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        match self {
            Self::Noise(noise_gen) => noise_gen.random_config.seed,
            Self::Flat(flat_gen) => flat_gen.seed,
            Self::Custom(custom_gen) => custom_gen.seed(),
        }
    }

    #[must_use]
    pub fn global_structure_cache(
        &self,
    ) -> Option<&crate::generation::structure::placement::GlobalStructureCache> {
        match self {
            Self::Noise(noise_gen) => Some(&noise_gen.global_structure_cache),
            Self::Flat(_) => None,
            Self::Custom(custom_gen) => custom_gen.global_structure_cache(),
        }
    }

    #[must_use]
    pub fn find_spawn_position(&self) -> pumpkin_util::math::position::BlockPos {
        match self {
            Self::Noise(noise_gen) => noise_gen.find_spawn_position(),
            _ => pumpkin_util::math::position::BlockPos::ZERO,
        }
    }

    #[must_use]
    pub fn is_in_structure_bounds(
        &self,
        key: pumpkin_data::structures::StructureKeys,
        pos: &pumpkin_util::math::position::BlockPos,
    ) -> bool {
        match self {
            Self::Noise(noise_gen) => noise_gen.is_in_structure_bounds(key, pos),
            Self::Flat(_) => false,
            Self::Custom(custom_gen) => custom_gen.is_in_structure_bounds(key, pos),
        }
    }
}

pub struct VanillaGenerator {
    pub random_config: GlobalRandomConfig,
    pub base_router: ProtoNoiseRouters,
    pub dimension: Dimension,
    pub settings: &'static NoiseSettings,
    pub surface_rule: &'static MaterialRule,
    pub biome_mixer_seed: i64,

    pub terrain_cache: TerrainCache,

    pub default_block: &'static BlockState,

    pub global_structure_cache: crate::generation::structure::placement::GlobalStructureCache,
    pub structure_calculator: StructurePlacementCalculator,
    pub structure_allowed_biomes: FxHashMap<usize, Vec<u16>>,
    pub dimension_structure_sets: Box<[usize]>,
}

impl VanillaGenerator {
    #[must_use]
    pub fn find_spawn_position(&self) -> pumpkin_util::math::position::BlockPos {
        if self.settings.spawn_target.is_empty() {
            return pumpkin_util::math::position::BlockPos::ZERO;
        }
        let mut sampler =
            crate::generation::noise::router::multi_noise_sampler::MultiNoiseSampler::generate(
                &self.base_router.multi_noise,
            );
        crate::biome::position_finder::SpawnFinder::find_spawn_position(
            self.settings.spawn_target,
            &mut sampler,
        )
    }

    #[must_use]
    pub fn is_in_structure_bounds(
        &self,
        key: pumpkin_data::structures::StructureKeys,
        pos: &pumpkin_util::math::position::BlockPos,
    ) -> bool {
        let set = key.structure_set();
        let pumpkin_data::structures::StructurePlacementType::RandomSpread(spread) =
            &set.placement.placement_type
        else {
            return false;
        };

        let chunk_x = pos.0.x >> 4;
        let chunk_z = pos.0.z >> 4;
        let region_x = pumpkin_util::math::floor_div(chunk_x, spread.spacing);
        let region_z = pumpkin_util::math::floor_div(chunk_z, spread.spacing);
        let seed = self.random_config.seed as i64;

        for dx in -1..=1 {
            for dz in -1..=1 {
                let rx = region_x + dx;
                let rz = region_z + dz;
                let (scx, scz) =
                    crate::generation::structure::placement::get_structure_chunk_in_region(
                        spread,
                        seed,
                        rx,
                        rz,
                        set.placement.salt,
                    );

                let max_chunk_dist = match key {
                    pumpkin_data::structures::StructureKeys::PillagerOutpost => 5,
                    pumpkin_data::structures::StructureKeys::SwampHut => 2,
                    _ => 4,
                };
                if (scx - chunk_x).abs() > max_chunk_dist || (scz - chunk_z).abs() > max_chunk_dist
                {
                    continue;
                }

                let start = self.global_structure_cache.get_or_compute_structure_start(
                    key,
                    scx,
                    scz,
                    || {
                        let settings = self.settings;
                        let mut height_sampler =
                            crate::generation::structure::height_sampler::NoiseHeightSampler::new(
                                self,
                            );
                        let mut biome_sampler =
                            crate::generation::noise::router::multi_noise_sampler::MultiNoiseSampler::generate(
                                &self.base_router.multi_noise,
                            );
                        let context =
                            crate::generation::structure::structures::StructureGeneratorContext {
                                seed,
                                chunk_x: scx,
                                chunk_z: scz,
                                random:
                                    crate::generation::structure::structures::create_chunk_random(
                                        seed, scx, scz,
                                    ),
                                sea_level: settings.sea_level,
                                min_y: self.dimension.min_y,
                                height_sampler: Some(&mut height_sampler),
                                structure_key: Some(key),
                            };
                        let biome_supplier: &dyn crate::biome::BiomeSupplier =
                            if self.dimension == Dimension::THE_END {
                                &crate::biome::end::TheEndBiomeSupplier
                            } else if self.dimension == Dimension::THE_NETHER {
                                &crate::biome::MultiNoiseBiomeSupplier::NETHER
                            } else {
                                &crate::biome::MultiNoiseBiomeSupplier::OVERWORLD
                            };
                        crate::generation::structure::lazily_generate_structure(
                            &key,
                            pumpkin_data::structures::Structure::get(&key),
                            context,
                            biome_supplier,
                            &mut biome_sampler,
                        )
                    },
                );

                if let Some(start) = start {
                    let mut collector = start
                        .collector
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let bbox = collector.get_bounding_box();
                    if bbox.contains_pos(&pos.0) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl GeneratorInit for VanillaGenerator {
    fn new(seed: Seed, dimension: Dimension) -> Self {
        let settings = NoiseSettings::from_dimension(&dimension);
        let surface_rule = MaterialRule::from_dimension(&dimension);
        let random_config = GlobalRandomConfig::new(seed.0, settings.legacy_random_source);

        // TODO: The generation settings contains (part of?) the noise routers too; do we keep the separate or
        // use only the generation settings?
        let base = if dimension == Dimension::OVERWORLD {
            OVERWORLD_BASE_NOISE_ROUTER
        } else if dimension == Dimension::THE_NETHER {
            NETHER_BASE_NOISE_ROUTER
        } else if dimension == Dimension::THE_END {
            END_BASE_NOISE_ROUTER
        } else {
            tracing::error!("Unsupported dimension for noise router: {:?}", dimension);
            OVERWORLD_BASE_NOISE_ROUTER
        };
        let terrain_cache = TerrainCache::from_random(&random_config);

        let default_block = settings.default_block;
        let base_router = ProtoNoiseRouters::generate(&base, &random_config);
        let biome_mixer_seed = crate::biome::hash_seed(seed.0);

        let mut structure_allowed_biomes = FxHashMap::default();
        for (i, set) in StructureSet::ALL.iter().enumerate() {
            structure_allowed_biomes.insert(
                i,
                crate::generation::proto_chunk::ProtoChunk::get_allowed_biomes(set),
            );
        }

        let dim_biomes: &[u16] = if dimension == Dimension::THE_NETHER {
            pumpkin_data::tag::WorldgenBiome::MINECRAFT_IS_NETHER.1
        } else if dimension == Dimension::THE_END {
            pumpkin_data::tag::WorldgenBiome::MINECRAFT_IS_END.1
        } else {
            pumpkin_data::tag::WorldgenBiome::MINECRAFT_IS_OVERWORLD.1
        };

        let dimension_structure_sets: Box<[usize]> = StructureSet::ALL
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                let allowed = &structure_allowed_biomes[&i];
                if allowed.iter().any(|b| dim_biomes.contains(b)) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        Self {
            random_config,
            base_router,
            dimension,
            settings,
            surface_rule,
            biome_mixer_seed,
            terrain_cache,
            default_block,
            global_structure_cache:
                crate::generation::structure::placement::GlobalStructureCache::new(),
            structure_calculator: StructurePlacementCalculator::new(seed.0 as i64),
            structure_allowed_biomes,
            dimension_structure_sets,
        }
    }
}
