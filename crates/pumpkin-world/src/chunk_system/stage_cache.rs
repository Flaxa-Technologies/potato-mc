use std::sync::{Arc, OnceLock};

use crate::generation::biome_coords;

use dashmap::DashMap;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ProtoChunk;
use crate::chunk_system::StagedChunkEnum;
use crate::generation::generator::WorldGenerator;
use crate::generation::height_limit::HeightLimitView;
use crate::generation::proto_chunk::GenerationCache;
use crate::world::BlockAccessor;
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::block_properties::is_air;
use pumpkin_data::chunk::Biome;
use pumpkin_data::fluid::{Fluid, FluidState};
use pumpkin_data::{Block, BlockState, BlockStateId};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::HeightMap;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

/// Per-batch shared cache of ProtoChunks already advanced to a given stage.
///
/// When generating N chunks concurrently, each output chunk (cx, cz) needs its
/// 3×3 neighbors advanced through Biomes and StructureStart before it can proceed
/// to Noise. Without this cache, interior neighbors are recomputed up to 9× each.
///
/// **Single-flight guarantee:** Each (x, z) slot uses `Arc<OnceLock<…>>`.
/// The first thread to request a slot computes it; all concurrent callers share
/// the same Arc and block on `get_or_init` until the result is stored.
///
/// **Lifetime:** Created fresh per `generate_batch_chunks` call. Dropped when the
/// batch completes — no cross-batch stale data.
///
/// **Correctness (Steel Trap 1):** No thread-local noise state is stored in the
/// cache — each computation creates its own `MultiNoiseSampler` on the stack.
/// The biome map stored in the cached ProtoChunk is the *completed* output, never
/// a partially-computed intermediate.
///
/// **Correctness (Steel Trap 2):** `structure_starts` in `ProtoChunk` is written
/// exactly once by `set_structure_starts` and then only read. The `FxHashMap`
/// iteration order is fixed at write time, not re-ordered by caching.
pub struct StageCache {
    biomes: DashMap<(i32, i32), Arc<OnceLock<Arc<ProtoChunk>>>, rustc_hash::FxBuildHasher>,
    structure_start: DashMap<(i32, i32), Arc<OnceLock<Arc<ProtoChunk>>>, rustc_hash::FxBuildHasher>,
}

impl StageCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            biomes: DashMap::with_hasher(rustc_hash::FxBuildHasher),
            structure_start: DashMap::with_hasher(rustc_hash::FxBuildHasher),
        }
    }

    /// Get-or-compute a ProtoChunk at `(cx, cz)` advanced through the Biomes stage.
    /// Returns an `Arc` to the cached chunk — cheap to clone, safe to share.
    pub fn get_or_compute_biomes(
        &self,
        cx: i32,
        cz: i32,
        generator: &WorldGenerator,
    ) -> Arc<ProtoChunk> {
        let slot_arc = if let Some(slot) = self.biomes.get(&(cx, cz)) {
            slot.clone()
        } else {
            self.biomes
                .entry((cx, cz))
                .or_insert_with(|| Arc::new(OnceLock::new()))
                .clone()
        };

        let result = slot_arc.get_or_init(|| {
            let mut chunk = ProtoChunk::new(cx, cz, generator);
            match generator {
                WorldGenerator::Noise(noise_gen) => chunk.step_to_biomes(noise_gen),
                WorldGenerator::Flat(flat_gen) => flat_gen.step_to_biomes(&mut chunk),
                WorldGenerator::Custom(custom_gen) => custom_gen.step_to_biomes(&mut chunk),
            }
            Arc::new(chunk)
        });
        Arc::clone(result)
    }

    /// Get-or-compute a ProtoChunk at `(cx, cz)` advanced through StructureStart.
    /// Computes both Biomes and StructureStart directly on a single chunk to avoid
    /// allocating an intermediate chunk copy.
    pub fn get_or_compute_structure_start(
        &self,
        cx: i32,
        cz: i32,
        generator: &WorldGenerator,
    ) -> Arc<ProtoChunk> {
        let slot_arc = if let Some(slot) = self.structure_start.get(&(cx, cz)) {
            slot.clone()
        } else {
            self.structure_start
                .entry((cx, cz))
                .or_insert_with(|| Arc::new(OnceLock::new()))
                .clone()
        };

        let result = slot_arc.get_or_init(|| {
            let mut chunk = ProtoChunk::new(cx, cz, generator);
            match generator {
                WorldGenerator::Noise(noise_gen) => {
                    chunk.step_to_biomes(noise_gen);
                    chunk.set_structure_starts(noise_gen);
                }
                WorldGenerator::Flat(flat_gen) => {
                    flat_gen.step_to_biomes(&mut chunk);
                    chunk.stage = StagedChunkEnum::StructureStart;
                }
                WorldGenerator::Custom(custom_gen) => {
                    custom_gen.step_to_biomes(&mut chunk);
                    custom_gen.set_structure_starts(&mut chunk);
                }
            }
            Arc::new(chunk)
        });
        Arc::clone(result)
    }
}

