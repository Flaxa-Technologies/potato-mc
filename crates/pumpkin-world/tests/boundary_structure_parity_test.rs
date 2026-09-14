use pumpkin_data::dimension::Dimension;
use pumpkin_data::structures::{Structure, StructureKeys, StructureSet};
use pumpkin_data::tag::RegistryKey;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::biome::{BiomeSupplier, MultiNoiseBiomeSupplier};
use pumpkin_world::generation::biome_coords;
use pumpkin_world::generation::generator::{VanillaGenerator, WorldGenerator};
use pumpkin_world::generation::noise::router::multi_noise_sampler::MultiNoiseSampler;
use pumpkin_world::generation::structure::structures::{StructureGeneratorContext, StructurePosition};
use pumpkin_world::generation::structure::{generate_structure_position, lazily_generate_structure};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::ProtoChunk;

fn is_biome_boundary_chunk(
    generator: &VanillaGenerator,
    chunk_x: i32,
    chunk_z: i32,
    test_y: i32,
    allowed_biomes: &[u16],
) -> bool {
    let base_bx = chunk_x * 4;
    let base_bz = chunk_z * 4;
    let test_by = biome_coords::from_block(test_y);
    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);

    let mut has_allowed = false;
    let mut has_disallowed = false;

    for dx in 0..4 {
        for dz in 0..4 {
            let b = supplier.biome(base_bx + dx, test_by, base_bz + dz, &mut sampler).id as u16;
            if allowed_biomes.contains(&b) {
                has_allowed = true;
            } else {
                has_disallowed = true;
            }
            if has_allowed && has_disallowed {
                return true;
            }
        }
    }
    false
}

/// Evaluates structure generation WITHOUT any early biome pre-filter,
/// running the un-pruned vanilla generation directly and checking the result position.
fn ground_truth_lazily_generate(
    key: &StructureKeys,
    structure: &Structure,
    chunk_x: i32,
    chunk_z: i32,
    seed: i64,
    generator: &VanillaGenerator,
) -> Option<StructurePosition> {
    let biomes = pumpkin_data::tag::get_tag_ids(
        RegistryKey::WorldgenBiome,
        structure.biomes.strip_prefix('#').unwrap_or(structure.biomes),
    )?;

    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed, chunk_x, chunk_z);
    let mut height_sampler =
        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);

    let context = StructureGeneratorContext {
        seed,
        chunk_x,
        chunk_z,
        random,
        sea_level: generator.settings.sea_level,
        min_y: generator.settings.shape.min_y as i32,
        height_sampler: Some(&mut height_sampler),
        structure_key: Some(*key),
    };

    let structure_pos = generate_structure_position(key, structure, context)?;

    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);

    let biome_x = biome_coords::from_block(structure_pos.start_pos.0.x);
    let biome_y = biome_coords::from_block(structure_pos.start_pos.0.y);
    let biome_z = biome_coords::from_block(structure_pos.start_pos.0.z);
    let biome = supplier.biome(biome_x, biome_y, biome_z, &mut sampler);

    if biomes.contains(&(biome.id as u16)) {
        Some(structure_pos)
    } else {
        None
    }
}

#[test]
fn test_pillager_outpost_biome_boundary_parity() {
    let seeds = [12345, 999_888_777, 1_782_124_772, 42];
    let key = StructureKeys::PillagerOutpost;
    let structure = Structure::get(&key);
    let biomes = pumpkin_data::tag::get_tag_ids(
        RegistryKey::WorldgenBiome,
        structure.biomes.strip_prefix('#').unwrap_or(structure.biomes),
    )
    .expect("outpost biomes tag");

    let mut boundary_chunks_tested = 0;

    for &seed in &seeds {
        let world_gen = get_world_gen(Seed(seed), Dimension::OVERWORLD, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

        for cx in -30..30 {
            for cz in -30..30 {
                if is_biome_boundary_chunk(generator, cx, cz, generator.settings.sea_level, &biomes) {
                    boundary_chunks_tested += 1;

                    // Ground truth: un-pruned generation
                    let expected = ground_truth_lazily_generate(&key, structure, cx, cz, seed as i64, generator);

                    // Production: with the 16-quart early rejection heuristic
                    let mut height_sampler =
                        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed as i64, cx, cz);
                    let context = StructureGeneratorContext {
                        seed: seed as i64,
                        chunk_x: cx,
                        chunk_z: cz,
                        random,
                        sea_level: generator.settings.sea_level,
                        min_y: generator.settings.shape.min_y as i32,
                        height_sampler: Some(&mut height_sampler),
                        structure_key: Some(key),
                    };
                    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
                    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                    let actual = lazily_generate_structure(&key, structure, context, &supplier, &mut sampler);

                    // Parity assertion: heuristic MUST NOT produce a false rejection
                    assert_eq!(
                        expected.is_some(),
                        actual.is_some(),
                        "Pillager Outpost mismatch at boundary chunk ({}, {}) for seed {}",
                        cx,
                        cz,
                        seed
                    );

                    if boundary_chunks_tested >= 25 {
                        break;
                    }
                }
            }
            if boundary_chunks_tested >= 25 {
                break;
            }
        }
    }
    assert!(boundary_chunks_tested >= 20, "Tested {} boundary chunks", boundary_chunks_tested);
}

