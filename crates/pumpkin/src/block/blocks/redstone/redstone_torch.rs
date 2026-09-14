use std::sync::Arc;

use crate::block::BlockIsReplacing;
use crate::block::CanPlaceAtArgs;
use crate::block::EmitsRedstonePowerArgs;
use crate::block::GetRedstonePowerArgs;
use crate::block::GetStateForNeighborUpdateArgs;
use crate::block::OnNeighborUpdateArgs;
use crate::block::OnPlaceArgs;
use crate::block::OnScheduledTickArgs;
use crate::block::OnStateReplacedArgs;
use crate::block::PlacedArgs;
use crate::entity::EntityBase;
use pumpkin_data::Block;
use pumpkin_data::BlockDirection;
use pumpkin_data::BlockId;
use pumpkin_data::BlockStateId;
use pumpkin_data::FacingExt;
use pumpkin_data::HorizontalFacingExt;
use pumpkin_data::block_properties::Facing;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::tick::TickPriority;
use pumpkin_world::world::BlockAccessor;
use pumpkin_world::world::BlockFlags;

type RWallTorchProps = pumpkin_data::block_properties::FurnaceLikeProperties;
type RTorchProps = pumpkin_data::block_properties::RedstoneOreLikeProperties;

use crate::block::{BlockBehaviour, BlockMetadata};
use crate::world::World;

use super::get_redstone_power;

pub struct RedstoneTorchBlock;

impl BlockMetadata for RedstoneTorchBlock {
    fn ids() -> Box<[BlockId]> {
        [BlockId::REDSTONE_TORCH, BlockId::REDSTONE_WALL_TORCH].into()
    }
}

impl RedstoneTorchBlock {
    #[must_use]
    pub fn torch_weak_power(
        block: &Block,
        state_id: BlockStateId,
        direction: BlockDirection,
    ) -> u8 {
        if block == &Block::REDSTONE_WALL_TORCH {
            let props = RWallTorchProps::from_state_id(state_id);
            if props.lit && direction != props.facing.to_block_direction() {
                return 15;
            }
        } else if block == &Block::REDSTONE_TORCH {
            let props = RTorchProps::from_state_id(state_id);
            if props.lit && direction != BlockDirection::Up {
                return 15;
            }
        }
        0
    }

    #[must_use]
    pub fn torch_strong_power(
        block: &Block,
        state_id: BlockStateId,
        direction: BlockDirection,
    ) -> u8 {
        if direction == BlockDirection::Down {
            if block == &Block::REDSTONE_WALL_TORCH {
                let props = RWallTorchProps::from_state_id(state_id);
                if props.lit {
                    return 15;
                }
            } else if block == &Block::REDSTONE_TORCH {
                let props = RTorchProps::from_state_id(state_id);
                if props.lit {
                    return 15;
                }
            }
        }
        0
    }
}

impl BlockBehaviour for RedstoneTorchBlock {
    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        let world = args.world;
        let block = args.block;
        let location = args.position;

        if args.direction == BlockDirection::Down {
            let support_block = world.get_block_state(&location.down());
            if support_block.is_center_solid(BlockDirection::Up) {
                return block.default_state.id;
            }
        }
        let mut directions = args.player.get_entity().get_entity_facing_order();

        if args.replacing == BlockIsReplacing::None {
            let face = args.direction.to_facing();
            let mut i = 0;
            while i < directions.len() && directions[i] != face {
                i += 1;
            }

            if i > 0 {
                directions.copy_within(0..i, 1);
                directions[0] = face;
            }
        } else if directions[0] == Facing::Down {
            let support_block = world.get_block_state(&location.down());
            if support_block.is_center_solid(BlockDirection::Up) {
                return block.default_state.id;
            }
        }

        for dir in directions {
            if dir != Facing::Up
                && dir != Facing::Down
                && can_place_at(world, location, dir.to_block_direction())
            {
                let mut torch_props = RWallTorchProps::default(&Block::REDSTONE_WALL_TORCH);
                if let Some(facing) = dir.opposite().to_horizontal_facing() {
                    torch_props.facing = facing;
                    return torch_props.to_state_id(&Block::REDSTONE_WALL_TORCH);
                }
            }
        }

