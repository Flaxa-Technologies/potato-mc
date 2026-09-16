use pumpkin_data::BlockStateId;

use crate::block::{
    BlockBehaviour, BlockMetadata, CanPlaceAtArgs, GetStateForNeighborUpdateArgs,
    blocks::plant::PlantBlockBase,
};

/// Red Shrub block introduced in Minecraft 26.3.
/// Nether vegetation / bush block that can survive on nylium, netherrack, dirt, etc.
pub struct RedShrubBlock;

impl BlockMetadata for RedShrubBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [].into()
    }
}

impl BlockBehaviour for RedShrubBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        <Self as PlantBlockBase>::can_place_at(self, args.block_accessor, args.position)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        <Self as PlantBlockBase>::get_state_for_neighbor_update(
            self,
            args.world,
            args.position,
            args.state_id,
        )
    }
}

impl PlantBlockBase for RedShrubBlock {}