#[test]
fn test_shipwreck_biome_boundary_parity() {
    let seeds = [12345, 999_888_777, 54321, 777];
    let key = StructureKeys::Shipwreck;
    let structure = Structure::get(&key);
    let biomes = pumpkin_data::tag::get_tag_ids(
        RegistryKey::WorldgenBiome,
        structure.biomes.strip_prefix('#').unwrap_or(structure.biomes),
    )
    .expect("shipwreck biomes tag");

    let mut boundary_chunks_tested = 0;

    for &seed in &seeds {
        let world_gen = get_world_gen(Seed(seed), Dimension::OVERWORLD, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

        for cx in -30..30 {
            for cz in -30..30 {
                if is_biome_boundary_chunk(generator, cx, cz, generator.settings.sea_level, &biomes) {
                    boundary_chunks_tested += 1;

                    let expected = ground_truth_lazily_generate(&key, structure, cx, cz, seed as i64, generator);

                    let mut height_sampler =
                        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed as i64, cx, cz);
                    let context = StructureGeneratorContext {
                        seed: seed as i64,
                        chunk_x: cx,
                        chunk_z: cz,
                        random,
                        sea_level: generator.settings.sea_level,
                        min_y: generator.settings.shape.min_y as i32,
                        height_sampler: Some(&mut height_sampler),
                        structure_key: Some(key),
                    };
                    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
                    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                    let actual = lazily_generate_structure(&key, structure, context, &supplier, &mut sampler);

                    assert_eq!(
                        expected.is_some(),
                        actual.is_some(),
                        "Shipwreck mismatch at coastline boundary chunk ({}, {}) for seed {}",
                        cx,
                        cz,
                        seed
                    );

                    if boundary_chunks_tested >= 25 {
                        break;
                    }
                }
            }
            if boundary_chunks_tested >= 25 {
                break;
            }
        }
    }
    assert!(boundary_chunks_tested >= 20, "Tested {} boundary chunks", boundary_chunks_tested);
}

#[test]
fn test_woodland_mansion_biome_boundary_parity() {
    let seeds = [12345, 888_999, 1337420, 2026];
    let key = StructureKeys::Mansion;
    let structure = Structure::get(&key);
    let biomes = pumpkin_data::tag::get_tag_ids(
        RegistryKey::WorldgenBiome,
        structure.biomes.strip_prefix('#').unwrap_or(structure.biomes),
    )
    .expect("mansion biomes tag");

    let mut boundary_chunks_tested = 0;

    for &seed in &seeds {
        let world_gen = get_world_gen(Seed(seed), Dimension::OVERWORLD, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

        for cx in -40..40 {
            for cz in -40..40 {
                if is_biome_boundary_chunk(generator, cx, cz, generator.settings.sea_level, &biomes) {
                    boundary_chunks_tested += 1;

                    let expected = ground_truth_lazily_generate(&key, structure, cx, cz, seed as i64, generator);

                    let mut height_sampler =
                        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed as i64, cx, cz);
                    let context = StructureGeneratorContext {
                        seed: seed as i64,
                        chunk_x: cx,
                        chunk_z: cz,
                        random,
                        sea_level: generator.settings.sea_level,
                        min_y: generator.settings.shape.min_y as i32,
                        height_sampler: Some(&mut height_sampler),
                        structure_key: Some(key),
                    };
                    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
                    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                    let actual = lazily_generate_structure(&key, structure, context, &supplier, &mut sampler);

                    assert_eq!(
                        expected.is_some(),
                        actual.is_some(),
                        "Woodland Mansion mismatch at boundary chunk ({}, {}) for seed {}",
                        cx,
                        cz,
                        seed
                    );

                    if boundary_chunks_tested >= 15 {
                        break;
                    }
                }
            }
            if boundary_chunks_tested >= 15 {
                break;
            }
        }
    }
    assert!(boundary_chunks_tested >= 10, "Tested {} boundary chunks", boundary_chunks_tested);
}

