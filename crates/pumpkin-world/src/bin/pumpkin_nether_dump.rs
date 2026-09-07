use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use pumpkin_data::block_properties::is_air;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::{Block, BlockStateId, chunk::Biome};
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::chunk::ChunkData;
use pumpkin_world::chunk_system::{Chunk, StagedChunkEnum, generate_single_chunk};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
use serde::Serialize;

struct MockBlockRegistry;

impl WorldPortalExt for MockBlockRegistry {
    fn can_place_at(
        &self,
        _block: &Block,
        _state: &pumpkin_data::BlockState,
        _block_accessor: &dyn BlockAccessor,
        _block_pos: &BlockPos,
    ) -> bool {
        true
    }

    fn mirror(
        &self,
        block: &Block,
        state_id: BlockStateId,
        mirror: pumpkin_data::Mirror,
    ) -> &'static pumpkin_data::BlockState {
        block.mirror(state_id, mirror)
    }

    fn rotate(
        &self,
        block: &Block,
        state_id: BlockStateId,
        rotation: pumpkin_data::Rotation,
    ) -> &'static pumpkin_data::BlockState {
        block.rotate(state_id, rotation)
    }

    fn spawn_mobs_for_chunk_generation(
        &self,
        _cache: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache,
        _biome: &'static Biome,
        _chunk_x: i32,
        _chunk_z: i32,
    ) {
    }
}

