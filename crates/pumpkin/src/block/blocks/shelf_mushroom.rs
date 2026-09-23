use pumpkin_data::block_properties::{Facing, ShelfMushroomProperties};
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::{Block, BlockDirection, BlockId, BlockStateId, FacingExt};
use pumpkin_world::world::BlockFlags;

use crate::block::{
    BlockBehaviour, BlockMetadata, BonemealArgs, CanPlaceAtArgs, GetStateForNeighborUpdateArgs,
    OnLandedUponArgs, OnPlaceArgs, UpdateEntityMovementAfterFallOnArgs, bounce_entity_after_fall,
};
use crate::entity::EntityBase;

/// Shelf Mushroom block introduced in Minecraft 26.3.
/// Attaches to horizontal wall faces and provides bouncy trampoline behavior
/// with 0.75 bounce restitution and 50% fall damage reduction.
/// Small size (age=0) grows to large (age=1) with bone meal; large drops 2 items.
pub struct ShelfMushroomBlock;

impl BlockMetadata for ShelfMushroomBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [BlockId::SHELF_MUSHROOM].into()
    }
}

impl BlockBehaviour for ShelfMushroomBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        let pos = *args.position;
        for dir in BlockDirection::horizontal() {
            let support_pos = pos.offset(dir.to_offset());
            let support_state = args.block_accessor.get_block_state(&support_pos);
            if support_state.is_solid() {
                return true;
            }
        }
        false
    }

    fn on_place(&self, args: OnPlaceArgs<'_>) -> BlockStateId {
        let mut props = ShelfMushroomProperties::default(args.block);
        props.age = 0;

        let directions = args.player.get_entity().get_entity_facing_order();
        for dir in directions {
            if dir == Facing::Up || dir == Facing::Down {
                continue;
            }
            let block_dir = dir.to_block_direction();
            let support_pos = args.position.offset(block_dir.to_offset());
            if args.world.get_block_state(&support_pos).is_solid() {
                if let Some(facing) = dir.opposite().to_horizontal_facing() {
                    props.facing = facing;
                    return props.to_state_id(args.block);
                }
            }
        }

        // Fallback: check all horizontal directions
        for dir in BlockDirection::horizontal() {
            let support_pos = args.position.offset(dir.to_offset());
            if args.world.get_block_state(&support_pos).is_solid() {
                props.facing = dir.opposite();
                return props.to_state_id(args.block);
            }
        }

        props.to_state_id(args.block)
    }

    fn get_state_for_neighbor_update(
        &self,
        args: GetStateForNeighborUpdateArgs<'_>,
    ) -> BlockStateId {
        let props = ShelfMushroomProperties::from_state_id(args.state_id);
        let support_pos = args.position.offset(props.facing.opposite().to_offset());
        let support_state = args.world.get_block_state(&support_pos);
        if !support_state.is_solid() {
            Block::AIR.default_state.id
        } else {
            args.state_id
        }
    }

    fn on_landed_upon(&self, args: OnLandedUponArgs<'_>) {
        // Reduces fall damage by 50%
        if let Some(living) = args.entity.get_living_entity() {
            living.handle_fall_damage(args.entity, args.fall_distance * 0.5, 1.0);
        }
    }

    fn update_entity_movement_after_fall_on(
        &self,
        args: UpdateEntityMovementAfterFallOnArgs<'_>,
    ) {
        // Bounce restitution: 0.75
        bounce_entity_after_fall(args.entity, 0.75);
        let base_entity = args.entity.get_entity();
        let world = base_entity.world.load();
        let pos = base_entity.pos.load();
        world.play_sound(Sound::BlockShelfMushroomBounce, SoundCategory::Blocks, &pos);
    }

    fn is_valid_bonemeal_target(&self, args: BonemealArgs<'_>) -> bool {
        let props = ShelfMushroomProperties::from_state_id(args.state_id);
        props.age == 0
    }

    fn perform_bonemeal(&self, args: BonemealArgs<'_>) {
        let mut props = ShelfMushroomProperties::from_state_id(args.state_id);
        if props.age == 0 {
            props.age = 1;
            args.world.set_block_state(
                args.position,
                props.to_state_id(args.block),
                BlockFlags::NOTIFY_ALL,
            );
        }
    }
}

