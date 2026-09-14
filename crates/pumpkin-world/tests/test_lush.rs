use pumpkin_util::random::{
    get_decorator_seed, worldgen_random::WorldgenRandom, RandomGenerator, RandomImpl,
};

#[test]
fn test_trace_lush_placement() {
    let world_seed = 1789322517659391064u64;
    let chunk_x = -32i32;
    let chunk_z = -18i32;
    let origin_x = chunk_x * 16;
    let origin_z = chunk_z * 16;

    let population_seed =
        WorldgenRandom::get_population_seed(world_seed, origin_x, origin_z);

    println!("Decoration seed: 0x{:x} ({})", population_seed, population_seed as i64);

    let step = 9usize;
    let feature_index = 27usize;
    let decorator_seed = get_decorator_seed(population_seed, feature_index as u64, step as u64);
    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

    println!("\nTracing first 10 attempts for feature 27 (lush_caves_ceiling_vegetation):");
    for attempt in 0..10 {
        let dx = random.next_bounded_i32(16);
        let dz = random.next_bounded_i32(16);
        let y = random.next_inbetween_i32(-64, 256);
        println!(
            "  Attempt {:2}: in_square=({:2}, {:2}) -> world=({:4}, {:3}, {:4})",
            attempt,
            dx,
            dz,
            origin_x + dx,
            y,
            origin_z + dz
        );
    }
}

#[test]
fn test_trace_lush_chunk_32_18() {
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use pumpkin_world::generation::get_world_gen;
    use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum, Chunk};
    use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
    use pumpkin_data::Block;
    use pumpkin_data::chunk::Biome;
    use pumpkin_util::math::position::BlockPos;
    use pumpkin_world::generation::proto_chunk::GenerationCache;

    struct DummyPortal;
    impl WorldPortalExt for DummyPortal {
        fn can_place_at(&self, _: &Block, _: &pumpkin_data::BlockState, _: &dyn BlockAccessor, _: &BlockPos) -> bool { true }
        fn mirror(&self, b: &Block, s: pumpkin_data::BlockStateId, m: pumpkin_data::Mirror) -> &'static pumpkin_data::BlockState { b.mirror(s, m) }
        fn rotate(&self, b: &Block, s: pumpkin_data::BlockStateId, r: pumpkin_data::Rotation) -> &'static pumpkin_data::BlockState { b.rotate(s, r) }
        fn spawn_mobs_for_chunk_generation(&self, _: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache, _: &'static Biome, _: i32, _: i32) {}
    }

    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let chunk = generate_single_chunk(
        &generator,
        &DummyPortal,
        -32,
        -18,
        StagedChunkEnum::Surface,
    );
    let Chunk::Proto(proto) = chunk else { panic!("Expected ProtoChunk"); };
    
    let step = 9usize;
    let feature_index = 27usize;
    let origin_x = -32 * 16;
    let origin_z = -18 * 16;
    let population_seed = WorldgenRandom::get_population_seed(1789322517659391064u64, origin_x, origin_z);
    let decorator_seed = get_decorator_seed(population_seed, feature_index as u64, step as u64);
    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

    println!("Scanning 125 attempts for feature 27:");
    let mut successful_attempts = Vec::new();
    for attempt in 0..125 {
        let dx = random.next_bounded_i32(16);
        let dz = random.next_bounded_i32(16);
        let y = random.next_inbetween_i32(-64, 256);
        let mut pos = BlockPos::new(origin_x + dx, y, origin_z + dz);
        
        // EnvironmentScan: direction Up, target Solid, allowed Air, max_steps 12
        let mut hit = false;
        for _ in 0..12 {
            if !proto.is_air(&pos.0) {
                hit = true;
                break;
            }
            pos.0.y += 1;
        }
        if !hit { continue; }
        
        // RandomOffset: (0, -1, 0)
        let offset_pos = BlockPos::new(pos.0.x, pos.0.y - 1, pos.0.z);
        let biome = proto.get_biome_for_terrain_gen(offset_pos.0.x, offset_pos.0.y, offset_pos.0.z);
        let has_feat = biome.features.get(9).map(|f| f.contains(&pumpkin_data::placed_feature::PlacedFeature::LushCavesCeilingVegetation)).unwrap_or(false);
        if has_feat {
            successful_attempts.push((attempt, pos, offset_pos, biome.registry_id));
        }
    }
    println!("Successful attempts (passed scan & biome): {}", successful_attempts.len());
    for s in &successful_attempts {
        println!("  Attempt {}: ceiling at {:?}, offset {:?}, biome {}", s.0, s.1, s.2, s.3);
    }
}