#[derive(Serialize)]
struct ChunkDumpOutput {
    seed: i64,
    dimension: String,
    chunk: [i32; 2],
    status: String,
    biomes: HashMap<String, f64>,
    biomes_counts: HashMap<String, u32>,
    height: HeightData,
    blocks: HashMap<String, u32>,
    lava: u32,
    air: u32,
    nether_specific: HashMap<String, u32>,
    structures: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct HeightData {
    highest_solid_y: i32,
    lowest_solid_y: i32,
    columns_surface: HashMap<String, i32>,
    columns_lava: HashMap<String, i32>,
}

fn analyze_chunk(
    chunk_data: &ChunkData,
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
) -> ChunkDumpOutput {
    let mut blocks_hist: HashMap<String, u32> = HashMap::new();
    let mut biomes_hist: HashMap<String, u32> = HashMap::new();
    let mut lava_count = 0u32;
    let mut air_count = 0u32;

    let sections = &chunk_data.section;
    let block_sections = sections.block_sections.read().unwrap();
    let biome_sections = sections.biome_sections.read().unwrap();

    // 3D block representation [y][z][x]
    let mut block_matrix = HashMap::new(); // (lx, y, lz) -> block_name

    let section_count = block_sections.len().min(16);

    for (sec_idx, block_palette) in block_sections.iter().enumerate().take(section_count) {
        let sec_y = sec_idx as i32;

        // Count biomes
        if sec_idx < biome_sections.len() {
            let biome_palette = &biome_sections[sec_idx];
            for b_id in biome_palette.iter() {
                let name = pumpkin_data::biome::Biome::from_id(b_id)
                    .map_or("plains", |b| b.registry_id);
                let b_name = if name.starts_with("minecraft:") {
                    name.to_string()
                } else {
                    format!("minecraft:{name}")
                };
                *biomes_hist.entry(b_name).or_insert(0) += 1;
            }
        }

        // Count blocks and record 3D positions
        for ly in 0..16usize {
            let gy = (sec_y * 16) + (ly as i32);
            for lz in 0..16usize {
                for lx in 0..16usize {
                    let state_id = block_palette.get(lx, ly, lz);
                    let block = Block::from_state_id(state_id);
                    let b_name = format!("minecraft:{}", block.name);

                    *blocks_hist.entry(b_name.clone()).or_insert(0) += 1;
                    if b_name.contains("lava") {
                        lava_count += 1;
                    }
                    if is_air(state_id) || b_name.contains("air") {
                        air_count += 1;
                    }

                    block_matrix.insert((lx, gy, lz), b_name);
                }
            }
        }
    }

    // Compute biome percentages
    let total_biomes: u32 = biomes_hist.values().sum();
    let mut biomes_pct = HashMap::new();
    if total_biomes > 0 {
        for (k, v) in &biomes_hist {
            let pct = ((*v as f64) / (total_biomes as f64) * 10000.0).round() / 100.0;
            biomes_pct.insert(k.clone(), pct);
        }
    }

    let mut nether_specific = HashMap::new();
    let keys = [
        "netherrack",
        "soul_sand",
        "soul_soil",
        "basalt",
        "blackstone",
        "crimson_nylium",
        "warped_nylium",
        "glowstone",
        "nether_quartz_ore",
        "nether_gold_ore",
        "ancient_debris",
        "nether_bricks",
        "bone_block",
        "magma_block",
    ];
    for key in keys {
        let full_name = format!("minecraft:{key}");
        let count = blocks_hist.get(&full_name).copied().unwrap_or(0);
        nether_specific.insert(key.to_string(), count);
    }

    let mut columns_surface = HashMap::new();
    let mut columns_lava = HashMap::new();
    let mut highest_solid_y = -1;
    let mut lowest_solid_y = 256;

    for lx in 0..16 {
        for lz in 0..16 {
            let mut found_air_below_ceiling = false;
            let mut cavern_floor_y = 0;
            let mut lava_y = 0;

            for gy in (0..=127).rev() {
                let b = block_matrix
                    .get(&(lx, gy, lz))
                    .map_or("minecraft:air", |s| s.as_str());
                let is_solid = !b.contains("air");
                if is_solid {
                    highest_solid_y = highest_solid_y.max(gy);
                    lowest_solid_y = lowest_solid_y.min(gy);
                    if found_air_below_ceiling && cavern_floor_y == 0 {
                        cavern_floor_y = gy;
                    }
                } else if gy < 125 {
                    found_air_below_ceiling = true;
                }

                if b.contains("lava") && lava_y == 0 {
                    lava_y = gy;
                }
            }

            let col_key = format!("{lx},{lz}");
            let surf = if cavern_floor_y > 0 {
                cavern_floor_y
            } else if lava_y > 0 {
                lava_y
            } else {
                31
            };
            columns_surface.insert(col_key.clone(), surf);
            columns_lava.insert(col_key, lava_y);
        }
    }

    ChunkDumpOutput {
        seed,
        dimension: "minecraft:the_nether".to_string(),
        chunk: [chunk_x, chunk_z],
        status: "minecraft:full".to_string(),
        biomes: biomes_pct,
        biomes_counts: biomes_hist,
        height: HeightData {
            highest_solid_y,
            lowest_solid_y: if lowest_solid_y != 256 {
                lowest_solid_y
            } else {
                0
            },
            columns_surface,
            columns_lava,
        },
        blocks: blocks_hist,
        lava: lava_count,
        air: air_count,
        nether_specific,
        structures: HashMap::new(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut seed = 12345i64;
    let mut chunk_coords = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (10, 10),
        (-10, -10),
        (25, -25),
    ];
    let mut out_dir = "nether-parity/pumpkin".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                if i + 1 < args.len() {
                    seed = args[i + 1].parse().unwrap_or(12345);
                    i += 1;
                }
            }
            "--out-dir" => {
                if i + 1 < args.len() {
                    out_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            "--chunks" => {
                chunk_coords.clear();
                i += 1;
                while i < args.len() && !args[i].starts_with("--") {
                    let parts: Vec<&str> = args[i].split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(x), Ok(z)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                            chunk_coords.push((x, z));
                        }
                    }
                    i += 1;
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    fs::create_dir_all(&out_dir).expect("Failed to create out_dir");

    println!(
        "[PUMPKIN-DUMP] Generating {} Nether chunks for seed {}...",
        chunk_coords.len(),
        seed
    );

    let builder = std::thread::Builder::new().stack_size(16 * 1024 * 1024);
    let handler = builder
        .spawn(move || {
            let world_gen = get_world_gen(
                Seed(seed as u64),
                Dimension::THE_NETHER,
                false,
                Vec::new(),
                String::new(),
            );

            for (cx, cz) in chunk_coords {
                print!("[PUMPKIN-DUMP] Generating chunk ({cx}, {cz})... ");
                let chunk = generate_single_chunk(
                    &world_gen,
                    &MockBlockRegistry,
                    cx,
                    cz,
                    StagedChunkEnum::Full,
                );

                let Chunk::Level(chunk_level) = chunk else {
                    panic!("generate_single_chunk returned proto chunk instead of level chunk");
                };

                let dump = analyze_chunk(&chunk_level, seed, cx, cz);
                let out_file = Path::new(&out_dir).join(format!("chunk_{cx}_{cz}.json"));
                let json_str = serde_json::to_string_pretty(&dump).expect("Serialize error");
                let mut file = File::create(&out_file).expect("File create error");
                file.write_all(json_str.as_bytes()).expect("Write error");

                println!("saved to {}", out_file.display());
            }

            println!("[PUMPKIN-DUMP] All requested chunks dumped successfully.");
        })
        .expect("Failed to spawn generator thread");

    handler.join().expect("Generator thread panicked");
}