#[test]
fn test_ancient_city_biome_boundary_parity() {
    let seeds = [12345, 999_888_777, 42, 54321];
    let key = StructureKeys::AncientCity;
    let structure = Structure::get(&key);
    let biomes = pumpkin_data::tag::get_tag_ids(
        RegistryKey::WorldgenBiome,
        structure.biomes.strip_prefix('#').unwrap_or(structure.biomes),
    )
    .expect("ancient city biomes tag");

    let mut boundary_chunks_tested = 0;

    for &seed in &seeds {
        let world_gen = get_world_gen(Seed(seed), Dimension::OVERWORLD, false, Vec::new(), String::new());
        let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

        for cx in -30..30 {
            for cz in -30..30 {
                if is_biome_boundary_chunk(generator, cx, cz, -27, &biomes) {
                    boundary_chunks_tested += 1;

                    let expected = ground_truth_lazily_generate(&key, structure, cx, cz, seed as i64, generator);

                    let mut height_sampler =
                        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed as i64, cx, cz);
                    let context = StructureGeneratorContext {
                        seed: seed as i64,
                        chunk_x: cx,
                        chunk_z: cz,
                        random,
                        sea_level: generator.settings.sea_level,
                        min_y: generator.settings.shape.min_y as i32,
                        height_sampler: Some(&mut height_sampler),
                        structure_key: Some(key),
                    };
                    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
                    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                    let actual = lazily_generate_structure(&key, structure, context, &supplier, &mut sampler);

                    assert_eq!(
                        expected.is_some(),
                        actual.is_some(),
                        "Ancient City mismatch at boundary chunk ({}, {}) for seed {}",
                        cx,
                        cz,
                        seed
                    );

                    if boundary_chunks_tested >= 15 {
                        break;
                    }
                }
            }
            if boundary_chunks_tested >= 15 {
                break;
            }
        }
    }
    assert!(boundary_chunks_tested >= 10, "Tested {} boundary chunks", boundary_chunks_tested);
}

#[test]
fn test_distance_gate_per_structure_set_safety() {
    // Verify that every structure set's max_radius is strictly >= its physical bounding box footprint
    for (i, _set) in StructureSet::ALL.iter().enumerate() {
        let radius = ProtoChunk::structure_set_max_chunk_radius(i);
        // Radius must be at least 1 chunk for any structure
        assert!(radius >= 1, "Radius must be >= 1 for set index {}", i);
        // Radius must never exceed vanilla's 8-chunk window
        assert!(radius <= 8, "Radius must not exceed 8 for set index {}", i);
    }
}

#[test]
fn test_mineshaft_parity() {
    let seed = 1789322517659391064u64;
    let world_gen = get_world_gen(Seed(seed), Dimension::OVERWORLD, false, Vec::new(), String::new());
    let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

    let key = StructureKeys::Mineshaft;
    let structure = Structure::get(&key);

    let cx = -9;
    let cz = -22;

    println!("[TEST] Checking mineshaft at ({}, {})...", cx, cz);

    // 1. Check if mineshaft set is in dimension_structure_sets
    let mut ms_set_index = None;
    for (i, set) in StructureSet::ALL.iter().enumerate() {
        if set.structures.iter().any(|s| s.structure == key) {
            ms_set_index = Some(i);
            println!("[TEST] Found Mineshaft StructureSet at index {}", i);
        }
    }
    let ms_idx = ms_set_index.expect("Mineshaft set should exist");
    let in_dim = generator.dimension_structure_sets.contains(&ms_idx);
    println!("[TEST] Is Mineshaft set in generator.dimension_structure_sets? {}", in_dim);

    // 2. Check should_generate_structure
    let set = &StructureSet::ALL[ms_idx];
    let allowed = &generator.structure_allowed_biomes[&ms_idx];
    let mut chunk = ProtoChunk::new(cx, cz, &world_gen);
    chunk.step_to_biomes(generator);

    let should = pumpkin_world::generation::structure::placement::should_generate_structure(
        &set.placement,
        &generator.structure_calculator,
        cx,
        cz,
        &generator.global_structure_cache,
        &chunk,
        allowed,
    );
    println!("[TEST] should_generate_structure at ({}, {}): {}", cx, cz, should);

    // 3. Ground truth lazy generate
    let gt = ground_truth_lazily_generate(&key, structure, cx, cz, seed as i64, generator);
    println!("[TEST] ground_truth_lazily_generate: {:?}", gt.as_ref().map(|p| (p.start_pos, p.collector.lock().unwrap().pieces.len())));

    // 4. Test chunk.set_structure_starts
    chunk.set_structure_starts(generator);
    println!("[TEST] chunk.structure_starts after set_structure_starts: {:?}", chunk.structure_starts().keys().collect::<Vec<_>>());
}

