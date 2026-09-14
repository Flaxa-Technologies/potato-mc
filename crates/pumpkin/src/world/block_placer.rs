use pumpkin_data::{BlockState, BlockStateId};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_world::generation::structure::template::BlockPlacer;
use pumpkin_world::level::Level;
use pumpkin_world::world::{BlockAccessor, WorldPortalExt};

use crate::world::World;

impl World {
    pub fn clear_synced_block_events_in_box(&self, min: &BlockPos, max: &BlockPos) {
        let mut events = self
            .synced_block_event_queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        events.retain(|event| {
            let pos = event.pos;
            pos.0.x < min.0.x
                || pos.0.x >= max.0.x
                || pos.0.y < min.0.y
                || pos.0.y >= max.0.y
                || pos.0.z < min.0.z
                || pos.0.z >= max.0.z
        });
    }
}

pub struct WorldBlockPlacer<'a> {
    world: &'a World,
    pub block_entity_nbts: Vec<NbtCompound>,
    pub changed_positions: Vec<(BlockPos, BlockStateId)>,
}

impl<'a> WorldBlockPlacer<'a> {
    #[must_use]
    pub const fn new(world: &'a World) -> Self {
        Self {
            world,
            block_entity_nbts: Vec::new(),
            changed_positions: Vec::new(),
        }
    }

    #[allow(clippy::unused_async)]
    pub fn finalize(&self) {
        for nbt in &self.block_entity_nbts {
            if let Some(block_entity) = crate::block::entities::block_entity_from_nbt(nbt) {
                self.world.add_block_entity(block_entity);
            }
        }
    }
}

impl BlockPlacer for WorldBlockPlacer<'_> {
    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId {
        self.world
            .get_block_state_id(&BlockPos::new(pos.x, pos.y, pos.z))
    }

    fn set_block_state(&mut self, pos: &Vector3<i32>, state: &BlockState) {
        let block_pos = BlockPos::new(pos.x, pos.y, pos.z);
        Level::set_block_state(&self.world.level, &block_pos, state.id);
        self.changed_positions.push((block_pos, state.id));
    }

    fn add_block_entity(&mut self, nbt: NbtCompound) {
        self.block_entity_nbts.push(nbt);
    }

    fn get_top_y(
        &self,
        _heightmap: pumpkin_world::generation::structure::template::processor::HeightmapType,
        x: i32,
        z: i32,
    ) -> i32 {
        let block_pos = BlockPos::new(x, 0, z);
        let chunk_pos = block_pos.chunk_position();
        let rel_x = (x.rem_euclid(16)) as usize;
        let rel_z = (z.rem_euclid(16)) as usize;
        let top_block_y = self
            .world
            .level
            .read_chunk_sync(&chunk_pos, |chunk| chunk.section.get_top_y(rel_x, rel_z, 319))
            .flatten();

        top_block_y.map_or(64, |y| y + 1)
    }
}

/// Minimal `WorldPortalExt` implementation for feature/tree generation.
pub struct CommandBlockRegistry;

impl WorldPortalExt for CommandBlockRegistry {
    fn can_place_at(
        &self,
        _block: &pumpkin_data::Block,
        _state: &pumpkin_data::BlockState,
        _block_accessor: &dyn BlockAccessor,
        _block_pos: &BlockPos,
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
    ) {
    }
}

pub struct WorldGenAdapter<'a> {
    pub world: &'a World,
    pub changed_positions: Vec<(BlockPos, BlockStateId)>,
}

impl<'a> WorldGenAdapter<'a> {
    #[must_use]
    pub const fn new(world: &'a World) -> Self {
        Self {
            world,
            changed_positions: Vec::new(),
        }
    }

    pub fn finalize(&self) {
        self.world.queue_block_updates(&self.changed_positions);
        self.world.flush_block_updates();
    }
}

impl pumpkin_world::generation::height_limit::HeightLimitView for WorldGenAdapter<'_> {
    fn height(&self) -> u16 {
        self.world.level.world_gen().dimension().height as u16
    }

    fn bottom_y(&self) -> i8 {
        self.world.level.world_gen().dimension().min_y as i8
    }
}

