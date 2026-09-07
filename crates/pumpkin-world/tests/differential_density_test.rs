use pumpkin_data::noise_router::{NETHER_BASE_NOISE_ROUTER, OVERWORLD_BASE_NOISE_ROUTER};
use pumpkin_world::generation::{
    GlobalRandomConfig,
    noise::router::{
        aot_noise_router::{evaluate_nether_final_density_volume, evaluate_overworld_final_density_volume},
        chunk_density_function::ChunkNoiseFunctionBuilderOptions,
        chunk_noise_router::{ChunkNoiseFunctionComponent, ChunkNoiseRouter},
        density_volume::DensityVolume,
        proto_noise_router::{IndependentProtoNoiseFunctionComponent, ProtoNoiseRouters},
    },
};

#[test]
fn test_nether_aot_density_differential_parity() {
    for seed in [0u64, 42, 12345, 999_888_777] {
        let random_config = GlobalRandomConfig::new(seed, true);
        let proto_routers = ProtoNoiseRouters::generate(&NETHER_BASE_NOISE_ROUTER, &random_config);
        let builder_options = ChunkNoiseFunctionBuilderOptions::new(Vec::new(), Vec::new(), None);

        let mut router = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);

        // Test multiple chunk coordinates
        for (chunk_x, chunk_z) in [(0, 0), (5, -3), (-10, 20)] {
            let volume = DensityVolume::with_block_step(16, 128, 16, chunk_x * 16, 0, chunk_z * 16);

            let mut legacy_buf = vec![0.0f32; volume.size_x * volume.size_y * volume.size_z];
            let mut aot_buf = vec![0.0f32; volume.size_x * volume.size_y * volume.size_z];

            // 1. Run legacy AST evaluation
            ChunkNoiseFunctionComponent::sample_volume_from_stack(
                &mut router.component_stack_mut()[..=12],
                &mut legacy_buf,
                &volume,
            );

            // 2. Run AOT static evaluation
            let (sampler, beardifier) = {
                let stack = router.component_stack_mut();
                let sampler = match &stack[4] {
                    ChunkNoiseFunctionComponent::Independent(
                        IndependentProtoNoiseFunctionComponent::InterpolatedNoise(s),
                    ) => s,
                    _ => unreachable!(),
                };
                let beardifier = match &stack[11] {
                    ChunkNoiseFunctionComponent::Chunk(
                        pumpkin_world::generation::noise::router::chunk_density_function::ChunkSpecificNoiseFunctionComponent::Beardifier(b),
                    ) => b.clone(),
                    _ => unreachable!(),
                };
                (sampler, beardifier)
            };

            evaluate_nether_final_density_volume(&sampler, &beardifier, &mut aot_buf, &volume);

            // 3. Verify exact parity across all 32,768 voxels
            assert_eq!(legacy_buf.len(), aot_buf.len());
            let mut max_diff = 0.0f32;
            for i in 0..legacy_buf.len() {
                let diff = (legacy_buf[i] - aot_buf[i]).abs();
                if diff > max_diff {
                    max_diff = diff;
                }
                assert!(
                    diff < 1e-5,
                    "Discrepancy at voxel {} (seed {}, chunk ({}, {})): legacy={}, aot={}, diff={}",
                    i, seed, chunk_x, chunk_z, legacy_buf[i], aot_buf[i], diff
                );
            }
            println!(
                "Seed {} chunk ({}, {}): verified {} voxels, max diff: {:e}",
                seed, chunk_x, chunk_z, legacy_buf.len(), max_diff
            );
        }
    }
}

#[test]
fn test_overworld_aot_density_differential_parity() {
    for seed in [0u64, 42, 12345, 999_888_777] {
        let random_config = GlobalRandomConfig::new(seed, true);
        let proto_routers = ProtoNoiseRouters::generate(&OVERWORLD_BASE_NOISE_ROUTER, &random_config);
        let builder_options = ChunkNoiseFunctionBuilderOptions::new(Vec::new(), Vec::new(), None);

        let mut router = ChunkNoiseRouter::generate(&proto_routers.noise, &builder_options);

        // Test 4 chunk coordinates across all seeds (16 total pairs)
        for (chunk_x, chunk_z) in [(0, 0), (3, -2), (-7, 12), (100, -200)] {
            let volume = DensityVolume::with_block_step(16, 384, 16, chunk_x * 16, -64, chunk_z * 16);

            let mut legacy_buf = vec![0.0f32; volume.size_x * volume.size_y * volume.size_z];
            let mut aot_buf = vec![0.0f32; volume.size_x * volume.size_y * volume.size_z];

            // 1. Run legacy AST evaluation
            ChunkNoiseFunctionComponent::sample_volume_from_stack(
                &mut router.component_stack_mut()[..=195],
                &mut legacy_buf,
                &volume,
            );

            // 2. Run AOT static evaluation
            let beardifier = match &router.component_stack_mut()[194] {
                ChunkNoiseFunctionComponent::Chunk(
                    pumpkin_world::generation::noise::router::chunk_density_function::ChunkSpecificNoiseFunctionComponent::Beardifier(b),
                ) => b.clone(),
                _ => unreachable!(),
            };

            evaluate_overworld_final_density_volume(
                router.component_stack_mut(),
                &beardifier,
                &mut aot_buf,
                &volume,
            );

            // 3. Verify exact parity across all 98,304 voxels
            assert_eq!(legacy_buf.len(), aot_buf.len());
            let mut max_diff = 0.0f32;
            for i in 0..legacy_buf.len() {
                let diff = (legacy_buf[i] - aot_buf[i]).abs();
                if diff > max_diff {
                    max_diff = diff;
                }
                assert!(
                    diff < 1e-4,
                    "Discrepancy at voxel {} (seed {}, chunk ({}, {})): legacy={}, aot={}, diff={}",
                    i, seed, chunk_x, chunk_z, legacy_buf[i], aot_buf[i], diff
                );
            }
            println!(
                "Overworld Seed {} chunk ({}, {}): verified {} voxels, max diff: {:e}",
                seed, chunk_x, chunk_z, legacy_buf.len(), max_diff
            );
        }
    }
}

