use pumpkin_data::block_properties::{PotentSulfurProperties, PotentSulfurState};
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::{Block, BlockStateId};
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockAccessor;

use crate::block::{
    BlockBehaviour, GetStateForNeighborUpdateArgs, OnPlaceArgs, PlacedArgs,
};

#[pumpkin_block("minecraft:potent_sulfur")]
pub struct PotentSulfurBlock;

impl PotentSulfurBlock {
    pub fn valid_block_state(
        world: &dyn BlockAccessor,
        pos: &BlockPos,
        current_state_id: BlockStateId,
    ) -> BlockStateId {
        let above_state = world.get_block_state(&pos.up());
        if above_state.id.to_block_id() != Block::WATER.id {
            let mut props = PotentSulfurProperties::from_state_id(current_state_id);
            props.r#potent_sulfur_state = PotentSulfurState::Dry;
            return props.to_state_id(&Block::POTENT_SULFUR);
        }

        let below_state = world.get_block_state(&pos.down());
        let below_id = below_state.id.to_block_id();

        if below_id == Block::LAVA.id {
            let mut props = PotentSulfurProperties::from_state_id(current_state_id);
            props.r#potent_sulfur_state = PotentSulfurState::Continuous;
            return props.to_state_id(&Block::POTENT_SULFUR);
        }

        if below_id == Block::MAGMA_BLOCK.id {
            let mut props = PotentSulfurProperties::from_state_id(current_state_id);
            if props.r#potent_sulfur_state == PotentSulfurState::Erupting {
                return current_state_id;
            }
            props.r#potent_sulfur_state = PotentSulfurState::Dormant;
            return props.to_state_id(&Block::POTENT_SULFUR);
        }

        let mut props = PotentSulfurProperties::from_state_id(current_state_id);
        props.r#potent_sulfur_state = PotentSulfurState::Wet;
        props.to_state_id(&Block::POTENT_SULFUR)
    }
}

impl BlockBehaviour for PotentSulfurBlock {
    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        Self::valid_block_state(args.world, args.position, args.block.default_state.id)
    }

    fn get_state_for_neighbor_update(&self, args: GetStateForNeighborUpdateArgs<'_>) -> BlockStateId {
        Self::valid_block_state(args.world, args.position, args.state_id)
    }

    fn placed(&self, args: PlacedArgs<'_>) {
        let props = PotentSulfurProperties::from_state_id(args.state_id);
        if props.r#potent_sulfur_state == PotentSulfurState::Erupting
            || props.r#potent_sulfur_state == PotentSulfurState::Continuous
        {
            args.world.play_sound(
                if props.r#potent_sulfur_state == PotentSulfurState::Continuous {
                    Sound::BlockPotentSulfurGeyserContinuousEruption
                } else {
                    Sound::BlockPotentSulfurGeyserEruption
                },
                SoundCategory::Blocks,
                &args.position.to_f64(),
            );
        }
    }
}