impl Default for StageCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Clone the minimum fields needed to continue generation from the Biomes stage.
/// Only copies biome map + dimension metadata. Does NOT copy block map (empty at
/// this stage), lighting, carving mask, or height maps (all zeroed).
fn clone_at_biome_stage(chunk: &ProtoChunk) -> ProtoChunk {
    let mut cloned = ProtoChunk::new_raw(chunk.x, chunk.z, chunk);
    cloned.stage = StagedChunkEnum::Biomes;
    cloned
}

// ─── Batch generation entry-point ───────────────────────────────────────────

/// Generate a batch of chunks, sharing Biomes and StructureStart computations
/// for overlapping 3×3 neighbor windows across concurrent worker threads.
///
/// **Thread safety:** `StageCache` is `Send + Sync`. Worker threads run via
/// `rayon` parallel iterator and each take independent ownership of their center chunk.
/// Neighbor chunks are accessed read-only after their OnceLock is initialised.
struct BatchGridCache<'a> {
    center_x: i32,
    center_z: i32,
    chunks: &'a mut FxHashMap<(i32, i32), ProtoChunk>,
}

impl<'a> HeightLimitView for BatchGridCache<'a> {
    fn height(&self) -> u16 {
        self.get_center_chunk().height()
    }

    fn bottom_y(&self) -> i8 {
        self.get_center_chunk().bottom_y()
    }
}

impl<'a> BlockAccessor for BatchGridCache<'a> {
    fn get_block(&self, position: &BlockPos) -> &'static Block {
        GenerationCache::get_block_state(self, &position.0).to_block()
    }

    fn get_block_state(&self, position: &BlockPos) -> &'static BlockState {
        GenerationCache::get_block_state(self, &position.0).to_state()
    }

    fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
        GenerationCache::get_block_state(self, &position.0)
    }

    fn get_block_and_state(&self, position: &BlockPos) -> (&'static Block, &'static BlockState) {
        let id = GenerationCache::get_block_state(self, &position.0);
        BlockState::from_id_with_block(id)
    }
}

impl<'a> GenerationCache for BatchGridCache<'a> {
    fn get_center_chunk_mut(&mut self) -> &mut ProtoChunk {
        self.chunks
            .get_mut(&(self.center_x, self.center_z))
            .expect("center chunk must exist in grid")
    }

    fn get_center_chunk(&self) -> &ProtoChunk {
        self.chunks
            .get(&(self.center_x, self.center_z))
            .expect("center chunk must exist in grid")
    }

    fn get_world_seed(&self) -> u64 {
        self.get_center_chunk().world_seed
    }

    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut ProtoChunk> {
        self.chunks.get_mut(&(chunk_x, chunk_z))
    }

    fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&ProtoChunk> {
        self.chunks.get(&(chunk_x, chunk_z))
    }

