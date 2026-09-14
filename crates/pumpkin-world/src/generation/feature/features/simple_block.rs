use pumpkin_data::Block;
use pumpkin_util::{math::position::BlockPos, random::RandomGenerator};

use crate::generation::proto_chunk::GenerationCache;
use crate::{
    generation::block_state_provider::BlockStateProvider,
    world::{BlockAccessor, WorldPortalExt},
};

pub struct SimpleBlockFeature {
    pub to_place: BlockStateProvider,
    pub schedule_tick: Option<bool>,
}

impl SimpleBlockFeature {
    pub fn generate<T: GenerationCache>(
        &self,
        block_registry: &dyn WorldPortalExt,
        chunk: &mut T,
        random: &mut RandomGenerator,
        pos: BlockPos,
    ) -> bool {
        let state = self.to_place.get(random, pos, chunk, block_registry);
        let block = Block::from_state_id(state.id);
        let block_accessor: &dyn BlockAccessor = chunk;
        if !block_registry.can_place_at(block, state, block_accessor, &pos) {
            return false;
        }

        let is_double_plant = matches!(
            block.name,
            "tall_grass"
                | "large_fern"
                | "sunflower"
                | "lilac"
                | "peony"
                | "rose_bush"
                | "pitcher_plant"
        );
        if is_double_plant {
            let up_pos = pos.up();
            let up_block = chunk.get_block(&up_pos);
            if !up_block.is_air() {
                return false;
            }
            chunk.set_block_state(&pos.0, state);
            let upper_state =
                pumpkin_data::BlockStateId::new_or_air(state.id.as_u16().saturating_sub(1))
                    .to_state();
            chunk.set_block_state(&up_pos.0, upper_state);
        } else {
            chunk.set_block_state(&pos.0, state);
        }
        true
    }
}