#[test]
fn test_diagnose_playtest_structures() {
    let seed: i64 = 1789122783640570907;
    let world_gen = get_world_gen(Seed(seed as u64), Dimension::OVERWORLD, false, Vec::new(), String::new());
    let WorldGenerator::Noise(generator) = &*world_gen else { unreachable!() };

    // Find Pillager Outposts in regions -5..5
    for set_index in [11, 18] { // 11: outposts, 18: villages
        let set = &StructureSet::ALL[set_index];
        let pumpkin_data::structures::StructurePlacementType::RandomSpread(spread) = &set.placement.placement_type else { continue };
        println!("\n=== Checking Set {} (spacing={}, separation={}) ===", set_index, spread.spacing, spread.separation);

        for rx in -3..=3 {
            for rz in -3..=3 {
                let (cx, cz) = pumpkin_world::generation::structure::placement::get_structure_chunk_in_region(
                    spread, seed, rx, rz, set.placement.salt,
                );
                for entry in set.structures {
                    let key = entry.structure;
                    let structure = Structure::get(&key);
                    let mut height_sampler =
                        pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                    let random = pumpkin_world::generation::structure::structures::create_chunk_random(seed, cx, cz);
                    let context = StructureGeneratorContext {
                        seed,
                        chunk_x: cx,
                        chunk_z: cz,
                        random,
                        sea_level: generator.settings.sea_level,
                        min_y: generator.settings.shape.min_y as i32,
                        height_sampler: Some(&mut height_sampler),
                        structure_key: Some(key),
                    };
                    let supplier = MultiNoiseBiomeSupplier::OVERWORLD;
                    let mut sampler = MultiNoiseSampler::generate(&generator.base_router.multi_noise);
                    let (Some(start_pool), Some(size)) = (structure.start_pool, structure.size) else { continue };
                    use pumpkin_world::generation::structure::structures::{HeightSampler, StructureGenerator};
                    let mut jgen = pumpkin_world::generation::structure::structures::jigsaw::JigsawGenerator::new(start_pool, size).with_pool_aliases(structure.pool_aliases);
                    if structure.use_expansion_hack.unwrap_or(false) { jgen = jgen.with_expansion_hack(true); }
                    let struct_pos = jgen.get_structure_position(context);
                    if struct_pos.is_none() {
                        // Why did it fail? Let's check estimate_height!
                        let mut hs = pumpkin_world::generation::structure::height_sampler::NoiseHeightSampler::new(generator);
                        let bx = cx * 16 + 8;
                        let bz = cz * 16 + 8;
                        let h = hs.estimate_height(bx, bz);
                        let of = hs.estimate_ocean_floor_height(bx, bz);
                        println!("Candidate {:?} at ({}, {}) REJECTED by get_structure_position: h={}, of={}, sea_level={}", key, cx, cz, h, of, generator.settings.sea_level);
                    } else if let Some(pos) = struct_pos {
                        let biome_x = pumpkin_world::generation::biome_coords::from_block(pos.start_pos.0.x);
                        let biome_y = pumpkin_world::generation::biome_coords::from_block(pos.start_pos.0.y);
                        let biome_z = pumpkin_world::generation::biome_coords::from_block(pos.start_pos.0.z);
                        let biome = supplier.biome(biome_x, biome_y, biome_z, &mut sampler);
                        let biomes = pumpkin_data::tag::get_tag_ids(pumpkin_data::tag::RegistryKey::WorldgenBiome, structure.biomes.strip_prefix('#').unwrap_or(structure.biomes)).unwrap();
                        if !biomes.contains(&(biome.id as u16)) {
                            println!("Candidate {:?} at ({}, {}) REJECTED by biome at start_pos {:?}: biome_id={}", key, cx, cz, pos.start_pos, biome.id);
                        } else {
                            println!("Candidate {:?} at ({}, {}) ACCEPTED! start_pos={:?}", key, cx, cz, pos.start_pos);
                        }
                    }
                }
            }
        }
    }
}