    fn try_get_proto_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&ProtoChunk> {
        self.chunks.get(&(chunk_x, chunk_z))
    }

    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId {
        let chunk_pos = (pos.x >> 4, pos.z >> 4);
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            chunk.get_block_state(pos)
        } else {
            BlockStateId::AIR
        }
    }

    fn get_fluid_and_fluid_state(&self, position: &Vector3<i32>) -> (Fluid, FluidState) {
        let id = GenerationCache::get_block_state(self, position);
        let Some(fluid) = Fluid::from_state_id(id) else {
            let fluid = if id.is_waterlogged() {
                Fluid::FLOWING_WATER
            } else {
                Fluid::EMPTY
            };
            return (fluid.clone(), fluid.states[0].clone());
        };
        (fluid.clone(), fluid.states[0].clone())
    }

    fn set_block_state(&mut self, pos: &Vector3<i32>, block_state: &BlockState) {
        let chunk_pos = (pos.x >> 4, pos.z >> 4);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_block_state(pos.x, pos.y, pos.z, block_state);
        }
    }

    fn add_block_entity(&mut self, pos: &Vector3<i32>, nbt: NbtCompound) {
        let chunk_pos = (pos.x >> 4, pos.z >> 4);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.add_block_entity(nbt);
        }
    }

    fn top_motion_blocking_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks
            .get(&chunk_pos)
            .map_or(0, |c| c.top_motion_blocking_block_height_exclusive(x, z))
    }

    fn top_motion_blocking_block_no_leaves_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks.get(&chunk_pos).map_or(0, |c| {
            c.top_motion_blocking_block_no_leaves_height_exclusive(x, z)
        })
    }

    fn get_top_y(&self, heightmap: &HeightMap, x: i32, z: i32) -> i32 {
        match heightmap {
            HeightMap::WorldSurfaceWg => self.top_block_wg_height_exclusive(x, z),
            HeightMap::WorldSurface => self.top_block_height_exclusive(x, z),
            HeightMap::OceanFloorWg => self.ocean_floor_wg_height_exclusive(x, z),
            HeightMap::OceanFloor => self.ocean_floor_height_exclusive(x, z),
            HeightMap::MotionBlocking => self.top_motion_blocking_block_height_exclusive(x, z),
            HeightMap::MotionBlockingNoLeaves => {
                self.top_motion_blocking_block_no_leaves_height_exclusive(x, z)
            }
        }
    }

    fn top_block_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks
            .get(&chunk_pos)
            .map_or(0, |c| c.top_block_height_exclusive(x, z))
    }

    fn ocean_floor_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks
            .get(&chunk_pos)
            .map_or(0, |c| c.ocean_floor_height_exclusive(x, z))
    }

    fn top_block_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks
            .get(&chunk_pos)
            .map_or(0, |c| c.top_block_wg_height_exclusive(x, z))
    }

    fn ocean_floor_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        let chunk_pos = (x >> 4, z >> 4);
        self.chunks
            .get(&chunk_pos)
            .map_or(0, |c| c.ocean_floor_wg_height_exclusive(x, z))
    }

    fn is_air(&self, local_pos: &Vector3<i32>) -> bool {
        let id = GenerationCache::get_block_state(self, local_pos);
        is_air(id)
    }

    fn get_biome_for_terrain_gen(&self, x: i32, y: i32, z: i32) -> &'static Biome {
        let center = self.get_center_chunk();
        let biome_pos = center.get_terrain_gen_biome_pos(x, y, z);
        let chunk_pos = (biome_pos.x >> 2, biome_pos.z >> 2);
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            chunk.get_biome(biome_pos.x, biome_pos.y, biome_pos.z)
        } else {
            center.get_biome(biome_pos.x, biome_pos.y, biome_pos.z)
        }
    }

    fn get_blending_data(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<&crate::generation::blender::blending_data::BlendingData> {
        self.chunks
            .get(&(chunk_x, chunk_z))
            .and_then(|c| c.blending_data.as_ref())
    }
}

