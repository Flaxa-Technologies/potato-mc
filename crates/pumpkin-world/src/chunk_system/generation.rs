use crate::ProtoChunk;
use crate::generation::generator::WorldGenerator;
use crate::world::WorldPortalExt;
use pumpkin_config::lighting::LightingEngineConfig;

use super::{Cache, Chunk, StagedChunkEnum};

pub fn generate_single_chunk(
    generator: &WorldGenerator,
    block_registry: &dyn WorldPortalExt,
    chunk_x: i32,
    chunk_z: i32,
    target_stage: StagedChunkEnum,
) -> Chunk {
    generate_single_chunk_with_radius(
        generator,
        block_registry,
        chunk_x,
        chunk_z,
        target_stage,
        target_stage.get_direct_radius(),
    )
}

pub fn generate_single_chunk_with_radius(
    generator: &WorldGenerator,
    block_registry: &dyn WorldPortalExt,
    chunk_x: i32,
    chunk_z: i32,
    target_stage: StagedChunkEnum,
    radius: i32,
) -> Chunk {
    let mut cache = Cache::new(chunk_x - radius, chunk_z - radius, radius * 2 + 1);

    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let new_x = chunk_x + dx;
            let new_z = chunk_z + dz;

            let proto_chunk = Box::new(ProtoChunk::new(new_x, new_z, generator));

            cache.chunks.push(Chunk::Proto(proto_chunk));
        }
    }

    let stages = [
        StagedChunkEnum::Biomes,
        StagedChunkEnum::StructureStart,
        StagedChunkEnum::StructureReferences,
        StagedChunkEnum::Noise,
        StagedChunkEnum::Surface,
        StagedChunkEnum::Carvers,
        StagedChunkEnum::Features,
        StagedChunkEnum::Lighting,
        StagedChunkEnum::Spawn,
        StagedChunkEnum::Full,
    ];

    for &stage in &stages {
        if stage as u8 > target_stage as u8 {
            break;
        }

        if matches!(
            stage,
            StagedChunkEnum::Biomes
                | StagedChunkEnum::StructureStart
                | StagedChunkEnum::StructureReferences
        ) {
            cache.advance_all(
                stage,
                generator,
                block_registry,
                &LightingEngineConfig::Default,
            );
        } else {
            cache.advance(
                stage,
                generator,
                block_registry,
                &LightingEngineConfig::Default,
            );
        }
    }

    let mid = ((cache.size * cache.size) >> 1) as usize;
    cache.chunks.swap_remove(mid)
}

#[cfg(test)]
mod tests {
    use crate::chunk::ChunkHeightmapType;
    use crate::chunk_system::Chunk;
    use crate::chunk_system::{
        StagedChunkEnum, generate_single_chunk, generation::generate_single_chunk_with_radius,
    };
    use crate::generation::get_world_gen;
    use crate::world::WorldPortalExt;
    use pumpkin_data::BlockStateId;
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use std::sync::Arc;

    struct BlockRegistry;
    impl WorldPortalExt for BlockRegistry {
        fn can_place_at(
            &self,
            _block: &pumpkin_data::Block,
            _state: &pumpkin_data::BlockState,
            _block_accessor: &dyn crate::world::BlockAccessor,
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
            _cache: &mut dyn crate::generation::proto_chunk::GenerationCache,
            _biome: &'static pumpkin_data::chunk::Biome,
            _chunk_x: i32,
            _chunk_z: i32,
        ) {
        }
    }

    #[test]
    fn dimensions_taller_than_their_noise_settings_generate_all_sections() {
        for (dimension, terrain_state) in [
            (
                Dimension::THE_NETHER,
                pumpkin_data::Block::NETHERRACK.default_state.id,
            ),
            (
                Dimension::THE_END,
                pumpkin_data::Block::END_STONE.default_state.id,
            ),
        ] {
            let seed = Seed(42);
            let block_registry = Arc::new(BlockRegistry);
            let world_gen =
                get_world_gen(seed, dimension.clone(), false, Vec::new(), String::new());

            let chunk = generate_single_chunk(
                &world_gen,
                block_registry.as_ref(),
                0,
                0,
                StagedChunkEnum::Full,
            );
            let Chunk::Level(chunk) = chunk else {
                panic!("full generation must return a level chunk");
            };

            assert_eq!(chunk.section.min_y, dimension.min_y);
            assert_eq!(chunk.section.count, dimension.height as usize / 16);
            assert_eq!(
                chunk.light_engine.lock().unwrap().sky_light.len(),
                chunk.section.count
            );

            let dumped = chunk.section.dump_blocks();
            assert!(dumped.contains(&terrain_state));
            let top_section = &dumped[dumped.len() - 16 * 16 * 16..];
            assert!(top_section.iter().all(|&state| state == BlockStateId::AIR));
        }
    }

