use pumpkin_data::tag::Taggable;
use pumpkin_data::{Block, BlockId, BlockStateId, tag};
use pumpkin_world::world::{BlockAccessor, BlockFlags};

use crate::block::{
    BlockBehaviour, BlockMetadata, BonemealArgs, CanPlaceAtArgs, GetStateForNeighborUpdateArgs,
    blocks::plant::PlantBlockBase,
};

/// Red Shrub block introduced in Minecraft 26.3.
/// Vegetation / shrub block in dappled forest and Nether that can survive on soil, nylium, etc.
/// Bone meal grows another shrub in a random adjacent space.
pub struct RedShrubBlock;

impl BlockMetadata for RedShrubBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [BlockId::RED_SHRUB].into()
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

    fn is_valid_bonemeal_target(&self, _args: BonemealArgs<'_>) -> bool {
        true
    }

    fn perform_bonemeal(&self, args: BonemealArgs<'_>) {
        let offsets = [
            (-1, -1), (-1, 0), (-1, 1),
            (0, -1),           (0, 1),
            (1, -1),  (1, 0),  (1, 1),
        ];
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        let mut candidates = offsets;
        candidates.shuffle(&mut rng);

        for (dx, dz) in candidates {
            let target_pos = pumpkin_util::math::position::BlockPos(
                pumpkin_util::math::vector3::Vector3::new(
                    args.position.0.x + dx,
                    args.position.0.y,
                    args.position.0.z + dz,
                ),
            );
            if args.world.get_block_state(&target_pos).replaceable()
                && <Self as PlantBlockBase>::can_place_at(self, args.world.as_ref(), &target_pos)
            {
                args.world.set_block_state(
                    &target_pos,
                    Block::RED_SHRUB.default_state.id,
                    BlockFlags::NOTIFY_ALL,
                );
                break;
            }
        }
    }
}

impl PlantBlockBase for RedShrubBlock {
    fn can_plant_on_top(&self, block_accessor: &dyn BlockAccessor, pos: &pumpkin_util::math::position::BlockPos) -> bool {
        let block = block_accessor.get_block(pos);
        block.has_tag(&tag::Block::MINECRAFT_SUPPORTS_VEGETATION)
            || block.has_tag(&tag::Block::MINECRAFT_NYLIUM)
            || block.has_tag(&tag::Block::MINECRAFT_DIRT)
            || block.id == BlockId::NETHERRACK
    }
}

