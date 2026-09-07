use std::sync::Arc;

use crate::entity::player::Player;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_util::math::vector3::Vector3;

use crate::entity::Entity;
use crate::entity::EntityBase;
use crate::entity::projectile::ThrownItemEntity;
use crate::entity::projectile::wind_charge::{WIND_CHARGE_GRAVITY, WindChargeEntity};
use crate::item::{ItemBehaviour, ItemMetadata};

pub struct WindChargeItem;

impl ItemMetadata for WindChargeItem {
    fn ids() -> Box<[u16]> {
        [Item::WIND_CHARGE.id].into()
    }
}

const POWER: f32 = 1.5;

impl ItemBehaviour for WindChargeItem {
    fn normal_use(&self, _block: &Item, player: &Player) {
        if player.get_cooldown("minecraft:wind_charge") > 0.0 {
            return;
        }
        player.start_cooldown("minecraft:wind_charge".to_string(), 10);

        let world = player.world();
        let eye_pos = player.eye_position();
        let spawn_pos = Vector3::new(player.position().x, eye_pos.y, player.position().z);

        let rand_pitch = 0.4 / (rand::random::<f32>() * 0.4 + 0.8);
        world.play_sound_fine(
            Sound::EntityWindChargeThrow,
            SoundCategory::Neutral,
            &spawn_pos,
            0.5,
            rand_pitch,
        );

        let entity = Entity::new(world.clone(), spawn_pos, &EntityType::WIND_CHARGE);

        let wind_charge =
            ThrownItemEntity::new_with_eye_offset(entity, player.get_entity(), WIND_CHARGE_GRAVITY, 0.0);
        let (yaw, pitch) = player.rotation();
        wind_charge.set_velocity_from(pitch, yaw, 0.0, POWER, 1.0);

        let player_vel = player.living_entity.entity.velocity.load();
        let on_ground = player
            .living_entity
            .entity
            .on_ground
            .load(std::sync::atomic::Ordering::Relaxed);
        let cur_vel = wind_charge.entity.velocity.load();
        let final_vel = cur_vel + Vector3::new(
            player_vel.x,
            if on_ground { 0.0 } else { player_vel.y },
            player_vel.z,
        );
        wind_charge.entity.velocity.store(final_vel);
        let len = final_vel.horizontal_length();
        wind_charge.entity.set_rotation(
            final_vel.x.atan2(final_vel.z) as f32 * 57.295_776,
            final_vel.y.atan2(len) as f32 * 57.295_776,
        );

        world.spawn_entity(Arc::new(WindChargeEntity::new_normal(wind_charge)));
        player.swing_hand(pumpkin_util::Hand::Right, true);

        let mut main_hand = player.inventory.held_item();
        let consumed = if !main_hand.is_empty() && main_hand.item.id == Item::WIND_CHARGE.id {
            main_hand.decrement_unless_creative(player.gamemode.load(), 1);
            player.inventory.set_held_item(main_hand);
            true
        } else {
            false
        };

        if !consumed {
            let mut off_hand = player.inventory.off_hand_item();
            if !off_hand.is_empty() && off_hand.item.id == Item::WIND_CHARGE.id {
                off_hand.decrement_unless_creative(player.gamemode.load(), 1);
                player
                    .inventory
                    .set_stack_in_hand(pumpkin_util::Hand::Left, off_hand);
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