#[test]
fn test_trace_attempt_5() {
    let step = 9usize;
    let feature_index = 27usize;
    let origin_x = -32 * 16;
    let origin_z = -18 * 16;
    let population_seed = WorldgenRandom::get_population_seed(1789322517659391064u64, origin_x, origin_z);
    let decorator_seed = get_decorator_seed(population_seed, feature_index as u64, step as u64);
    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

    for _ in 0..5 {
        let _ = random.next_bounded_i32(16);
        let _ = random.next_bounded_i32(16);
        let _ = random.next_inbetween_i32(-64, 256);
    }

    let dx5 = random.next_bounded_i32(16);
    let dz5 = random.next_bounded_i32(16);
    let y5 = random.next_inbetween_i32(-64, 256);
    println!("Attempt 5 initial: in_square=({}, {}), y={} -> world=({}, {}, {})",
        dx5, dz5, y5, origin_x + dx5, y5, origin_z + dz5);

    let x_radius = random.next_inbetween_i32(4, 7) + 1;
    let z_radius = random.next_inbetween_i32(4, 7) + 1;
    println!("xRadius: {}, zRadius: {}", x_radius, z_radius);

    // In Rust, VegetationPatchFeature uses JavaBlockPosSet
    // Let's test JavaBlockPosSet directly with dx, 0, dz
    let extra_edge_column_chance = 0.3f32;
    let mut surface = pumpkin_world::generation::feature::features::vegetation_patch::JavaBlockPosSet::new();

    let mut edge_rolls = 0;
    for dx in -x_radius..=x_radius {
        let is_x_edge = dx == -x_radius || dx == x_radius;
        for dz in -z_radius..=z_radius {
            let is_z_edge = dz == -z_radius || dz == z_radius;
            let is_corner = is_x_edge && is_z_edge;
            let is_edge = is_x_edge || is_z_edge;
            let is_edge_but_not_corner = is_edge && !is_corner;

            if is_corner { continue; }
            if is_edge_but_not_corner && (extra_edge_column_chance == 0.0 || random.next_f32() > extra_edge_column_chance) {
                continue;
            }
            if is_edge_but_not_corner { edge_rolls += 1; }
            let _depth = random.next_inbetween_i32(1, 2);
            surface.insert(pumpkin_util::math::position::BlockPos::new(dx, 0, dz));
        }
    }
    let vec = surface.into_vec();
    println!("Surface size: {}, edgeRolls: {}", vec.len(), edge_rolls);
    println!("First 5 in surface iteration order:");
    for p in vec.iter().take(5) {
        println!("  {:?}", p);
    }
}

