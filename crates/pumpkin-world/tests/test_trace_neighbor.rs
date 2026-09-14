use pumpkin_world::generation::get_world_gen;
use pumpkin_world::chunk_system::{stage_cache::generate_batch_chunks, StagedChunkEnum};
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
fn test_which_neighbor_writes_dirt() {
    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );
    
    // We want to test batch generation of (-9, -20) along with its neighbors
    // Let's generate a 3x3 of chunks centered at (-9, -20)
    let mut coords = Vec::new();
    for cx in -10..=-8 {
        for cz in -21..=-19 {
            coords.push((cx, cz));
        }
    }
    
    let chunks = generate_batch_chunks(&generator, &DummyPortal, &coords, StagedChunkEnum::Full);
    println!("Generated {} chunks", chunks.len());
    
    // Find chunk (-9, -20)
    for (i, &(cx, cz)) in coords.iter().enumerate() {
        if cx == -9 && cz == -20 {
            match &chunks[i] {
                pumpkin_world::chunk_system::Chunk::Level(chunk_data) => {
                    let state = chunk_data.section.get_block_absolute_y(7, 4, 1);
                    println!("Chunk::Level block in (-9, -20) at (7, 4, 1): {:?}", state.map(|s| s.to_block().name));
                }
                pumpkin_world::chunk_system::Chunk::Proto(proto) => {
                    let pos = BlockPos(Vector3::new(-9 * 16 + 7, 4, -20 * 16 + 1));
                    println!("Chunk::Proto block in (-9, -20) at {:?}: {:?}", pos, proto.get_block(&pos).name);
                }
            }
        }
    }
}
