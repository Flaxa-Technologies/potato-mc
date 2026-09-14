use pumpkin_data::dimension::Dimension;
use pumpkin_data::Block;
use pumpkin_util::world_seed::Seed;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum, Chunk};
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
use pumpkin_data::BlockStateId;

struct DummyPortal;
impl WorldPortalExt for DummyPortal {
    fn can_place_at(
        &self,
        _block: &pumpkin_data::Block,
        _state: &pumpkin_data::BlockState,
        _block_accessor: &dyn pumpkin_world::world::BlockAccessor,
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
    ) {}
}

#[test]
fn test_ore_vein_tuff_count_parity() {
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
        -23,
        -26,
        StagedChunkEnum::Full,
    );
    let (tuff_count, deepslate_count) = match chunk {
        Chunk::Level(chunk_data) => {
            let mut tuff = 0;
            let mut deepslate = 0;
            for x in 0..16 {
                for z in 0..16 {
                    for y in -64..320 {
                        if let Some(state_id) = chunk_data.section.get_block_absolute_y(x, y, z) {
                            let block = state_id.to_block();
                            if *block == Block::TUFF {
                                tuff += 1;
                            } else if *block == Block::DEEPSLATE {
                                deepslate += 1;
                            }
                        }
                    }
                }
            }
            (tuff, deepslate)
        }
        Chunk::Proto(proto) => {
            let mut tuff = 0;
            let mut deepslate = 0;
            for x in 0..16 {
                for z in 0..16 {
                    let wx = -23 * 16 + x as i32;
                    let wz = -26 * 16 + z as i32;
                    for y in -64..320 {
                        let pos = BlockPos(Vector3::new(wx, y, wz));
                        let block = proto.get_block(&pos);
                        if *block == Block::TUFF {
                            tuff += 1;
                        } else if *block == Block::DEEPSLATE {
                            deepslate += 1;
                        }
                    }
                }
            }
            (tuff, deepslate)
        }
    };
    println!("Chunk (-23, -26) new generation: TUFF={}, DEEPSLATE={}", tuff_count, deepslate_count);
    assert!(tuff_count < 2000, "TUFF count must drop from 6301 down near Vanilla's ~1078, got {}", tuff_count);
}

#[test]
fn test_bedrock_and_deepslate_seeds() {
    use pumpkin_world::generation::GlobalRandomConfig;
    use pumpkin_util::random::RandomImpl;
    let random_config = GlobalRandomConfig::new(1789322517659391064, false);
    let mut bedrock_rand = random_config.base_random_deriver.split_string("minecraft:bedrock_floor");
    let bedrock_splitter = bedrock_rand.next_splitter();
    
    let mut r1 = bedrock_splitter.split_pos(0, 0, 0);
    let mut r2 = bedrock_splitter.split_pos(10, -60, -5);
    let v1 = r1.next_f32();
    let v2 = r2.next_f32();
    println!("Pumpkin split_string bedrock at(0,0,0): {}", v1);
    println!("Pumpkin split_string bedrock at(10,-60,-5): {}", v2);

    let be_lo = 13544455532117611141u64;
    let be_hi = 14185350335435586452u64;
    let mut be_bedrock_rand = random_config.base_random_deriver.from_lo_and_hi(be_lo, be_hi);
    let be_bedrock_splitter = be_bedrock_rand.next_splitter();
    let mut be_r1 = be_bedrock_splitter.split_pos(0, 0, 0);
    let mut be_r2 = be_bedrock_splitter.split_pos(10, -60, -5);
    let be_v1 = be_r1.next_f32();
    let be_v2 = be_r2.next_f32();
    println!("Pumpkin from_lo_and_hi (fixed BE) bedrock at(0,0,0): {}", be_v1);
    println!("Pumpkin from_lo_and_hi (fixed BE) bedrock at(10,-60,-5): {}", be_v2);

    // Assert BE matches split_string and exact Vanilla 26.2 values
    assert_eq!(v1, 0.06072384);
    assert_eq!(v2, 0.7912304);
    assert_eq!(be_v1, 0.06072384);
    assert_eq!(be_v2, 0.7912304);

    let mut deepslate_rand = random_config.base_random_deriver.split_string("minecraft:deepslate");
    let deepslate_splitter = deepslate_rand.next_splitter();
    let mut dr1 = deepslate_splitter.split_pos(0, 0, 0);
    let mut dr2 = deepslate_splitter.split_pos(10, 4, -5);
    let dv1 = dr1.next_f32();
    let dv2 = dr2.next_f32();
    println!("Pumpkin split_string deepslate at(0,0,0): {}", dv1);
    println!("Pumpkin split_string deepslate at(10,4,-5): {}", dv2);

    let be_d_lo = 10411719568726253007u64;
    let be_d_hi = 14964796469053385315u64;
    let mut be_deepslate_rand = random_config.base_random_deriver.from_lo_and_hi(be_d_lo, be_d_hi);
    let be_deepslate_splitter = be_deepslate_rand.next_splitter();
    let mut be_dr1 = be_deepslate_splitter.split_pos(0, 0, 0);
    let mut be_dr2 = be_deepslate_splitter.split_pos(10, 4, -5);
    let be_dv1 = be_dr1.next_f32();
    let be_dv2 = be_dr2.next_f32();
    println!("Pumpkin from_lo_and_hi deepslate at(0,0,0): {}", be_dv1);
    println!("Pumpkin from_lo_and_hi deepslate at(10,4,-5): {}", be_dv2);
    assert_eq!(dv1, be_dv1);
    assert_eq!(dv2, be_dv2);
}