impl pumpkin_world::world::BlockAccessor for WorldGenAdapter<'_> {
    fn get_block(&self, position: &BlockPos) -> &'static pumpkin_data::Block {
        let id = self.world.get_block_state_id(position);
        id.to_block()
    }

    fn get_block_state(&self, position: &BlockPos) -> &'static pumpkin_data::BlockState {
        let id = self.world.get_block_state_id(position);
        pumpkin_data::BlockState::from_id(id)
    }

    fn get_block_state_id(&self, position: &BlockPos) -> BlockStateId {
        self.world.get_block_state_id(position)
    }

    fn get_block_and_state(&self, position: &BlockPos) -> (&'static pumpkin_data::Block, &'static pumpkin_data::BlockState) {
        let id = self.world.get_block_state_id(position);
        pumpkin_data::BlockState::from_id_with_block(id)
    }
}

impl pumpkin_world::generation::proto_chunk::GenerationCache for WorldGenAdapter<'_> {
    fn get_center_chunk_mut(&mut self) -> &mut pumpkin_world::generation::proto_chunk::ProtoChunk {
        unreachable!("Not called by tree generator")
    }

    fn get_center_chunk(&self) -> &pumpkin_world::generation::proto_chunk::ProtoChunk {
        unreachable!("Not called by tree generator")
    }

    fn get_world_seed(&self) -> u64 {
        self.world.level.world_gen().seed()
    }

    fn get_chunk_mut(&mut self, _chunk_x: i32, _chunk_z: i32) -> Option<&mut pumpkin_world::generation::proto_chunk::ProtoChunk> {
        None
    }

    fn get_chunk(&self, _chunk_x: i32, _chunk_z: i32) -> Option<&pumpkin_world::generation::proto_chunk::ProtoChunk> {
        None
    }

    fn try_get_proto_chunk(&self, _chunk_x: i32, _chunk_z: i32) -> Option<&pumpkin_world::generation::proto_chunk::ProtoChunk> {
        None
    }

    fn get_block_state(&self, pos: &Vector3<i32>) -> BlockStateId {
        self.world.get_block_state_id(&BlockPos::new(pos.x, pos.y, pos.z))
    }

    fn get_fluid_and_fluid_state(&self, _position: &Vector3<i32>) -> (pumpkin_data::fluid::Fluid, pumpkin_data::fluid::FluidState) {
        (
            pumpkin_data::fluid::Fluid::EMPTY,
            pumpkin_data::fluid::FluidState {
                height: 0.0,
                level: 0,
                is_empty: true,
                blast_resistance: 0.0,
                block_state_id: BlockStateId::AIR,
                is_still: false,
                is_source: false,
                falling: false,
            },
        )
    }

    fn set_block_state(&mut self, pos: &Vector3<i32>, block_state: &BlockState) {
        let block_pos = BlockPos::new(pos.x, pos.y, pos.z);
        Level::set_block_state(&self.world.level, &block_pos, block_state.id);
        self.changed_positions.push((block_pos, block_state.id));
    }

    fn add_block_entity(&mut self, _pos: &Vector3<i32>, _nbt: NbtCompound) {}

    fn top_motion_blocking_block_height_exclusive(&self, _x: i32, _z: i32) -> i32 {
        320
    }

    fn top_motion_blocking_block_no_leaves_height_exclusive(&self, _x: i32, _z: i32) -> i32 {
        320
    }

    fn get_top_y(&self, _heightmap: &pumpkin_util::HeightMap, _x: i32, _z: i32) -> i32 {
        320
    }

    fn top_block_height_exclusive(&self, _x: i32, _z: i32) -> i32 {
        320
    }

    fn top_block_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        self.top_block_height_exclusive(x, z)
    }

    fn ocean_floor_height_exclusive(&self, _x: i32, _z: i32) -> i32 {
        0
    }

    fn ocean_floor_wg_height_exclusive(&self, x: i32, z: i32) -> i32 {
        self.ocean_floor_height_exclusive(x, z)
    }

    fn is_air(&self, local_pos: &Vector3<i32>) -> bool {
        let id = self.world.get_block_state_id(&BlockPos::new(local_pos.x, local_pos.y, local_pos.z));
        pumpkin_data::BlockState::from_id(id).is_air()
    }

    fn get_biome_for_terrain_gen(&self, _x: i32, _y: i32, _z: i32) -> &'static pumpkin_data::chunk::Biome {
        &pumpkin_data::chunk::Biome::PLAINS
    }

    fn get_blending_data(
        &self,
        _chunk_x: i32,
        _chunk_z: i32,
    ) -> Option<&pumpkin_world::generation::blender::blending_data::BlendingData> {
        None
    }

    fn get_sea_level(&self) -> i32 {
        63
    }
}

