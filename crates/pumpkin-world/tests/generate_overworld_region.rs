use std::time::Instant;

use pumpkin_config::chunk::AnvilChunkConfig;
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::BlockStateId;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector2::Vector2;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::chunk::format::anvil::AnvilChunkFile;
use pumpkin_world::chunk::io::file_manager::ChunkFileManager;
use pumpkin_world::chunk::io::{Dirtiable, FileIO};
use pumpkin_world::chunk::ChunkData;
use pumpkin_world::chunk_system::stage_cache::generate_batch_chunks;
use pumpkin_world::chunk_system::{Chunk, StagedChunkEnum};
use pumpkin_world::generation::get_world_gen;
use pumpkin_world::level::LevelFolder;
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};

struct DummyPortal;
impl WorldPortalExt for DummyPortal {
    fn can_place_at(
        &self,
        block: &pumpkin_data::Block,
        _state: &pumpkin_data::BlockState,
        block_accessor: &dyn BlockAccessor,
        block_pos: &BlockPos,
    ) -> bool {
        pumpkin_world::generation::block_predicate::WouldSurviveBlockPredicate::check_vegetation_survival(
            block,
            block_accessor,
            block_pos,
        )
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

#[tokio::test(flavor = "multi_thread")]
async fn test_generate_and_save_overworld_region() {
    let t0 = Instant::now();

    let mut workspace_root = std::env::current_dir().expect("Failed to get current dir");
    while !workspace_root.join("pumpkin.toml").exists() {
        if let Some(parent) = workspace_root.parent() {
            workspace_root = parent.to_path_buf();
        } else {
            break;
        }
    }

    let world_dir = workspace_root.join("world");
    let region_dir = world_dir
        .join("dimensions")
        .join("minecraft")
        .join("overworld")
        .join("region");

    std::fs::create_dir_all(&region_dir).expect("Failed to create region dir");

    let mca_path = region_dir.join("r.-1.-1.mca");
    let mca_bak = region_dir.join("r.-1.-1.mca.bak");

    if mca_path.exists() && !mca_bak.exists() {
        println!("Backing up old r.-1.-1.mca to r.-1.-1.mca.bak...");
        std::fs::copy(&mca_path, &mca_bak).expect("Failed to backup r.-1.-1.mca");
    }
    if mca_path.exists() {
        std::fs::remove_file(&mca_path).expect("Failed to remove old r.-1.-1.mca");
    }

    println!("[GEN] Initializing WorldGenerator for seed 1789322517659391064...");
    let generator = get_world_gen(
        Seed(1789322517659391064),
        Dimension::OVERWORLD,
        false,
        Vec::new(),
        String::new(),
    );

    let mut coords = Vec::with_capacity(1024);
    for cx in -32..0 {
        for cz in -32..0 {
            coords.push((cx, cz));
        }
    }

    println!("[GEN] Generating 1,024 chunks in parallel using generate_batch_chunks...");
    let t_gen = Instant::now();
    let chunks = generate_batch_chunks(
        &generator,
        &DummyPortal,
        &coords,
        StagedChunkEnum::Full,
    );
    println!(
        "[GEN] 1,024 chunks generated in {:.2}s",
        t_gen.elapsed().as_secs_f64()
    );

    println!("[GEN] Upgrading chunks to Level ChunkData and marking dirty...");
    let mut chunks_to_write = Vec::with_capacity(chunks.len());
    for (i, mut chunk) in chunks.into_iter().enumerate() {
        let (cx, cz) = coords[i];
        let pos = Vector2::new(cx, cz);
        match &mut chunk {
            Chunk::Level(level_chunk) => {
                level_chunk.mark_dirty(true);
                chunks_to_write.push((pos, level_chunk.clone()));
            }
            Chunk::Proto(_) => {
                chunk.upgrade_to_level_chunk(
                    generator.dimension(),
                    &LightingEngineConfig::Default,
                );
                let Chunk::Level(level_chunk) = chunk else {
                    panic!("Upgrade failed for ({cx}, {cz})");
                };
                level_chunk.mark_dirty(true);
                chunks_to_write.push((pos, level_chunk));
            }
        }
    }

    println!("[GEN] Saving 1,024 chunks to {}...", region_dir.display());
    let level_folder = LevelFolder {
        root_folder: world_dir.clone(),
        dim_folder: world_dir
            .join("dimensions")
            .join("minecraft")
            .join("overworld"),
        region_folder: region_dir.clone(),
        entities_folder: world_dir
            .join("dimensions")
            .join("minecraft")
            .join("overworld")
            .join("entities"),
        poi_folder: world_dir
            .join("dimensions")
            .join("minecraft")
            .join("overworld")
            .join("poi"),
    };

    let chunk_saver = ChunkFileManager::<AnvilChunkFile<ChunkData>>::new(AnvilChunkConfig::default());
    chunk_saver
        .save_chunks(&level_folder, chunks_to_write)
        .await
        .expect("Failed to save chunks via ChunkFileManager");

    println!(
        "[GEN] Successfully wrote r.-1.-1.mca! File size: {} bytes. Total elapsed: {:.2}s",
        std::fs::metadata(&mca_path).map(|m| m.len()).unwrap_or(0),
        t0.elapsed().as_secs_f64()
    );

    assert!(mca_path.exists(), "r.-1.-1.mca should exist");
    let file_size = std::fs::metadata(&mca_path).unwrap().len();
    assert!(file_size > 1_000_000, "r.-1.-1.mca should be substantial, got {} bytes", file_size);
}