    #[test]
    fn generate_chunk_should_return() {
        let dimension = Dimension::OVERWORLD;
        let seed = Seed(42);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());

        let chunk = generate_single_chunk(
            &world_gen,
            block_registry.as_ref(),
            0,
            0,
            StagedChunkEnum::Full,
        );
        let Chunk::Level(chunk) = chunk else {
            panic!("full generation must return a level chunk");
        };
        let recalculated = chunk.calculate_heightmap();
        let generated = chunk.heightmap.lock().unwrap();
        for x in 0..16 {
            for z in 0..16 {
                for heightmap_type in [
                    ChunkHeightmapType::WorldSurface,
                    ChunkHeightmapType::MotionBlocking,
                    ChunkHeightmapType::MotionBlockingNoLeaves,
                ] {
                    assert_eq!(
                        generated.get(heightmap_type, x, z, chunk.section.min_y),
                        recalculated.get(heightmap_type, x, z, chunk.section.min_y),
                    );
                }
            }
        }
    }

    #[test]
    fn configured_seed_generates_vanilla_ancient_city_chunk() {
        let dimension = Dimension::OVERWORLD;
        let seed = Seed(1_782_124_772_053_846_960);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());

        let chunk = generate_single_chunk(
            &world_gen,
            block_registry.as_ref(),
            31,
            -12,
            StagedChunkEnum::Features,
        );
        let super::Chunk::Proto(chunk) = chunk else {
            panic!("features stage should return a proto chunk");
        };

        let mut city_blocks = 0;
        let mut jigsaw_blocks = 0;
        for x in 496..512 {
            for z in -192..-176 {
                for y in -64..320 {
                    let block = chunk
                        .get_block_state(&pumpkin_util::math::vector3::Vector3::new(x, y, z))
                        .to_block_id();
                    if [
                        pumpkin_data::Block::DEEPSLATE_BRICKS.id,
                        pumpkin_data::Block::POLISHED_DEEPSLATE.id,
                        pumpkin_data::Block::REINFORCED_DEEPSLATE.id,
                        pumpkin_data::Block::SCULK.id,
                    ]
                    .contains(&block)
                    {
                        city_blocks += 1;
                    }
                    if block == pumpkin_data::Block::JIGSAW.id {
                        jigsaw_blocks += 1;
                    }
                }
            }
        }

        assert!(
            city_blocks > 0,
            "reference chunk contains no Ancient City blocks"
        );
        assert_eq!(jigsaw_blocks, 0, "jigsaw blocks were not replaced");
    }

    #[test]
    fn seed_zero_generates_the_vanilla_pillager_outpost_chunk() {
        let dimension = Dimension::OVERWORLD;
        let seed = Seed(1_782_124_772_053_846_960);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());

        let chunk = generate_single_chunk_with_radius(
            &world_gen,
            block_registry.as_ref(),
            73,
            -82,
            StagedChunkEnum::Spawn,
            16,
        );
        let super::Chunk::Proto(chunk) = chunk else {
            panic!("spawn stage should return a proto chunk");
        };
        let mut outpost_blocks = 0;
        let mut jigsaw_blocks = 0;
        for x in 1168..1184 {
            for z in -1328..-1312 {
                for y in -64..320 {
                    let block = chunk
                        .get_block_state(&pumpkin_util::math::vector3::Vector3::new(x, y, z))
                        .to_block_id();
                    if [
                        pumpkin_data::Block::DARK_OAK_LOG.id,
                        pumpkin_data::Block::DARK_OAK_PLANKS.id,
                        pumpkin_data::Block::DARK_OAK_FENCE.id,
                    ]
                    .contains(&block)
                    {
                        outpost_blocks += 1;
                    }
                    if block == pumpkin_data::Block::JIGSAW.id {
                        jigsaw_blocks += 1;
                    }
                }
            }
        }

        assert!(
            outpost_blocks > 0,
            "reference chunk contains no outpost blocks"
        );
        assert_eq!(jigsaw_blocks, 0, "jigsaw blocks were not replaced");
    }

    #[test]
    fn fixed_seed_generates_vanilla_end_ship_chunk() {
        // Vanilla 26.2 places this seed's ship in chunk (-306, -275).
        let dimension = Dimension::THE_END;
        let seed = Seed(12_345);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());
        let chunk = generate_single_chunk_with_radius(
            &world_gen,
            block_registry.as_ref(),
            -306,
            -275,
            StagedChunkEnum::Features,
            16,
        );
        let Chunk::Proto(chunk) = chunk else {
            panic!("features stage should return a proto chunk");
        };
        let mut hash = 0xcbf29ce484222325u64;
        let mut non_air = 0;
        for y in 123..=146 {
            for x in -4896..=-4881 {
                for z in -4400..=-4393 {
                    let state =
                        chunk.get_block_state(&pumpkin_util::math::vector3::Vector3::new(x, y, z));
                    hash ^= u64::from(state.as_u16());
                    hash = hash.wrapping_mul(0x100000001b3);
                    non_air += usize::from(!state.to_state().is_air());
                }
            }
        }
        assert_eq!(non_air, 59);
        assert_eq!(hash, 0x5af3_06b3_536d_8053);
        assert!(chunk.pending_block_entities.iter().any(|nbt| {
            nbt.get_string("id") == Some("minecraft:skull")
                && nbt.get_int("x") == Some(-4888)
                && nbt.get_int("y") == Some(131)
                && nbt.get_int("z") == Some(-4399)
        }));
        assert_eq!(
            chunk
                .pending_block_entities
                .iter()
                .filter(
                    |nbt| nbt.get_string("LootTable") == Some("minecraft:chests/end_city_treasure")
                )
                .count(),
            2
        );
    }

    #[test]
    fn pillager_outpost_features_shape_ground_at_vanilla_height() {
        let dimension = Dimension::OVERWORLD;
        let seed = Seed(1_782_124_772_053_846_960);
        let block_registry = Arc::new(BlockRegistry);
        let world_gen = get_world_gen(seed, dimension, false, Vec::new(), String::new());

        let chunk = generate_single_chunk(
            &world_gen,
            block_registry.as_ref(),
            73,
            -82,
            StagedChunkEnum::Features,
        );
        let super::Chunk::Proto(chunk) = chunk else {
            panic!("features stage should return a proto chunk");
        };

        for (x, y, z) in [(1173, 70, -1311), (1173, 70, -1305)] {
            let state = chunk.get_block_state(&pumpkin_util::math::vector3::Vector3::new(x, y, z));
            assert_eq!(state.to_block_id(), pumpkin_data::Block::GRASS_BLOCK.id);
        }

        let cage_chunk = generate_single_chunk(
            &world_gen,
            block_registry.as_ref(),
            73,
            -84,
            StagedChunkEnum::Features,
        );
        let super::Chunk::Proto(cage_chunk) = cage_chunk else {
            panic!("features stage should return a proto chunk");
        };
        let state =
            cage_chunk.get_block_state(&pumpkin_util::math::vector3::Vector3::new(1183, 68, -1330));
        assert_eq!(state.to_block_id(), pumpkin_data::Block::GRASS_BLOCK.id);
    }

    #[test]
    fn batched_generation_matches_single_chunk_generation() {
        use crate::chunk_system::stage_cache::{generate_single_chunk_batched, StageCache};

        let seed = Seed(12345);
        let block_registry = Arc::new(BlockRegistry);

        // Expanded well beyond 5 coordinates to 25 coordinates per dimension (75 total chunks)
        // covering origin, adjacent clusters, negative coordinates, chunk-border boundaries, and distant offsets.
        let test_coords = [
            (0, 0), (1, 0), (0, 1), (1, 1), (-1, -1),
            (2, 2), (-2, 2), (2, -2), (-2, -2), (3, -1),
            (-3, 1), (4, 4), (-5, -5), (7, -3), (-8, 6),
            (10, -10), (-16, 16), (31, 31), (-32, -32), (0, 15),
            (15, 0), (-7, 0), (0, -7), (12, -8), (-15, -15),
        ];

        for dimension in [Dimension::OVERWORLD, Dimension::THE_NETHER, Dimension::THE_END] {
            let world_gen_single =
                get_world_gen(seed, dimension.clone(), false, Vec::new(), String::new());
            let world_gen_batched =
                get_world_gen(seed, dimension.clone(), false, Vec::new(), String::new());
            let stage_cache = StageCache::new();

            for &(cx, cz) in &test_coords {
                let single = generate_single_chunk(
                    &world_gen_single,
                    block_registry.as_ref(),
                    cx,
                    cz,
                    StagedChunkEnum::Full,
                );
                let batched = generate_single_chunk_batched(
                    &world_gen_batched,
                    block_registry.as_ref(),
                    cx,
                    cz,
                    StagedChunkEnum::Full,
                    &stage_cache,
                );

                let super::Chunk::Level(s) = single else {
                    panic!("expected Level chunk for single at ({cx}, {cz})");
                };
                let super::Chunk::Level(b) = batched else {
                    panic!("expected Level chunk for batched at ({cx}, {cz})");
                };

                assert_eq!(s.x, b.x);
                assert_eq!(s.z, b.z);
                if s.section.dump_blocks() != b.section.dump_blocks() {
                    let sb = s.section.dump_blocks();
                    let bb = b.section.dump_blocks();
                    let mut diff_count = 0;
                    let mut diff_samples = Vec::new();
                    for i in 0..sb.len() {
                        if sb[i] != bb[i] {
                            diff_count += 1;
                            if diff_samples.len() < 5 {
                                let sec = i / 4096;
                                let in_sec = i % 4096;
                                let y = sec * 16 + (in_sec / 256);
                                let rem = in_sec % 256;
                                let z = rem / 16;
                                let x = rem % 16;
                                let name0 = pumpkin_data::Block::from_state_id(sb[i]).name;
                                let name1 = pumpkin_data::Block::from_state_id(bb[i]).name;
                                diff_samples.push(format!("({x},{y},{z}) idx {i}: single={name0}({:?}) vs batched={name1}({:?})", sb[i], bb[i]));
                            }
                        }
                    }
                    println!(
                        "BATCHED PARITY MISMATCH at ({cx}, {cz}) in {:?}: {} blocks differ. Samples: {:?}",
                        dimension, diff_count, diff_samples
                    );
                }
                assert_eq!(
                    s.section.dump_blocks().len(),
                    b.section.dump_blocks().len()
                );
                assert_eq!(
                    s.section.dump_blocks() == b.section.dump_blocks(),
                    true,
                    "blocks mismatch in {:?} at ({cx}, {cz})",
                    dimension
                );
                assert_eq!(
                    s.section.dump_biomes(),
                    b.section.dump_biomes(),
                    "biomes mismatch in {:?} at ({cx}, {cz})",
                    dimension
                );
            }
        }
    }

    /// Tests for Steel's Trap 1: thread-local state, thread-assignment order, or thread scheduling
    /// leaking or affecting chunk generation output.
    ///
    /// Verifies that:
    /// 1. A single-threaded sequential pool (1 thread)
    /// 2. A 4-threaded concurrent pool (forward dispatch order)
    /// 3. A 12-threaded concurrent pool (reversed dispatch order)
    /// 4. A concurrent batched pool using shared StageCache (12 threads)
    ///
    /// all produce 100% bit-for-bit identical block states and biome dumps for every chunk.
    #[test]
    fn test_steels_trap_thread_invariance() {
        use crate::chunk_system::stage_cache::{generate_single_chunk_batched, StageCache};
        use std::collections::HashMap;

        let seed = Seed(42);
        let block_registry = Arc::new(BlockRegistry);

        for dimension in [Dimension::OVERWORLD, Dimension::THE_NETHER] {
            // Test a 4x4 chunk cluster (16 chunks) where 3x3 neighbor pyramids heavily overlap
            let mut coords = Vec::new();
            for cx in 0..4 {
                for cz in 0..4 {
                    coords.push((cx, cz));
                }
            }

            // 1. Reference Run: Single thread (num_threads: 1)
            let world_gen_1 = Arc::new(get_world_gen(
                seed,
                dimension.clone(),
                false,
                Vec::new(),
                String::new(),
            ));
            let pool_1 = rayon::ThreadPoolBuilder::new()
                .num_threads(1)
                .build()
                .expect("Failed to build 1-thread pool");

            let ref_chunks: Arc<std::sync::Mutex<HashMap<(i32, i32), (Vec<BlockStateId>, Vec<u8>)>>> =
                Arc::new(std::sync::Mutex::new(HashMap::new()));

            pool_1.install(|| {
                for &(cx, cz) in &coords {
                    let chunk = generate_single_chunk(
                        world_gen_1.as_ref(),
                        block_registry.as_ref(),
                        cx,
                        cz,
                        StagedChunkEnum::Full,
                    );
                    let super::Chunk::Level(lvl) = chunk else {
                        panic!("expected Level chunk for Full stage");
                    };
                    ref_chunks.lock().unwrap().insert(
                        (cx, cz),
                        (lvl.section.dump_blocks(), lvl.section.dump_biomes()),
                    );
                }
            });

            let reference = ref_chunks.lock().unwrap().clone();
            assert_eq!(reference.len(), 16);

            // 2. Test Run: 4 threads, forward dispatch
            let world_gen_4 = Arc::new(get_world_gen(
                seed,
                dimension.clone(),
                false,
                Vec::new(),
                String::new(),
            ));
            let pool_4 = rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .expect("Failed to build 4-thread pool");

            pool_4.scope(|s| {
                for &(cx, cz) in &coords {
                    let wg = world_gen_4.clone();
                    let br = block_registry.clone();
                    let expected = reference.get(&(cx, cz)).unwrap().clone();
                    let dim = dimension.clone();

                    s.spawn(move |_| {
                        let chunk = generate_single_chunk(
                            wg.as_ref(),
                            br.as_ref(),
                            cx,
                            cz,
                            StagedChunkEnum::Full,
                        );
                        let super::Chunk::Level(lvl) = chunk else {
                            panic!("expected Level chunk for Full stage");
                        };
                        assert_eq!(
                            lvl.section.dump_blocks(),
                            expected.0,
                            "Steel's Trap 1: 4-thread block mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                        assert_eq!(
                            lvl.section.dump_biomes(),
                            expected.1,
                            "Steel's Trap 1: 4-thread biome mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                    });
                }
            });

            // 3. Test Run: 12 threads, reversed dispatch order
            let world_gen_12 = Arc::new(get_world_gen(
                seed,
                dimension.clone(),
                false,
                Vec::new(),
                String::new(),
            ));
            let pool_12 = rayon::ThreadPoolBuilder::new()
                .num_threads(12)
                .build()
                .expect("Failed to build 12-thread pool");

            let mut reversed_coords = coords.clone();
            reversed_coords.reverse();

            pool_12.scope(|s| {
                for &(cx, cz) in &reversed_coords {
                    let wg = world_gen_12.clone();
                    let br = block_registry.clone();
                    let expected = reference.get(&(cx, cz)).unwrap().clone();
                    let dim = dimension.clone();

                    s.spawn(move |_| {
                        let chunk = generate_single_chunk(
                            wg.as_ref(),
                            br.as_ref(),
                            cx,
                            cz,
                            StagedChunkEnum::Full,
                        );
                        let super::Chunk::Level(lvl) = chunk else {
                            panic!("expected Level chunk");
                        };
                        assert_eq!(
                            lvl.section.dump_blocks(),
                            expected.0,
                            "Steel's Trap 1: 12-thread reversed block mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                        assert_eq!(
                            lvl.section.dump_biomes(),
                            expected.1,
                            "Steel's Trap 1: 12-thread reversed biome mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                    });
                }
            });

            // 4. Test Run: Concurrent Batched StageCache (multi-threaded with cached intermediate stages)
            let world_gen_batched = Arc::new(get_world_gen(
                seed,
                dimension.clone(),
                false,
                Vec::new(),
                String::new(),
            ));
            let stage_cache = Arc::new(StageCache::new());
            pool_12.scope(|s| {
                for &(cx, cz) in &coords {
                    let wg = world_gen_batched.clone();
                    let br = block_registry.clone();
                    let sc = stage_cache.clone();
                    let expected = reference.get(&(cx, cz)).unwrap().clone();
                    let dim = dimension.clone();

                    s.spawn(move |_| {
                        let chunk = generate_single_chunk_batched(
                            wg.as_ref(),
                            br.as_ref(),
                            cx,
                            cz,
                            StagedChunkEnum::Full,
                            &sc,
                        );
                        let super::Chunk::Level(lvl) = chunk else {
                            panic!("expected Level chunk");
                        };
                        assert_eq!(
                            lvl.section.dump_blocks(),
                            expected.0,
                            "Steel's Trap 1: Batched StageCache block mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                        assert_eq!(
                            lvl.section.dump_biomes(),
                            expected.1,
                            "Steel's Trap 1: Batched StageCache biome mismatch at ({cx}, {cz}) in {:?}",
                            dim
                        );
                    });
                }
            });
        }
    }
}
