use pumpkin_world::generation::feature_order::select_features;
use pumpkin_world::generation::feature::placed_features::PLACED_FEATURES;
use pumpkin_world::generation::GlobalRandomConfig;
use pumpkin_world::generation::proto_chunk::GenerationCache;
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::chunk_system::{generate_single_chunk, StagedChunkEnum, Chunk};
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};
use pumpkin_data::chunk::Biome;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::Block;
use pumpkin_util::world_seed::Seed;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

struct DummyPortal;
impl WorldPortalExt for DummyPortal {
    fn can_place_at(&self, _: &Block, _: &pumpkin_data::BlockState, _: &dyn pumpkin_world::world::BlockAccessor, _: &BlockPos) -> bool { true }
    fn mirror(&self, b: &Block, s: pumpkin_data::BlockStateId, m: pumpkin_data::Mirror) -> &'static pumpkin_data::BlockState { b.mirror(s, m) }
    fn rotate(&self, b: &Block, s: pumpkin_data::BlockStateId, r: pumpkin_data::Rotation) -> &'static pumpkin_data::BlockState { b.rotate(s, r) }
    fn spawn_mobs_for_chunk_generation(&self, _: &mut dyn pumpkin_world::generation::proto_chunk::GenerationCache, _: &'static Biome, _: i32, _: i32) {}
}

#[test]
fn test_trace_dirt_features_chunk_9_20() {
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
        -9,
        -20,
        StagedChunkEnum::Full,
    );
    match chunk {
        Chunk::Level(chunk_data) => {
            println!("Level chunk loaded, checking block at (7, 4, 1):");
            let state = chunk_data.section.get_block_absolute_y(7, 4, 1);
            println!("Block at (7, 4, 1): {:?}", state.map(|s| s.to_block().name));
        }
        Chunk::Proto(proto) => {
            let pos = BlockPos(Vector3::new(-9 * 16 + 7, 4, -20 * 16 + 1));
            println!("Proto block at {:?}: {:?}", pos, proto.get_block(&pos).name);
        }
    }
}

#[test]
fn test_print_step_9_features() {
    let biomes: Vec<u8> = (0..=255).collect();
    let features = select_features(&biomes, 9);
    println!("Step 9 total features: {}", features.len());
    for (idx, feat) in features {
        println!("  {:2}: {:?}", idx, feat);
    }
}