#[test]
fn test_trace_clay_chunk_32_18() {
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use pumpkin_world::generation::get_world_gen;
    use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum, Chunk};
    use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
    use pumpkin_data::Block;
    use pumpkin_data::chunk::Biome;
    use pumpkin_util::math::position::BlockPos;
    use pumpkin_world::generation::proto_chunk::GenerationCache;

    struct DummyPortal;
    impl WorldPortalExt for DummyPortal {
        fn can_place_at(&self, _: &Block, _: &pumpkin_data::BlockState, _: &dyn BlockAccessor, _: &BlockPos) -> bool { true }
        fn mirror(&self, b: &Block, s: pumpkin_data::BlockStateId, m: pumpkin_data::Mirror) -> &'static pumpkin_data::BlockState { b.mirror(s, m) }
        fn rotate(&self, b: &Block, s: pumpkin_data::BlockStateId, r: pumpkin_data::Rotation) -> &'static pumpkin_data::BlockState { b.rotate(s, r) }
        fn spawn_mobs_for_chunk_generation(&self, _: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache, _: &'static Biome, _: i32, _: i32) {}
    }

    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let chunk = generate_single_chunk(
        &generator,
        &DummyPortal,
        -32,
        -18,
        StagedChunkEnum::Carvers,
    );
    let Chunk::Proto(proto) = chunk else { panic!("Expected ProtoChunk"); };

    let step = 9usize;
    let feature_index = 29usize; // lush_caves_clay
    let origin_x = -32 * 16;
    let origin_z = -18 * 16;
    let population_seed = WorldgenRandom::get_population_seed(1789322517659391064u64, origin_x, origin_z);
    let decorator_seed = get_decorator_seed(population_seed, feature_index as u64, step as u64);
    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

    println!("Tracing 62 attempts for Feature 29 (lush_caves_clay):");
    for attempt in 0..62 {
        let dx = random.next_bounded_i32(16);
        let dz = random.next_bounded_i32(16);
        let y = random.next_inbetween_i32(-64, 256);
        let mut pos = BlockPos::new(origin_x + dx, y, origin_z + dz);

        // Allowed search condition: AIR
        if !proto.is_air(&pos.0) {
            continue;
        }

        // EnvironmentScan: direction Down, target Solid, allowed Air, max_steps 12
        let mut hit = false;
        for _ in 0..12 {
            if !proto.is_air(&pos.0) {
                hit = true;
                break;
            }
            pos.0.y -= 1;
            if pos.0.y < -64 { break; }
            if !proto.is_air(&pos.0) {
                hit = true;
                break;
            }
        }
        if !hit { continue; }

        // RandomOffset: (0, 1, 0)
        let offset_pos = BlockPos::new(pos.0.x, pos.0.y + 1, pos.0.z);
        let biome = proto.get_biome_for_terrain_gen(offset_pos.0.x, offset_pos.0.y, offset_pos.0.z);
        let has_feat = biome.features.get(9).map(|f| f.contains(&pumpkin_data::placed_feature::PlacedFeature::LushCavesClay)).unwrap_or(false);

        println!("  Attempt {:2}: start=({:4}, {:3}, {:4}) -> hit floor at y={:3}, offset_y={:3}, biome={}, has_feat={}",
            attempt, origin_x + dx, y, origin_z + dz, pos.0.y, offset_pos.0.y, biome.registry_id, has_feat);
    }

    println!("\nNow executing PlacedFeature::generate for Feature 29 on proto:");
    let feat_29 = pumpkin_world::generation::feature::placed_features::PLACED_FEATURES
        .get(&pumpkin_data::placed_feature::PlacedFeature::LushCavesClay)
        .expect("LushCavesClay exists");

    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));
    let mut chunk = proto;
    let min_y = chunk.generation_bottom_y();
    let height = chunk.generation_height();
    let origin_pos = BlockPos::new(origin_x, min_y as i32, origin_z);

    let sample_pos = BlockPos::new(-507, -29, -274);
    println!("Block at {:?} BEFORE feature 29: {:?}", sample_pos, chunk.get_block_state(&sample_pos.0).to_state().id.to_block().name);

    let mut old_blocks = Vec::with_capacity(16 * 16 * chunk.height() as usize);
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..chunk.height() as i32 {
                old_blocks.push(chunk.get_block_state_raw(x, y, z));
            }
        }
    }

    let res_29 = feat_29.generate(
        &mut *chunk,
        &DummyPortal,
        min_y,
        height,
        pumpkin_data::placed_feature::PlacedFeature::LushCavesClay,
        &mut random,
        origin_pos,
    );
    println!("feat_29 returned: {}", res_29);

    let mut changed = 0;
    let mut idx = 0;
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..chunk.height() as i32 {
                let current = chunk.get_block_state_raw(x, y, z);
                if current != old_blocks[idx] {
                    changed += 1;
                    let world_x = origin_x + x;
                    let world_y = y + chunk.bottom_y() as i32;
                    let world_z = origin_z + z;
                    let old_b = old_blocks[idx].to_state().id.to_block().name;
                    let new_b = current.to_state().id.to_block().name;
                    if changed <= 25 {
                        println!("  Changed ({}, {}, {}): {} -> {}", world_x, world_y, world_z, old_b, new_b);
                    }
                }
                idx += 1;
            }
        }
    }
    println!("Total blocks changed by feature 29: {}", changed);

    println!("Block at {:?} after feature 29: {:?}", sample_pos, chunk.get_block_state(&sample_pos.0).to_state().id.to_block().name);

    println!("\nNow executing PlacedFeature::generate for Feature 30 (LushCavesVegetation) on proto:");
    let feat_30 = pumpkin_world::generation::feature::placed_features::PLACED_FEATURES
        .get(&pumpkin_data::placed_feature::PlacedFeature::LushCavesVegetation)
        .expect("LushCavesVegetation exists");

    let feat_30_index = 30usize;
    let decorator_seed_30 = get_decorator_seed(population_seed, feat_30_index as u64, step as u64);
    let mut random_30 = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed_30));

    feat_30.generate(
        &mut *chunk,
        &DummyPortal,
        min_y,
        height,
        pumpkin_data::placed_feature::PlacedFeature::LushCavesVegetation,
        &mut random_30,
        origin_pos,
    );

    println!("Block at {:?} after feature 30: {:?}", sample_pos, chunk.get_block_state(&sample_pos.0).to_state().id.to_block().name);
}

