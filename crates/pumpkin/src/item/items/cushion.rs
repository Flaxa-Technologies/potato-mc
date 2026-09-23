use std::sync::Arc;

use crate::entity::Entity;
use crate::entity::decoration::cushion::CushionEntity;
use crate::entity::player::Player;
use crate::item::{ItemBehaviour, ItemMetadata};
use crate::server::Server;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::{Block, BlockDirection};
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

pub struct CushionItem;

impl ItemMetadata for CushionItem {
    fn ids() -> Box<[u16]> {
        // All 16 cushion item IDs in internal palette (1626..=1641)
        (1626..=1641).collect::<Vec<u16>>().into_boxed_slice()
    }
}

impl CushionItem {
    /// Maps internal cushion item ID (1626..=1641) to color index (0..15).
    #[must_use]
    pub const fn item_id_to_color(item_id: u16) -> u8 {
        if item_id >= 1626 && item_id <= 1641 {
            (item_id - 1626) as u8
        } else {
            0
        }
    }
}

impl ItemBehaviour for CushionItem {
    fn use_on_block(
        &self,
        item: &mut ItemStack,
        player: &Player,
        location: BlockPos,
        face: BlockDirection,
        _cursor_pos: Vector3<f32>,
        _block: &Block,
        _server: &Server,
    ) {
        if face != BlockDirection::Up {
            return;
        }

        let world = player.world();
        let target_pos = location.offset(face.to_offset());
        let bottom_center = Vector3::new(
            f64::from(target_pos.0.x) + 0.5,
            f64::from(target_pos.0.y),
            f64::from(target_pos.0.z) + 0.5,
        );

        let cushion_dimensions = EntityType::CUSHION.dimension;
        let width = f64::from(cushion_dimensions[0]);
        let height = f64::from(cushion_dimensions[1]);

        let bounding_box = BoundingBox::new(
            Vector3::new(
                bottom_center.x - width / 2.0,
                bottom_center.y,
                bottom_center.z - width / 2.0,
            ),
            Vector3::new(
                bottom_center.x + width / 2.0,
                bottom_center.y + height,
                bottom_center.z + width / 2.0,
            ),
        );

        if world.is_space_empty(bounding_box) && world.get_entities_at_box(&bounding_box).is_empty() {
            let entity = Entity::new(world.clone(), bottom_center, &EntityType::CUSHION);
            let color = Self::item_id_to_color(item.item.id);

            world.play_sound(
                Sound::EntityCushionPlace,
                SoundCategory::Blocks,
                &entity.pos.load(),
            );

            let cushion = CushionEntity::new(entity, target_pos, color);
            world.spawn_entity(Arc::new(cushion));
            item.decrement_unless_creative(player.gamemode.load(), 1);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