        let support_block = world.get_block_state(&location.down());
        if support_block.is_center_solid(BlockDirection::Up) {
            block.default_state.id
        } else {
            BlockStateId::AIR
        }
    }

    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        let support_block = args.block_accessor.get_block_state(&args.position.down());
        if support_block.is_center_solid(BlockDirection::Up) {
            return true;
        }
        for dir in BlockDirection::horizontal() {
            if can_place_at(args.block_accessor, args.position, dir.to_block_direction()) {
                return true;
            }
        }
        false
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        if args.block == &Block::REDSTONE_WALL_TORCH {
            let props = RWallTorchProps::from_state_id(args.state_id);
            if props.facing.to_block_direction().opposite() == args.direction
                && !can_place_at(
                    args.world,
                    args.position,
                    props.facing.to_block_direction().opposite(),
                )
            {
                return BlockStateId::AIR;
            }
        } else if args.direction == BlockDirection::Down {
            let support_block = args.world.get_block_state(&args.position.down());
            if !support_block.is_center_solid(BlockDirection::Up) {
                return BlockStateId::AIR;
            }
        }
        args.state_id
    }

    fn on_neighbor_update(&self, args: OnNeighborUpdateArgs<'_>) {
        {
            let state = args.world.get_block_state(args.position);

            if args
                .world
                .is_block_tick_scheduled(args.position, args.block)
            {
                return;
            }

            if args.block == &Block::REDSTONE_WALL_TORCH {
                let props = RWallTorchProps::from_state_id(state.id);
                if props.lit
                    != should_be_lit(
                        args.world,
                        args.position,
                        props.facing.to_block_direction().opposite(),
                    )
                {
                    args.world.schedule_block_tick(
                        args.block,
                        *args.position,
                        2,
                        TickPriority::Normal,
                    );
                }
            } else if args.block == &Block::REDSTONE_TORCH {
                let props = RTorchProps::from_state_id(state.id);
                if props.lit != should_be_lit(args.world, args.position, BlockDirection::Down) {
                    args.world.schedule_block_tick(
                        args.block,
                        *args.position,
                        2,
                        TickPriority::Normal,
                    );
                }
            }
        }
    }

    fn emits_redstone_power(&self, _args: EmitsRedstonePowerArgs<'_>) -> bool {
        true
    }

    fn get_weak_redstone_power(&self, args: GetRedstonePowerArgs<'_>) -> u8 {
        Self::torch_weak_power(args.block, args.state.id, args.direction)
    }

    fn get_strong_redstone_power(&self, args: GetRedstonePowerArgs<'_>) -> u8 {
        Self::torch_strong_power(args.block, args.state.id, args.direction)
    }

    fn on_scheduled_tick(&self, args: OnScheduledTickArgs<'_>) {
        let (block, state) = args.world.get_block_and_state(args.position);
        if block == &Block::REDSTONE_WALL_TORCH {
            let mut props = RWallTorchProps::from_state_id(state.id);
            let should_be_lit_now = should_be_lit(
                args.world,
                args.position,
                props.facing.to_block_direction().opposite(),
            );
            if props.lit != should_be_lit_now {
                props.lit = should_be_lit_now;
                args.world.set_block_state(
                    args.position,
                    props.to_state_id(block),
                    BlockFlags::NOTIFY_ALL,
                );
                update_neighbors(args.world, args.position);
            }
        } else if block == &Block::REDSTONE_TORCH {
            let mut props = RTorchProps::from_state_id(state.id);
            let should_be_lit_now = should_be_lit(args.world, args.position, BlockDirection::Down);
            if props.lit != should_be_lit_now {
                props.lit = should_be_lit_now;
                args.world.set_block_state(
                    args.position,
                    props.to_state_id(block),
                    BlockFlags::NOTIFY_ALL,
                );
                update_neighbors(args.world, args.position);
            }
        }
    }

    fn placed(&self, args: PlacedArgs<'_>) {
        update_neighbors(args.world, args.position);
    }

    fn on_state_replaced(&self, args: OnStateReplacedArgs<'_>) {
        update_neighbors(args.world, args.position);
    }
}

#[must_use]
pub fn torch_should_be_lit_from_power(power: u8) -> bool {
    power == 0
}

pub fn should_be_lit_with_power_fn<F>(pos: &BlockPos, face: BlockDirection, get_power: F) -> bool
where
    F: FnOnce(&BlockPos, BlockDirection) -> u8,
{
    let other_pos = pos.offset(face.to_offset());
    torch_should_be_lit_from_power(get_power(&other_pos, face))
}

pub fn should_be_lit(world: &World, pos: &BlockPos, face: BlockDirection) -> bool {
    should_be_lit_with_power_fn(pos, face, |other_pos, f| {
        let (block, state) = world.get_block_and_state(other_pos);
        get_redstone_power(block, state, world, other_pos, f)
    })
}