/// Generate a batch of chunks, sharing Biomes and StructureStart computations
/// for overlapping 3×3 neighbor windows across concurrent worker threads.
///
/// When target_stage >= Features, this runs a coordinated multi-stage pipeline across
/// the full batch plus a 1-chunk boundary margin:
/// 1. Biomes & StructureStart precomputed in parallel
/// 2. StructureReferences computed in parallel
/// 3. Noise stage (terrain, stone, deepslate) computed in parallel
/// 4. Surface stage (grass, dirt, sand, ocean floor heightmaps) computed in parallel
/// 5. Carvers stage (caves, ravines) computed in parallel
/// 6. Features stage (ores, trees, vegetation) decorated with persistent cross-chunk writes
/// 7. Upgraded to Level chunks in parallel
pub fn generate_batch_chunks(
    generator: &WorldGenerator,
    block_registry: &dyn crate::world::WorldPortalExt,
    coords: &[(i32, i32)],
    target_stage: StagedChunkEnum,
) -> Vec<super::Chunk> {
    if coords.is_empty() {
        return Vec::new();
    }

    let stage_cache = Arc::new(StageCache::new());

    if (target_stage as u8) < (StagedChunkEnum::Features as u8)
        || matches!(generator, WorldGenerator::Custom(_))
    {
        return coords
            .par_iter()
            .map(|&(cx, cz)| {
                generate_single_chunk_batched(
                    generator,
                    block_registry,
                    cx,
                    cz,
                    target_stage,
                    &stage_cache,
                )
            })
            .collect();
    }

    // 1. Gather all unique coordinates needed (all requested chunks + 1-chunk border for 3x3 write radius)
    let mut all_coords = FxHashSet::default();
    for &(cx, cz) in coords {
        for dx in -1..=1 {
            for dz in -1..=1 {
                all_coords.insert((cx + dx, cz + dz));
            }
        }
    }
    let all_coords_vec: Vec<(i32, i32)> = all_coords.into_iter().collect();

    // 2. Precompute Biomes, StructureStarts, and StructureReferences in parallel
    let mut chunks_vec: Vec<ProtoChunk> = all_coords_vec
        .par_iter()
        .map(|&(cx, cz)| {
            let cached = stage_cache.get_or_compute_structure_start(cx, cz, generator);
            let mut chunk = ProtoChunk::new_raw(cx, cz, &cached);
            chunk.stage = StagedChunkEnum::StructureStart;
            chunk.copy_structure_starts_from(&cached);
            match generator {
                WorldGenerator::Noise(noise_gen) => chunk.set_structure_references(noise_gen),
                WorldGenerator::Flat(_) => chunk.stage = StagedChunkEnum::StructureReferences,
                WorldGenerator::Custom(custom_gen) => custom_gen.set_structure_references(&mut chunk),
            }
            chunk
        })
        .collect();

    // 3. Noise stage in parallel across all chunks
    chunks_vec.par_iter_mut().for_each(|chunk| {
        match generator {
            WorldGenerator::Noise(noise_gen) => chunk.step_to_noise(noise_gen),
            WorldGenerator::Flat(flat_gen) => flat_gen.step_to_noise(chunk),
            WorldGenerator::Custom(custom_gen) => custom_gen.step_to_noise(chunk),
        }
    });

    // 4. Surface stage — share biome palettes across 3×3 neighborhoods via Arc so
    // each chunk's flat_biome_map is copied once, not up to 9× via StageCache lookups.
    use super::generation_cache::{SurfaceBiomeNeighborhood, SurfaceBiomePalette};

    let shared_biome_palettes: FxHashMap<(i32, i32), SurfaceBiomePalette> = chunks_vec
        .iter()
        .map(|chunk| {
            (
                (chunk.x, chunk.z),
                SurfaceBiomePalette {
                    chunk_x: chunk.x,
                    chunk_z: chunk.z,
                    bottom_quart_y: biome_coords::from_block(chunk.bottom_y() as i32),
                    height_quarts: chunk.height() as usize >> 2,
                    biome_mask: chunk.biome_mask,
                    biomes: Arc::from(chunk.flat_biome_map.as_ref()),
                },
            )
        })
        .collect();

    let surface_neighborhoods: Vec<SurfaceBiomeNeighborhood> = chunks_vec
        .iter()
        .map(|chunk| {
            let mut neighborhood = SurfaceBiomeNeighborhood::new(chunk.x, chunk.z);
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if let Some(palette) =
                        shared_biome_palettes.get(&(chunk.x + dx, chunk.z + dz))
                    {
                        neighborhood.push_palette(palette);
                    }
                }
            }
            neighborhood
        })
        .collect();

    chunks_vec
        .par_iter_mut()
        .zip(surface_neighborhoods)
        .for_each(|(chunk, neighborhood)| {
            match generator {
                WorldGenerator::Noise(noise_gen) => {
                    chunk.step_to_surface(noise_gen, &neighborhood);
                }
                WorldGenerator::Flat(flat_gen) => flat_gen.step_to_surface(chunk),
                WorldGenerator::Custom(custom_gen) => custom_gen.step_to_surface(chunk),
            }
        });

    // 5. Carvers stage in parallel across all chunks
    chunks_vec.par_iter_mut().for_each(|chunk| {
        match generator {
            WorldGenerator::Noise(noise_gen) => chunk.step_to_carvers(noise_gen),
            WorldGenerator::Flat(flat_gen) => flat_gen.step_to_carvers(chunk),
            WorldGenerator::Custom(custom_gen) => custom_gen.step_to_carvers(chunk),
        }
    });

    // 6. Assemble into a grid map for feature decoration
    let mut grid: FxHashMap<(i32, i32), ProtoChunk> = chunks_vec
        .into_iter()
        .map(|chunk| ((chunk.x, chunk.z), chunk))
        .collect();

    // 7. Features stage: decorate each chunk in coords sequentially, writing directly into grid
    for &(cx, cz) in coords {
        let mut cache = BatchGridCache {
            center_x: cx,
            center_z: cz,
            chunks: &mut grid,
        };
        match generator {
            WorldGenerator::Noise(noise_gen) => {
                ProtoChunk::generate_features_and_structure(
                    &mut cache,
                    block_registry,
                    &noise_gen.random_config,
                );
            }
            WorldGenerator::Flat(_) => {
                cache.get_center_chunk_mut().stage = StagedChunkEnum::Features;
            }
            WorldGenerator::Custom(_) => unreachable!("Custom generator handled earlier"),
        }
    }

    // 8. Extract requested chunks in exact original order
    let mut result: Vec<super::Chunk> = coords
        .iter()
        .map(|&(cx, cz)| {
            let chunk = grid.remove(&(cx, cz)).expect("requested chunk must exist in grid");
            super::Chunk::Proto(Box::new(chunk))
        })
        .collect();

    // 9. If target stage is Full, upgrade chunks in parallel
    if target_stage == StagedChunkEnum::Full {
        result.par_iter_mut().for_each(|chunk_enum| {
            chunk_enum.upgrade_to_level_chunk(
                generator.dimension(),
                &LightingEngineConfig::Default,
            );
        });
    }

    result
}

