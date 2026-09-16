use pumpkin_data::BlockDirection;
use pumpkin_data::sound::{Sound, SoundCategory};

use crate::block::{
    BlockBehaviour, BlockMetadata, CanPlaceAtArgs, OnLandedUponArgs,
    UpdateEntityMovementAfterFallOnArgs, bounce_entity_after_fall,
};

/// Shelf Mushroom block introduced in Minecraft 26.3.
/// Attaches to horizontal wall faces and provides bouncy trampoline behavior
/// with 0.75 bounce restitution and 0.5 fall distance reduction.
pub struct ShelfMushroomBlock;

impl BlockMetadata for ShelfMushroomBlock {
    fn ids() -> Box<[pumpkin_data::BlockId]> {
        [].into()
    }
}

impl BlockBehaviour for ShelfMushroomBlock {
    fn can_place_at(&self, args: CanPlaceAtArgs<'_>) -> bool {
        // Can only survive if attached to a sturdy wall face
        let pos = *args.position;
        for dir in [
            BlockDirection::North,
            BlockDirection::South,
            BlockDirection::West,
            BlockDirection::East,
        ] {
            let support_pos = pos.offset(dir.to_offset());
            let support_state = args.block_accessor.get_block_state(&support_pos);
            if support_state.is_solid() {
                return true;
            }
        }
        false
    }

    fn on_landed_upon(&self, args: OnLandedUponArgs<'_>) {
        // Reduces fall damage by 50%
        if let Some(living) = args.entity.get_living_entity() {
            living.handle_fall_damage(args.entity, args.fall_distance, 0.5);
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
}

