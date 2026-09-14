use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use pumpkin_data::dimension::Dimension;
use pumpkin_data::BlockStateId;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::chunk_system::{
    generate_single_chunk, generate_single_chunk_batched, StageCache, StagedChunkEnum,
};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::generation::structure::placement::JIGSAW_CONCURRENT_MISSES;
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};

struct BenchmarkBlockRegistry;

impl WorldPortalExt for BenchmarkBlockRegistry {
    fn can_place_at(
        &self,
        _block: &pumpkin_data::Block,
        _state: &pumpkin_data::BlockState,
        _block_accessor: &dyn BlockAccessor,
        _block_pos: &pumpkin_util::math::position::BlockPos,
    ) -> bool {
        true
    }

    fn mirror(
        &self,
        block: &pumpkin_data::Block,
        state_id: BlockStateId,
        mirror: pumpkin_data::Mirror,
    ) -> &'static pumpkin_data::BlockState {
        block.mirror(state_id, mirror)
    }

    fn rotate(
        &self,
        block: &pumpkin_data::Block,
        state_id: BlockStateId,
        rotation: pumpkin_data::Rotation,
    ) -> &'static pumpkin_data::BlockState {
        block.rotate(state_id, rotation)
    }

    fn spawn_mobs_for_chunk_generation(
        &self,
        _cache: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache,
        _biome: &'static pumpkin_data::chunk::Biome,
        _chunk_x: i32,
        _chunk_z: i32,
    ) {
    }
}

fn benchmark_dimension(
    dimension: Dimension,
    name: &str,
    grid_size: i32,
    threads: usize,
    use_batched: bool,
) -> (f64, f64, f64, f64) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(16 * 1024 * 1024)
        .build()
        .expect("Failed to build rayon pool");

    let seed = Seed(12345);
    let world_gen = Arc::new(get_world_gen(
        seed,
        dimension,
        false,
        Vec::new(),
        String::new(),
    ));
    let block_registry = Arc::new(BenchmarkBlockRegistry);

    let total_chunks = (grid_size * grid_size) as usize;
    let worst_time_us = Arc::new(AtomicU64::new(0));
    let best_time_us = Arc::new(AtomicU64::new(u64::MAX));
    let sum_time_us = Arc::new(AtomicU64::new(0));

    // Reset thundering-herd counter before this run so we get a clean per-run count.
    JIGSAW_CONCURRENT_MISSES.store(0, Ordering::Relaxed);

    let overall_start = Instant::now();

    let chunk_times_us = Arc::new(std::sync::Mutex::new(Vec::with_capacity(total_chunks)));
    let stage_cache = Arc::new(StageCache::new());

    pool.scope(|s| {
        for cx in 0..grid_size {
            for cz in 0..grid_size {
                let wg = world_gen.clone();
                let br = block_registry.clone();
                let worst = worst_time_us.clone();
                let best = best_time_us.clone();
                let sum = sum_time_us.clone();
                let times = chunk_times_us.clone();
                let sc = stage_cache.clone();

                s.spawn(move |_| {
                    let c_start = Instant::now();
                    let chunk = if use_batched {
                        black_box(generate_single_chunk_batched(
                            &wg,
                            br.as_ref(),
                            cx,
                            cz,
                            StagedChunkEnum::Full,
                            &sc,
                        ))
                    } else {
                        black_box(generate_single_chunk(
                            &wg,
                            br.as_ref(),
                            cx,
                            cz,
                            StagedChunkEnum::Full,
                        ))
                    };
                    black_box(&chunk);
                    let elapsed_us = c_start.elapsed().as_micros() as u64;

                    worst.fetch_max(elapsed_us, Ordering::Relaxed);
                    best.fetch_min(elapsed_us, Ordering::Relaxed);
                    sum.fetch_add(elapsed_us, Ordering::Relaxed);
                    times.lock().unwrap().push(elapsed_us);
                });
            }
        }
    });

    let total_elapsed = overall_start.elapsed();
    let chunks_per_sec = total_chunks as f64 / total_elapsed.as_secs_f64();
    let chunks_per_cpu_sec = chunks_per_sec / threads as f64;
    let avg_chunk_ms = (sum_time_us.load(Ordering::Relaxed) as f64 / total_chunks as f64) / 1000.0;
    let worst_chunk_ms = (worst_time_us.load(Ordering::Relaxed) as f64) / 1000.0;
    let best_chunk_ms = (best_time_us.load(Ordering::Relaxed) as f64) / 1000.0;

    let mut times = chunk_times_us.lock().unwrap().clone();
    times.sort_unstable();
    let median_chunk_ms = if times.is_empty() { 0.0 } else { times[times.len() / 2] as f64 / 1000.0 };

    let concurrent_misses = JIGSAW_CONCURRENT_MISSES.load(Ordering::Relaxed);

    let mode_str = if use_batched { "Batched StageCache" } else { "Unbatched Baseline" };
    println!("==================================================");
    println!("DIMENSION: {} ({})", name, mode_str);
    println!("Grid: {}x{} ({} chunks), Rayon Threads: {}", grid_size, grid_size, total_chunks, threads);
    println!("Total Time: {:.3} s", total_elapsed.as_secs_f64());
    println!("Throughput: {:.2} chunks/sec", chunks_per_sec);
    println!("Normalized: {:.2} chunks/CPU-sec", chunks_per_cpu_sec);
    println!("Median Chunk Time: {:.2} ms", median_chunk_ms);
    println!("Average Chunk Time: {:.2} ms", avg_chunk_ms);
    println!("Worst Chunk Time: {:.2} ms", worst_chunk_ms);
    println!("Best Chunk Time: {:.2} ms", best_chunk_ms);
    println!("Estimated MSPT under load: {:.2} ms", 1000.0 / chunks_per_sec.max(1.0));
    println!("Jigsaw single-flight saves: {} concurrent misses prevented", concurrent_misses);
    println!("==================================================");

    (chunks_per_sec, avg_chunk_ms, worst_chunk_ms, total_elapsed.as_secs_f64())
}