#[test]
fn test_chunk_minus27_minus16_stages() {
    use pumpkin_world::chunk_system::stage_cache::generate_batch_chunks;
    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let pumpkin_world::generation::generator::WorldGenerator::Noise(noise_gen) = &*generator else { panic!() };
    let set = &pumpkin_data::structures::StructureSet::MINESHAFTS;
    let allowed = &noise_gen.structure_allowed_biomes[&9]; // 9 is MINESHAFTS index
    use pumpkin_util::random::{get_carver_seed, get_large_feature_seed, legacy_rand::LegacyRand, RandomImpl};
    let carver_seed = get_carver_seed(1789322517659391064, -26, -17);
    let mut rand1 = LegacyRand::from_seed(carver_seed);
    let val_carver = rand1.next_f64();

    let large_feature_seed = get_large_feature_seed(1789322517659391064, -26, -17);
    let mut rand2 = LegacyRand::from_seed(large_feature_seed);
    let val_large = rand2.next_f64();

    println!("At (-26, -17): CARVER seed f64 = {} ( < 0.004? {})", val_carver, val_carver < 0.004);
    println!("At (-26, -17): LARGE FEATURE seed f64 = {} ( < 0.004? {})", val_large, val_large < 0.004);
    let wx = -27 * 16;
    let wz = -16 * 16 + 3;
    let pos = BlockPos(Vector3::new(wx, 46, wz));

    let chunks_carvers = generate_batch_chunks(&generator, &DummyPortal, &[(-27, -16)], StagedChunkEnum::Carvers);
    match &chunks_carvers[0] {
        Chunk::Proto(p) => println!("At (0, 46, 3) after CARVERS: {:?}", p.get_block(&pos).name),
        _ => {}
    }

    let chunks_features = generate_batch_chunks(&generator, &DummyPortal, &[(-27, -16)], StagedChunkEnum::Features);
    match &chunks_features[0] {
        Chunk::Proto(p) => {
            for y in 40..=50 {
                let ppos = BlockPos(Vector3::new(wx, y, wz));
                println!("y={}: {:?}", y, p.get_block(&ppos).name);
            }
        }
        _ => {}
    }
}

#[test]
fn test_chunk_27_16_cave() {
    use pumpkin_world::generation::noise::{ChunkNoiseGenerator, CHUNK_DIM};
    use pumpkin_world::generation::noise::router::density_volume::DensityVolume;
    use pumpkin_world::generation::positions::chunk_pos::start_block_x;
    use pumpkin_world::generation::positions::chunk_pos::start_block_z;
    use pumpkin_world::generation::proto_chunk::StandardChunkFluidLevelSampler;
    use pumpkin_world::generation::noise::aquifer_sampler::FluidLevel;
    use pumpkin_world::generation::noise::router::surface_height_sampler::SurfaceHeightEstimateSampler;
    use pumpkin_world::generation::noise::router::surface_height_sampler::SurfaceHeightSamplerBuilderOptions;
    use pumpkin_util::random::RandomImpl;

    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    let pumpkin_world::generation::generator::WorldGenerator::Noise(noise_gen) = &*generator else { panic!() };

    let count_air = |chunk: &Chunk| {
        let mut count = 0;
        if let Chunk::Proto(p) = chunk {
            for y in 0..=30 {
                for z in 0..16 {
                    for x in 0..16 {
                        let wx = -27 * 16 + x;
                        let wz = -16 * 16 + z;
                        let pos = BlockPos(Vector3::new(wx, y, wz));
                        if p.get_block(&pos).is_air() {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    };
    {
        use pumpkin_world::generation::noise::router::chunk_density_function::ChunkNoiseFunctionBuilderOptions;
        use pumpkin_world::generation::noise::router::proto_noise_router::ProtoNoiseRouters;
        use pumpkin_data::noise_router::OVERWORLD_BASE_NOISE_ROUTER;
        use pumpkin_world::generation::GlobalRandomConfig;
        use pumpkin_world::generation::noise::router::chunk_noise_router::ChunkNoiseRouter;
        use pumpkin_world::generation::noise::router::density_volume::DensityVolume;

        let random_config = GlobalRandomConfig::new(1789322517659391064, false);
        let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, &random_config);
        let builder_options = ChunkNoiseFunctionBuilderOptions::new(Vec::new(), Vec::new(), None);
        let mut router1 = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);
        let mut router2 = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);

        // Grid volume for chunk (-27, -16)
        let volume = DensityVolume::with_block_step(16, 384, 16, -27 * 16, -64, -16 * 16);
        let mut buf_aot = vec![0.0f32; 16 * 384 * 16];
        let mut buf_interp = vec![0.0f32; 16 * 384 * 16];

        router1.final_density_volume(&mut buf_aot, &volume);
        
        // Force interpreted:
        // By calling final_density point by point or internal
        println!("Buffer length = {}", buf_aot.len());
        // Check at y=10, lz=0..15, lx=12:
        for lz in 0..16 {
            let idx = (12 * 16 + lz) * 384 + (10 + 64);
            let d_aot = buf_aot[idx];
            let pt_pos = Vector3::new(-27i32 * 16 + 12, 10, -16i32 * 16 + lz as i32);
            let d_pt = router2.final_density(&pt_pos);
            println!("lz={:2}: d_aot={:+.4}, d_pt={:+.4}, diff={:.6}", lz, d_aot, d_pt, (d_aot - d_pt).abs());
        }
    }
}



