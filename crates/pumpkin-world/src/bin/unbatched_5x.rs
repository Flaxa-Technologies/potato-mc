use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use pumpkin_data::dimension::Dimension;
use pumpkin_data::BlockStateId;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum};
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

fn run_single_unbatched_benchmark(grid_size: i32, threads: usize) -> (f64, f64, f64, f64) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .stack_size(16 * 1024 * 1024)
        .build()
        .expect("Failed to build rayon pool");

    let seed = Seed(12345);
    let world_gen = Arc::new(get_world_gen(
        seed,
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    ));
    let block_registry = Arc::new(BenchmarkBlockRegistry);

    let total_chunks = (grid_size * grid_size) as usize;
    let worst_time_us = Arc::new(AtomicU64::new(0));
    let best_time_us = Arc::new(AtomicU64::new(u64::MAX));
    let sum_time_us = Arc::new(AtomicU64::new(0));

    JIGSAW_CONCURRENT_MISSES.store(0, Ordering::Relaxed);

    let overall_start = Instant::now();
    let chunk_times_us = Arc::new(std::sync::Mutex::new(Vec::with_capacity(total_chunks)));

    pool.scope(|s| {
        for cx in 0..grid_size {
            for cz in 0..grid_size {
                let wg = world_gen.clone();
                let br = block_registry.clone();
                let worst = worst_time_us.clone();
                let best = best_time_us.clone();
                let sum = sum_time_us.clone();
                let times = chunk_times_us.clone();

                s.spawn(move |_| {
                    let c_start = Instant::now();
                    let chunk = black_box(generate_single_chunk(
                        &wg,
                        br.as_ref(),
                        cx,
                        cz,
                        StagedChunkEnum::Full,
                    ));
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
    let avg_chunk_ms = (sum_time_us.load(Ordering::Relaxed) as f64 / total_chunks as f64) / 1000.0;
    let worst_chunk_ms = (worst_time_us.load(Ordering::Relaxed) as f64) / 1000.0;

    (chunks_per_sec, avg_chunk_ms, worst_chunk_ms, total_elapsed.as_secs_f64())
}

fn main() {
    let handle = std::thread::Builder::new()
        .name("bench-runner-5x".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
            let grid_size = 32; // 32x32 = 1,024 chunks
            let total_chunks = grid_size * grid_size;

            println!("=== UNBATCHED OVERWORLD BASELINE 5X BACK-TO-BACK BENCHMARK ===");
            println!("Configuration: 32x32 ({} chunks), Threads: {}", total_chunks, num_threads);
            println!("Running 5 consecutive runs in a single session with no rebuild between runs...\n");

            let mut results = Vec::new();

            for run in 1..=5 {
                let (cps, avg_ms, worst_ms, total_s) = run_single_unbatched_benchmark(grid_size, num_threads);
                println!(
                    "RUN {}: {:.2} c/s | Total: {:.3}s | Avg: {:.2}ms | Worst: {:.2}ms",
                    run, cps, total_s, avg_ms, worst_ms
                );
                results.push(cps);
            }

            let mean = results.iter().sum::<f64>() / results.len() as f64;
            let variance = results.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / results.len() as f64;
            let std_dev = variance.sqrt();

            println!("\n=== 5X SUMMARY RAW RESULTS ===");
            for (i, v) in results.iter().enumerate() {
                println!("Run {}: {:.2} c/s", i + 1, v);
            }
            println!("Mean: {:.2} c/s", mean);
            println!("StdDev: {:.2} c/s", std_dev);
            println!("Min: {:.2} c/s | Max: {:.2} c/s", results.iter().cloned().fold(f64::INFINITY, f64::min), results.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            println!("===============================");
        })
        .expect("Failed to spawn thread");
    handle.join().expect("Thread panicked");
}