fn profile_stages(dimension: Dimension, name: &str) {
    let seed = Seed(12345);
    let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());
    let block_registry = BenchmarkBlockRegistry;

    let radius = 1;
    let chunk_x = 0;
    let chunk_z = 0;

    let t0 = Instant::now();
    let mut cache = pumpkin_world::chunk_system::Cache::new(chunk_x - radius, chunk_z - radius, radius * 2 + 1);
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let proto = Box::new(pumpkin_world::ProtoChunk::new(chunk_x + dx, chunk_z + dz, &world_gen));
            cache.chunks.push(pumpkin_world::chunk_system::Chunk::Proto(proto));
        }
    }
    let t_setup = t0.elapsed();

    let t1 = Instant::now();
    cache.advance_all(StagedChunkEnum::Biomes, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_biomes = t1.elapsed();

    let t2 = Instant::now();
    cache.advance_all(StagedChunkEnum::StructureStart, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_struct_start = t2.elapsed();

    let t3 = Instant::now();
    pumpkin_world::generation::proto_chunk::enable_structure_ref_profiling(true);
    cache.advance_all(StagedChunkEnum::StructureReferences, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    pumpkin_world::generation::proto_chunk::enable_structure_ref_profiling(false);
    let t_struct_ref = t3.elapsed();
    pumpkin_world::generation::proto_chunk::dump_and_reset_structure_ref_profiling(name);

    let t4 = Instant::now();
    cache.advance(StagedChunkEnum::Noise, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_noise = t4.elapsed();

    let t5 = Instant::now();
    cache.advance(StagedChunkEnum::Surface, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_surface = t5.elapsed();

    let t6 = Instant::now();
    cache.advance(StagedChunkEnum::Carvers, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_carvers = t6.elapsed();

    let t7 = Instant::now();
    cache.advance(StagedChunkEnum::Features, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_features = t7.elapsed();

    let t8 = Instant::now();
    cache.advance(StagedChunkEnum::Lighting, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_lighting = t8.elapsed();

    let t9 = Instant::now();
    cache.advance(StagedChunkEnum::Spawn, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_spawn = t9.elapsed();

    let t10 = Instant::now();
    cache.advance(StagedChunkEnum::Full, &world_gen, &block_registry, &pumpkin_config::lighting::LightingEngineConfig::Default);
    let t_full = t10.elapsed();

    println!("\n>>> [{}] Single Chunk Stage Profiling <<<", name);
    println!("  Cache Setup (3x3):         {:?}", t_setup);
    println!("  Biomes (advance_all 3x3):  {:?}", t_biomes);
    println!("  StructureStart (3x3):      {:?}", t_struct_start);
    println!("  StructureReferences (3x3): {:?}", t_struct_ref);
    println!("  Noise (center chunk):      {:?}", t_noise);
    println!("  Surface (center chunk):    {:?}", t_surface);
    println!("  Carvers (center chunk):    {:?}", t_carvers);
    println!("  Features (center chunk):   {:?}", t_features);
    println!("  Lighting (center chunk):   {:?}", t_lighting);
    println!("  Spawn (center chunk):      {:?}", t_spawn);
    println!("  Full (center chunk):       {:?}", t_full);
    println!("  Total Single Chunk:        {:?}\n", t0.elapsed());
}

fn main() {
    let handle = std::thread::Builder::new()
        .name("bench-runner".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let num_threads = std::env::args()
                .nth(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8));

            let grid_size = std::env::args()
                .nth(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(32);

            println!("=============================================================");
            println!("  POTATOMC HIGH-PERFORMANCE WORLDGEN BENCHMARK");
            println!("  Grid: {}x{} ({} chunks) | Rayon Worker Threads: {}", grid_size, grid_size, grid_size * grid_size, num_threads);
            println!("  Usage: ./worldgen-bench-linux-x86_64 [grid_size] [threads] [--exit]");
            println!("=============================================================");

            profile_stages(Dimension::OVERWORLD, "Overworld");
            profile_stages(Dimension::THE_NETHER, "The Nether");
            profile_stages(Dimension::THE_END, "The End");

            println!(">>> Benchmarking OVERWORLD ({}x{}) - Unbatched Baseline <<<", grid_size, grid_size);
            let (ow_cps_base, ow_avg_base, ow_worst_base, ow_total_base) =
                benchmark_dimension(Dimension::OVERWORLD, "Overworld", grid_size, num_threads, false);

            println!("\n>>> Benchmarking OVERWORLD ({}x{}) - Batched StageCache <<<", grid_size, grid_size);
            let (ow_cps_batch, ow_avg_batch, ow_worst_batch, ow_total_batch) =
                benchmark_dimension(Dimension::OVERWORLD, "Overworld", grid_size, num_threads, true);

            println!("\n>>> Benchmarking NETHER ({}x{}) - Unbatched Baseline <<<", grid_size, grid_size);
            let (nether_cps_base, nether_avg_base, nether_worst_base, nether_total_base) =
                benchmark_dimension(Dimension::THE_NETHER, "The Nether", grid_size, num_threads, false);

            println!("\n>>> Benchmarking NETHER ({}x{}) - Batched StageCache <<<", grid_size, grid_size);
            let (nether_cps_batch, nether_avg_batch, nether_worst_batch, nether_total_batch) =
                benchmark_dimension(Dimension::THE_NETHER, "The Nether", grid_size, num_threads, true);

            println!("\n>>> Benchmarking THE END ({}x{}) - Unbatched Baseline <<<", grid_size, grid_size);
            let (end_cps_base, end_avg_base, end_worst_base, end_total_base) =
                benchmark_dimension(Dimension::THE_END, "The End", grid_size, num_threads, false);

            println!("\n>>> Benchmarking THE END ({}x{}) - Batched StageCache <<<", grid_size, grid_size);
            let (end_cps_batch, end_avg_batch, end_worst_batch, end_total_batch) =
                benchmark_dimension(Dimension::THE_END, "The End", grid_size, num_threads, true);

            println!("\n=================== FINAL SUMMARY RESULTS ===================");
            println!("Overworld (Unbatched Baseline): {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", ow_cps_base, ow_avg_base, ow_worst_base, ow_total_base);
            println!("Overworld (Batched StageCache): {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", ow_cps_batch, ow_avg_batch, ow_worst_batch, ow_total_batch);
            let ow_speedup = (ow_cps_batch / ow_cps_base - 1.0) * 100.0;
            println!("Overworld Speedup:              {:+5.1}%", ow_speedup);
            println!("-------------------------------------------------------------");
            println!("Nether (Unbatched Baseline):    {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", nether_cps_base, nether_avg_base, nether_worst_base, nether_total_base);
            println!("Nether (Batched StageCache):    {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", nether_cps_batch, nether_avg_batch, nether_worst_batch, nether_total_batch);
            let nether_speedup = (nether_cps_batch / nether_cps_base - 1.0) * 100.0;
            println!("Nether Speedup:                 {:+5.1}%", nether_speedup);
            println!("-------------------------------------------------------------");
            println!("The End (Unbatched Baseline):   {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", end_cps_base, end_avg_base, end_worst_base, end_total_base);
            println!("The End (Batched StageCache):   {:.2} c/s | avg: {:.2}ms | worst: {:.2}ms | total: {:.2}s", end_cps_batch, end_avg_batch, end_worst_batch, end_total_batch);
            let end_speedup = (end_cps_batch / end_cps_base - 1.0) * 100.0;
            println!("The End Speedup:                {:+5.1}%", end_speedup);
            println!("=============================================================");

            let should_exit = std::env::args().any(|arg| arg == "--exit");
            if !should_exit {
                println!("\n[INFO] Benchmark complete. Process will remain alive so you can inspect results in console.");
                println!("[INFO] When finished, click 'Stop' in your panel or press Ctrl+C.");
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            }
        })
        .expect("Failed to spawn bench runner thread");
    handle.join().expect("Bench runner thread panicked");
}