pub fn generate_single_chunk_batched(
    generator: &WorldGenerator,
    block_registry: &dyn crate::world::WorldPortalExt,
    chunk_x: i32,
    chunk_z: i32,
    target_stage: StagedChunkEnum,
    stage_cache: &StageCache,
) -> super::Chunk {
    use super::{Cache, StagedChunkEnum as SE};
    use crate::ProtoChunk;
    use pumpkin_data::dimension::Dimension;

    // When the shared stage work (Biomes + StructureStart) is cheap (The End ~39µs, Nether ~467µs),
    // DashMap hashing, atomic OnceLock synchronization, and ProtoChunk buffer copying overhead
    // exceed recomputation savings. Overworld has >8ms of shared work where caching provides massive speedup.
    // For End and Nether, bypass directly to generate_single_chunk to avoid regression.
    if *generator.dimension() == Dimension::THE_END || *generator.dimension() == Dimension::THE_NETHER {
        return super::generation::generate_single_chunk(
            generator,
            block_registry,
            chunk_x,
            chunk_z,
            target_stage,
        );
    }

    // Surface stage's prepare_surface_biomes() requires cache.size >= 3 (radius >= 1).
    // If the target stage has a smaller direct_radius (e.g. Carvers = 0), the cache
    // would be 1x1 and prepare_surface_biomes would early-return, leaving surface_biomes
    // as None and panicking when advance(Surface) tries to take() it.
    // Fix: ensure radius is at least SE::Surface.get_direct_radius() (= 1) whenever
    // Surface will run as an intermediate stage on the way to target_stage.
    let min_radius = if target_stage as u8 >= SE::Surface as u8 {
        SE::Surface.get_direct_radius()
    } else {
        0
    };
    let radius = target_stage.get_direct_radius().max(min_radius);
    let mut cache = Cache::new(chunk_x - radius, chunk_z - radius, radius * 2 + 1);

    // ── Populate the 3×3 (or larger) neighbor window ────────────────────────
    // For Biomes and StructureStart, pull from the shared StageCache instead of
    // recomputing from scratch.
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let nx = chunk_x + dx;
            let nz = chunk_z + dz;

            let proto = if matches!(target_stage as u8, x if x >= SE::StructureStart as u8) {
                let cached = stage_cache.get_or_compute_structure_start(nx, nz, generator);
                let mut chunk = ProtoChunk::new_raw(nx, nz, &cached);
                chunk.stage = SE::StructureStart;
                chunk.copy_structure_starts_from(&cached);
                Box::new(chunk)
            } else if matches!(target_stage as u8, x if x >= SE::Biomes as u8) {
                let cached = stage_cache.get_or_compute_biomes(nx, nz, generator);
                let mut chunk = ProtoChunk::new_raw(nx, nz, &cached);
                chunk.stage = SE::Biomes;
                Box::new(chunk)
            } else {
                Box::new(ProtoChunk::new(nx, nz, generator))
            };

            cache.chunks.push(super::Chunk::Proto(proto));
        }
    }

    // ── Advance remaining stages (StructureReferences, Noise, …) ────────────
    let stages = [
        SE::Biomes,
        SE::StructureStart,
        SE::StructureReferences,
        SE::Noise,
        SE::Surface,
        SE::Carvers,
        SE::Features,
        SE::Lighting,
        SE::Spawn,
        SE::Full,
    ];

    for &stage in &stages {
        if stage as u8 > target_stage as u8 {
            break;
        }
        // Biomes and StructureStart are already populated from cache above;
        // skip advance_all for those stages — the chunks are already at the
        // right stage level.
        if matches!(
            stage,
            SE::Biomes | SE::StructureStart
        ) {
            continue;
        }

        cache.advance(stage, generator, block_registry, &LightingEngineConfig::Default);
    }

    let mid = ((cache.size * cache.size) >> 1) as usize;
    cache.chunks.swap_remove(mid)
}