pub fn update_neighbors(world: &Arc<World>, pos: &BlockPos) {
    for dir in BlockDirection::all() {
        let other_pos = pos.offset(dir.to_offset());
        world.update_neighbors(&other_pos, None);
    }
}

fn can_place_at(world: &dyn BlockAccessor, block_pos: &BlockPos, facing: BlockDirection) -> bool {
    world
        .get_block_state(&block_pos.offset(facing.to_offset()))
        .is_side_solid(facing.opposite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_data::block_properties::HorizontalFacing;
    use pumpkin_data::{Block, BlockDirection};
    use pumpkin_util::math::position::BlockPos;

    #[test]
    fn test_torch_inversion_logic() {
        // Redstone torch unpowers when the attached block is powered, and lights up when unpowered.
        assert!(
            torch_should_be_lit_from_power(0),
            "Torch must be lit when attached block has power 0"
        );
        for p in 1..=15 {
            assert!(
                !torch_should_be_lit_from_power(p),
                "Torch must unpower when attached block has power {p}"
            );
        }

        let pos = BlockPos::new(0, 64, 0);
        // Face Down represents standing torch attached to block below
        assert!(
            should_be_lit_with_power_fn(&pos, BlockDirection::Down, |_, _| 0),
            "Standing torch on unpowered block must be lit"
        );
        assert!(
            !should_be_lit_with_power_fn(&pos, BlockDirection::Down, |_, _| 15),
            "Standing torch on strongly/weakly powered block must turn off"
        );
    }

    #[test]
    fn test_standing_torch_strong_power_upwards_only() {
        let mut lit_props = RTorchProps::from_state_id(Block::REDSTONE_TORCH.default_state.id);
        lit_props.lit = true;
        let lit_state_id = lit_props.to_state_id(&Block::REDSTONE_TORCH);

        // When block above queries strong power, args.direction is BlockDirection::Down
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::Down
            ),
            15,
            "Standing torch must strongly power the block above with power 15"
        );

        // All other directions must receive 0 strong power
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::Up
            ),
            0,
            "Torch must not strongly power block below"
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::North
            ),
            0,
            "Torch must not strongly power horizontal neighbors"
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::South
            ),
            0
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::East
            ),
            0
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::West
            ),
            0
        );

        // Unlit torch emits 0 strong power in all directions
        let mut unlit_props = RTorchProps::from_state_id(Block::REDSTONE_TORCH.default_state.id);
        unlit_props.lit = false;
        let unlit_state_id = unlit_props.to_state_id(&Block::REDSTONE_TORCH);
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_TORCH,
                unlit_state_id,
                BlockDirection::Down
            ),
            0,
            "Unlit torch must emit 0 strong power"
        );
    }

    #[test]
    fn test_standing_torch_weak_power() {
        let mut lit_props = RTorchProps::from_state_id(Block::REDSTONE_TORCH.default_state.id);
        lit_props.lit = true;
        let lit_state_id = lit_props.to_state_id(&Block::REDSTONE_TORCH);

        // Weak power: emits 15 in all directions except Up (the block below querying it)
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::Down
            ),
            15,
            "Weak power above must be 15"
        );
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::North
            ),
            15
        );
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::South
            ),
            15
        );
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::East
            ),
            15
        );
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::West
            ),
            15
        );
        assert_eq!(
            RedstoneTorchBlock::torch_weak_power(
                &Block::REDSTONE_TORCH,
                lit_state_id,
                BlockDirection::Up
            ),
            0,
            "Torch must NOT emit weak power into the block below"
        );
    }

    #[test]
    fn test_wall_torch_strong_power_upwards_only() {
        let mut lit_props =
            RWallTorchProps::from_state_id(Block::REDSTONE_WALL_TORCH.default_state.id);
        lit_props.lit = true;
        lit_props.facing = HorizontalFacing::North;
        let lit_state_id = lit_props.to_state_id(&Block::REDSTONE_WALL_TORCH);

        // Wall torch strongly powers the block above
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_WALL_TORCH,
                lit_state_id,
                BlockDirection::Down
            ),
            15,
            "Wall torch must strongly power the block above"
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_WALL_TORCH,
                lit_state_id,
                BlockDirection::North
            ),
            0
        );
        assert_eq!(
            RedstoneTorchBlock::torch_strong_power(
                &Block::REDSTONE_WALL_TORCH,
                lit_state_id,
                BlockDirection::South
            ),
            0
        );
    }
}
