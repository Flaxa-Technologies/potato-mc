use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use pumpkin_data::damage::DamageType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::living::LivingEntity;
use crate::entity::player::Player;
use crate::entity::{Entity, EntityBase};

/// Cushion decoration entity introduced in Minecraft 26.3.
/// Players can sit on it (mounting as a vehicle).
pub struct CushionEntity {
    pub entity: Entity,
    pub attached_pos: BlockPos,
    pub color: AtomicU8,
}

impl CushionEntity {
    pub fn new(entity: Entity, attached_pos: BlockPos, color: u8) -> Self {
        // Tracked data index for cushion color is defined in 26.3 tracked data
        entity.set_synced_data(
            pumpkin_data::tracked_data::cushion::COLOR,
            VarInt(i32::from(color)),
        );
        Self {
            entity,
            attached_pos,
            color: AtomicU8::new(color),
        }
    }

    pub fn get_color(&self) -> u8 {
        self.color.load(Ordering::Relaxed)
    }

    pub fn set_color(&self, color: u8) {
        self.color.store(color, Ordering::Relaxed);
        self.entity.set_synced_data(
            pumpkin_data::tracked_data::cushion::COLOR,
            VarInt(i32::from(color)),
        );
    }

    /// Handles right-click interaction: mounts the player onto the cushion.
    pub fn on_interact(&self, player: &Arc<Player>) -> bool {
        if player.living_entity.entity.sneaking.load(Ordering::Relaxed) {
            return false;
        }

        if self.entity.has_passengers() {
            return false;
        }

        let world = self.entity.world.load();
        let pos = self.entity.pos.load();
        let Some(vehicle) = world.get_entity_by_id(self.entity.entity_id) else {
            return false;
        };
        let Some(passenger) = world.get_player_by_id(player.entity_id()) else {
            return false;
        };

        world.play_sound(Sound::EntityCushionSit, SoundCategory::Neutral, &pos);
        self.entity.add_passenger(vehicle, passenger as Arc<dyn EntityBase>);
        true
    }

    /// Called when a passenger dismounts from the cushion.
    pub fn on_passenger_removed(&self) {
        if !self.entity.is_removed() {
            let world = self.entity.world.load();
            let pos = self.entity.pos.load();
            world.play_sound(Sound::EntityCushionGetUp, SoundCategory::Neutral, &pos);
        }
    }

    /// Drops the cushion item on break.
    pub fn drop_item(&self) {
        let world = self.entity.world.load();
        let pos = self.entity.pos.load();
        world.play_sound(Sound::EntityCushionBreak, SoundCategory::Neutral, &pos);
        let color_idx = (self.get_color() as usize).min(15) as u16;
        let item_id = 1626 + color_idx;
        if let Some(item) = Item::from_id(item_id) {
            let block_pos = BlockPos(Vector3::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32));
            world.drop_stack(&block_pos, ItemStack::new(1, item));
        }
    }
}

impl EntityBase for CushionEntity {
    fn get_entity(&self) -> &Entity {
        &self.entity
    }

    fn get_living_entity(&self) -> Option<&LivingEntity> {
        None
    }

    fn interact(&self, player: &Arc<Player>, _item_stack: &mut ItemStack) -> bool {
        self.on_interact(player)
    }

    fn damage_with_context(
        &self,
        _caller: &dyn EntityBase,
        _amount: f32,
        _damage_type: DamageType,
        _position: Option<Vector3<f64>>,
        _source: Option<&dyn EntityBase>,
        _cause: Option<&dyn EntityBase>,
    ) -> bool {
        self.drop_item();
        self.entity.remove();
        true
    }

    fn cast_any(&self) -> &dyn std::any::Any {
        self
    }
}
