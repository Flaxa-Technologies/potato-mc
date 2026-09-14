use crate::block::blocks::redstone::block_receives_redstone_power;
use crate::block::registry::BlockActionResult;
use crate::block::{
    BlockBehaviour, CanPlaceAtArgs, ExplodeArgs, GetStateForNeighborUpdateArgs, NormalUseArgs,
    OnNeighborUpdateArgs, OnPlaceArgs, PathComputationType,
};
use crate::entity::EntityBase;
use crate::entity::player::Player;
use crate::world::World;
use pumpkin_data::BlockDirection;
use pumpkin_data::block_properties::Half;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag::Taggable;
use pumpkin_data::{Block, BlockState, BlockStateId, HorizontalFacingExt, tag};
use pumpkin_macros::pumpkin_block_from_tag;
use pumpkin_util::math::position::BlockPos;
use pumpkin_world::world::BlockFlags;
use std::sync::Arc;

type TrapDoorProperties = pumpkin_data::block_properties::OakTrapdoorLikeProperties;

fn toggle_trapdoor(player: &Player, world: &Arc<World>, block_pos: &BlockPos) {
    let (block, block_state) = world.get_block_and_state_id(block_pos);
    let mut trapdoor_props = TrapDoorProperties::from_state_id(block_state);
    trapdoor_props.open = !trapdoor_props.open;

    world.play_block_sound_expect(
        player,
        get_sound(block, trapdoor_props.open),
        SoundCategory::Blocks,
        *block_pos,
    );

    world.set_block_state(
        block_pos,
        trapdoor_props.to_state_id(block),
        BlockFlags::NOTIFY_LISTENERS,
    );
}

fn can_open_trapdoor(block: &Block) -> bool {
    if block == &Block::IRON_TRAPDOOR {
        return false;
    }
    true
}

fn get_sound(block: &Block, open: bool) -> Sound {
    if open {
        if block.has_tag(&tag::Block::MINECRAFT_WOODEN_TRAPDOORS) {
            Sound::BlockWoodenTrapdoorOpen
        } else if block == &Block::IRON_TRAPDOOR {
            Sound::BlockIronTrapdoorOpen
        } else {
            Sound::BlockCopperTrapdoorOpen
        }
    } else if block.has_tag(&tag::Block::MINECRAFT_WOODEN_TRAPDOORS) {
        Sound::BlockWoodenTrapdoorClose
    } else if block == &Block::IRON_TRAPDOOR {
        Sound::BlockIronTrapdoorClose
    } else {
        Sound::BlockCopperTrapdoorClose
    }
}

#[pumpkin_block_from_tag("minecraft:trapdoors")]
pub struct TrapDoorBlock;

impl BlockBehaviour for TrapDoorBlock {
    fn normal_use(&self, args: NormalUseArgs<'_>) -> BlockActionResult {
        {
            if !can_open_trapdoor(args.block) {
                return BlockActionResult::Pass;
            }

            toggle_trapdoor(args.player, args.world, args.position);

            BlockActionResult::Success
        }
    }

    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        let mut trapdoor_props = TrapDoorProperties::default(args.block);
        trapdoor_props.waterlogged = args.replacing.water_source();

        let powered = block_receives_redstone_power(args.world, args.position);
        let player_facing = args.player.get_entity().get_horizontal_facing();

        if let Some(horizontal_dir) = args.direction.to_horizontal_facing() {
            trapdoor_props.facing = horizontal_dir;
            trapdoor_props.half = if args.use_item_on.cursor_pos.y > 0.5 {
                Half::Top
            } else {
                Half::Bottom
            };
        } else {
            trapdoor_props.facing = player_facing.opposite();
            trapdoor_props.half = if args.direction == BlockDirection::Up {
                Half::Bottom
            } else {
                Half::Top
            };
        }

        trapdoor_props.powered = powered;
        trapdoor_props.open = powered;

        trapdoor_props.to_state_id(args.block)
    }

    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        // Vanilla TrapDoorBlock.canSurvive: floor/ceiling clicks are always valid;
        // side-face placement requires the adjacent block to have a sturdy face.
        let clicked_face = args
            .use_item_on
            .and_then(|u| pumpkin_data::BlockDirection::try_from(u.face.0).ok())
            .unwrap_or(BlockDirection::Up);
        match clicked_face {
            BlockDirection::Up | BlockDirection::Down => true,
            horizontal => {
                // The block behind the trapdoor (the one it attaches to) must have a sturdy face
                let support_pos = args.position.offset(horizontal.opposite().to_offset());
                let (_, support_state) = args.block_accessor.get_block_and_state(&support_pos);
                support_state.is_side_solid(horizontal)
            }
        }
    }

    fn on_neighbor_update(&self, args: OnNeighborUpdateArgs<'_>) {
        {
            let block_state = args.world.get_block_state(args.position);
            let mut trapdoor_props = TrapDoorProperties::from_state_id(block_state.id);
            let powered = block_receives_redstone_power(args.world, args.position);

            if powered != trapdoor_props.powered {
                trapdoor_props.powered = !trapdoor_props.powered;

                if powered != trapdoor_props.open {
                    trapdoor_props.open = trapdoor_props.powered;

                    args.world.play_block_sound(
                        get_sound(args.block, powered),
                        SoundCategory::Blocks,
                        *args.position,
                    );
                }
            }

            args.world.set_block_state(
                args.position,
                trapdoor_props.to_state_id(args.block),
                BlockFlags::NOTIFY_LISTENERS,
            );
        }
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        // Drop the trapdoor if its support block is removed (horizontal attachments only)
        let props = TrapDoorProperties::from_state_id(args.state_id);
        // The face the trapdoor is attached to is the opposite of its `facing` property.
        // `facing` points outward; the support block is in the facing.opposite() direction.
        let attach_dir = props.facing.to_block_direction().opposite();
        if args.direction == attach_dir {
            // neighbor_position is already the adjacent block position
            let (_, support_state) = args.world.get_block_and_state(args.neighbor_position);
            if !support_state.is_side_solid(props.facing.to_block_direction()) {
                return BlockStateId::AIR;
            }
        }
        args.state_id
    }

    fn is_pathfindable(&self, state: &BlockState, computation_type: PathComputationType) -> bool {
        let props = TrapDoorProperties::from_state_id(state.id);
        match computation_type {
            PathComputationType::Land | PathComputationType::Air => props.open,
            PathComputationType::Water => props.waterlogged,
        }
    }

    fn explode(&self, args: ExplodeArgs<'_>) {
        if !can_open_trapdoor(args.block) {
            return;
        }
        let (block, state_id) = args.world.get_block_and_state_id(args.position);
        let mut props = TrapDoorProperties::from_state_id(state_id);
        if props.powered {
            return;
        }
        props.open = !props.open;
        args.world.play_sound(
            get_sound(block, props.open),
            SoundCategory::Blocks,
            &args.position.to_f64(),
        );
        args.world.set_block_state(
            args.position,
            props.to_state_id(block),
            BlockFlags::NOTIFY_LISTENERS,
        );
    }
}