#[test]
fn test_attempt1_water_surface() {
    use pumpkin_data::dimension::Dimension;
    use pumpkin_util::world_seed::Seed;
    use pumpkin_world::generation::get_world_gen;
    use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum, Chunk};
    use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
    use pumpkin_data::{Block, BlockDirection};
    use pumpkin_data::chunk::Biome;
    use pumpkin_util::math::position::BlockPos;
    use pumpkin_world::generation::proto_chunk::GenerationCache;

    struct DummyPortal;
    impl WorldPortalExt for DummyPortal {
        fn can_place_at(&self, _: &Block, _: &pumpkin_data::BlockState, _: &dyn BlockAccessor, _: &BlockPos) -> bool { true }
        fn mirror(&self, b: &Block, s: pumpkin_data::BlockStateId, m: pumpkin_data::Mirror) -> &'static pumpkin_data::BlockState { b.mirror(s, m) }
        fn rotate(&self, b: &Block, s: pumpkin_data::BlockStateId, r: pumpkin_data::Rotation) -> &'static pumpkin_data::BlockState { b.rotate(s, r) }
        fn spawn_mobs_for_chunk_generation(&self, _: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache, _: &'static Biome, _: i32, _: i32) {}
    }

    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let chunk = generate_single_chunk(
        &generator,
        &DummyPortal,
        -32,
        -18,
        StagedChunkEnum::Carvers,
    );
    let Chunk::Proto(mut proto) = chunk else { panic!("Expected ProtoChunk"); };

    let step = 9usize;
    let feature_index = 29usize; // lush_caves_clay
    let origin_x = -32 * 16;
    let origin_z = -18 * 16;
    let population_seed = WorldgenRandom::get_population_seed(1789322517659391064u64, origin_x, origin_z);
    let decorator_seed = get_decorator_seed(population_seed, feature_index as u64, step as u64);
    let mut random = RandomGenerator::Worldgen(WorldgenRandom::from_seed(decorator_seed));

    // Attempt 0:
    let _ = random.next_bounded_i32(16);
    let _ = random.next_bounded_i32(16);
    let _ = random.next_inbetween_i32(-64, 256);

    // Attempt 1:
    let dx = random.next_bounded_i32(16);
    let dz = random.next_bounded_i32(16);
    let y = random.next_inbetween_i32(-64, 256);
    let mut scan_pos = BlockPos::new(origin_x + dx, y, origin_z + dz);
    println!("Attempt 1 scan start: {:?}", scan_pos);
    for _ in 0..12 {
        if !proto.is_air(&scan_pos.0) { break; }
        scan_pos.0.y -= 1;
        if !proto.is_air(&scan_pos.0) { break; }
    }
    println!("Attempt 1 hit floor at: {:?}", scan_pos);
    let origin = BlockPos::new(scan_pos.0.x, scan_pos.0.y + 1, scan_pos.0.z);

    // ConfiguredFeature: RandomBooleanSelector
    let selector_bool = random.next_bool();
    println!("RandomBooleanSelector: {} (false = clay_pool)", selector_bool);

    let feat = pumpkin_world::generation::feature::configured_features::CONFIGURED_FEATURES
        .get(&pumpkin_data::configured_feature::ConfiguredFeature::ClayPoolWithDripleaves)
        .expect("ClayPoolWithDripleaves exists");
    let pumpkin_world::generation::feature::configured_features::ConfiguredFeature::WaterloggedVegetationPatch(patch) = feat else {
        panic!("Expected WaterloggedVegetationPatch");
    };

    let x_radius = patch.base.xz_radius.get(&mut random) + 1;
    let z_radius = patch.base.xz_radius.get(&mut random) + 1;
    println!("x_radius={}, z_radius={}", x_radius, z_radius);

    // Dump proto blocks to file for Java
    {
        use std::io::Write;
        let mut f = std::fs::File::create("C:/Users/Prasad/Documents/Flaxa/test_auth/server-d/proto_blocks_32_18.txt").unwrap();
        for y in -25..=0 {
            for lz in -5..=20 {
                for lx in -5..=20 {
                    let wx = origin_x + lx;
                    let wz = origin_z + lz;
                    let state = GenerationCache::get_block_state(&*proto, &pumpkin_util::math::vector3::Vector3::new(wx, y, wz));
                    let name = state.to_state().id.to_block().name;
                    writeln!(f, "{},{},{},{}", lx, y, lz, name).unwrap();
                }
            }
        }
    }

    let surface = patch.base.place_ground_patch(
        &mut *proto,
        &DummyPortal,
        &mut random,
        origin,
        &patch.base.replaceable,
        x_radius,
        z_radius,
    );
    println!("place_ground_patch surface size: {}", surface.len());

    for lz in -2..=4 {
        let lx = 4;
        let mut pos = origin.offset(pumpkin_util::math::vector3::Vector3::new(lx - (origin.0.x - origin_x), 0, lz - (origin.0.z - origin_z)));
        let start_pos = pos;
        let inwards = BlockDirection::Down;
        let outwards = BlockDirection::Up;
        for _ in 0..patch.base.vertical_range {
            if !proto.is_air(&pos.0) { break; }
            pos = pos.offset(inwards.to_offset());
        }
        for _ in 0..patch.base.vertical_range {
            if proto.is_air(&pos.0) { break; }
            pos = pos.offset(outwards.to_offset());
        }
        let below_pos = pos.down();
        let below_state = GenerationCache::get_block_state(&*proto, &below_pos.0);
        let air_pos = proto.is_air(&pos.0);
        let solid = below_state.to_state().is_side_solid(inwards.opposite());
        println!("  column lx={}, lz={}: start={:?} -> air_pos={:?} (air={}), below={:?} (block={}, solid={})",
            lx, lz, start_pos, pos, air_pos, below_pos, below_state.to_state().id.to_block().name, solid);
    }

    let mut water_surface_set = pumpkin_world::generation::feature::features::vegetation_patch::JavaBlockPosSet::new();
    for pos in &surface {
        let mut exposed = false;
        for dir in [BlockDirection::North, BlockDirection::East, BlockDirection::South, BlockDirection::West, BlockDirection::Down] {
            let test_pos = pos.offset(dir.to_offset());
            let solid = GenerationCache::get_block_state(&*proto, &test_pos.0).to_state().is_side_solid(dir.opposite());
            if !solid {
                exposed = true;
                break;
            }
        }
        if !exposed {
            water_surface_set.insert(*pos);
        }
    }
    let water_surface = water_surface_set.into_vec();
    println!("water_surface size: {}", water_surface.len());

    println!("Tracing distribute_vegetation on water_surface (size={}):", water_surface.len());
    for (i, p) in water_surface.iter().enumerate() {
        let roll = random.next_f32();
        let pass = roll < 0.1f32;
        if pass {
            let choice = random.next_bounded_i32(5);
            println!("  [{:2}] {:?} -> roll={:.4} PASS! choice={}", i, p, roll, choice);
            if choice == 0 {
                // small dripleaf: rolls weighted state provider (4 weights of 1 -> next_bounded_i32(4))
                let facing_roll = random.next_bounded_i32(4);
                println!("      small_dripleaf facing_roll={}", facing_roll);
            } else {
                // big dripleaf: rolls weighted_list (weight 2 for uniform 0..4, weight 1 for 0)
                let stem_weight = random.next_bounded_i32(3);
                let stem_len = if stem_weight < 2 {
                    random.next_inbetween_i32(0, 4)
                } else {
                    0
                };
                println!("      big_dripleaf stem_weight={}, stem_len={}", stem_weight, stem_len);
            }
        }
    }
}







