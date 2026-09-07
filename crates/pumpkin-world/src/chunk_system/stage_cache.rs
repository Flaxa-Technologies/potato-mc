use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use crate::ProtoChunk;
use crate::chunk_system::StagedChunkEnum;
use crate::generation::generator::WorldGenerator;
use pumpkin_config::lighting::LightingEngineConfig;

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
    biomes: DashMap<(i32, i32), Arc<OnceLock<Arc<ProtoChunk>>>>,
    structure_start: DashMap<(i32, i32), Arc<OnceLock<Arc<ProtoChunk>>>>,
}

impl StageCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            biomes: DashMap::new(),
            structure_start: DashMap::new(),
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
        let slot_arc = self
            .biomes
            .entry((cx, cz))
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone();

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
    /// Internally calls `get_or_compute_biomes` to ensure the biome stage is done first.
    pub fn get_or_compute_structure_start(
        &self,
        cx: i32,
        cz: i32,
        generator: &WorldGenerator,
    ) -> Arc<ProtoChunk> {
        let slot_arc = self
            .structure_start
            .entry((cx, cz))
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone();

        let result = slot_arc.get_or_init(|| {
            let biome_arc = self.get_or_compute_biomes(cx, cz, generator);
            let mut chunk = clone_at_biome_stage(&biome_arc);

            match generator {
                WorldGenerator::Noise(noise_gen) => chunk.set_structure_starts(noise_gen),
                WorldGenerator::Flat(_) => {
                    chunk.stage = StagedChunkEnum::StructureStart;
                }
                WorldGenerator::Custom(custom_gen) => {
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
    cloned.flat_biome_map.clone_from(&chunk.flat_biome_map);
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
pub fn generate_batch_chunks(
    generator: &WorldGenerator,
    block_registry: &dyn crate::world::WorldPortalExt,
    coords: &[(i32, i32)],
    target_stage: StagedChunkEnum,
) -> Vec<super::Chunk> {
    use rayon::prelude::*;

    let stage_cache = Arc::new(StageCache::new());

    coords
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
        .collect()
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

    let radius = target_stage.get_direct_radius();
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
                chunk.flat_biome_map.clone_from(&cached.flat_biome_map);
                chunk.stage = SE::StructureStart;
                chunk.copy_structure_starts_from(&cached);
                Box::new(chunk)
            } else if matches!(target_stage as u8, x if x >= SE::Biomes as u8) {
                let cached = stage_cache.get_or_compute_biomes(nx, nz, generator);
                let mut chunk = ProtoChunk::new_raw(nx, nz, &cached);
                chunk.flat_biome_map.clone_from(&cached.flat_biome_map);
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

        if stage == SE::StructureReferences {
            cache.advance_all(stage, generator, block_registry, &LightingEngineConfig::Default);
        } else {
            cache.advance(stage, generator, block_registry, &LightingEngineConfig::Default);
        }
    }

    let mid = ((cache.size * cache.size) >> 1) as usize;
    cache.chunks.swap_remove(mid)
}
