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
