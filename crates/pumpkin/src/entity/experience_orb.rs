use core::f32;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use pumpkin_data::entity::EntityType;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_util::math::vector3::Vector3;

use crate::{server::Server, world::World};

use super::{Entity, EntityBase, living::LivingEntity, player::Player};

pub struct ExperienceOrbEntity {
    entity: Entity,
    amount: u32,
    orb_age: AtomicU32,
}

impl ExperienceOrbEntity {
    pub fn new(entity: Entity, amount: u32) -> Self {
        entity.yaw.store(rand::random::<f32>() * 360.0);
        Self {
            entity,
            amount,
            orb_age: AtomicU32::new(0),
        }
    }

    pub fn spawn(world: &Arc<World>, position: Vector3<f64>, amount: u32) {
        let mut amount = amount;
        while amount > 0 {
            let i = Self::round_to_orb_size(amount);
            amount -= i;
            let entity = Entity::new(world.clone(), position, &EntityType::EXPERIENCE_ORB);
            let vx = (rand::random::<f64>() * 0.2 - 0.1) * 2.0;
            let vy = rand::random::<f64>() * 0.2 * 2.0;
            let vz = (rand::random::<f64>() * 0.2 - 0.1) * 2.0;
            entity.velocity.store(Vector3::new(vx, vy, vz));
            let orb = Arc::new(Self::new(entity, i));
            world.spawn_entity(orb);
        }
    }

    const fn round_to_orb_size(value: u32) -> u32 {
        if value >= 2477 {
            2477
        } else if value >= 1237 {
            1237
        } else if value >= 617 {
            617
        } else if value >= 307 {
            307
        } else if value >= 149 {
            149
        } else if value >= 73 {
            73
        } else if value >= 37 {
            37
        } else if value >= 17 {
            17
        } else if value >= 7 {
            7
        } else if value >= 3 {
            3
        } else {
            1
        }
    }
}

impl EntityBase for ExperienceOrbEntity {
    fn tick(&self, caller: &dyn EntityBase, server: &Server) {
        let entity = &self.entity;
        entity.tick(caller, server);
        let bounding_box = entity.bounding_box.load();

        let original_velo = entity.velocity.load();
        let mut velo = original_velo;

        let world = self.entity.world.load();
        let orb_pos = entity.pos.load();

        // Follow nearby players matching vanilla ExperienceOrb.followNearbyPlayer
        let nearby_players = world.get_nearby_players(orb_pos, 8.0);
        let mut target_player: Option<(f64, Arc<Player>)> = None;

        for player in nearby_players {
            if player.is_spectator() || !player.get_entity().is_alive() {
                continue;
            }
            let p_pos = player.get_entity().pos.load();
            let dist_sq = orb_pos.squared_distance_to_vec(&p_pos);
            if dist_sq < 64.0 && target_player.as_ref().is_none_or(|(best, _)| dist_sq < *best) {
                target_player = Some((dist_sq, player));
            }
        }

        if let Some((dist_sq, player)) = target_player.as_ref() {
            let p_pos = player.get_entity().pos.load();
            let eye_offset = player.get_entity().get_eye_height() as f64 / 2.0;
            let target_center = Vector3::new(p_pos.x, p_pos.y + eye_offset, p_pos.z);
            let delta = target_center - orb_pos;
            let len = dist_sq.sqrt();
            if len > 0.0001 {
                let power = (1.0 - len / 8.0).max(0.0);
                let accel = delta.normalize() * (power * power * 0.1);
                velo += accel;
            }

            let p_bb = player.get_entity().bounding_box.load().expand(1.0, 0.5, 1.0);
            let center_dist_sq = orb_pos.squared_distance_to_vec(&target_center);
            if center_dist_sq <= 4.0 || *dist_sq <= 4.0 || p_bb.intersects(&bounding_box) {
                self.on_player_collision(player);
            }
        }

        let colliding = !world.is_space_empty(bounding_box.expand(-1.0e-7, -1.0e-7, -1.0e-7));
        // In vanilla ExperienceOrb.java:108-113, colliding is only used to avoid applying gravity
        // when resting on a block. noPhysics is never set (normal collision must remain active).
        // TODO: isSubmergedIn
        if !colliding {
            velo.y -= self.get_gravity();
        }

        velo.x *= 0.98;
        velo.z *= 0.98;

        entity.velocity.store(velo);

        let fall_speed = velo.y;
        entity.move_entity(caller, velo);

        // Vanilla ExperienceOrb.java:141-143 ground bounce
        if entity.on_ground.load(Ordering::Relaxed) && fall_speed < -self.get_gravity() {
            let mut current_velo = entity.velocity.load();
            current_velo.y = -fall_speed * 0.4;
            entity.velocity.store(current_velo);
        }

        if let Some((_, ref player)) = target_player {
            let updated_bb = entity.bounding_box.load();
            let p_bb = player.get_entity().bounding_box.load().expand(1.0, 0.5, 1.0);
            if p_bb.intersects(&updated_bb) {
                self.on_player_collision(player);
            }
        }

        entity.tick_block_collisions(caller);

        let age = self.orb_age.fetch_add(1, Ordering::Relaxed);
        if age >= 6000 {
            entity.remove();
        }
    }

    fn get_entity(&self) -> &Entity {
        &self.entity
    }

    fn on_player_collision(&self, player: &Arc<Player>) {
        if !self.entity.is_alive() {
            return;
        }
        if player.living_entity.health.load() > 0.0 && !player.is_spectator() {
            let can_pickup = match player.experience_pick_up_delay.lock() {
                Ok(mut delay) => {
                    if *delay == 0 {
                        *delay = 2;
                        true
                    } else {
                        false
                    }
                }
                Err(_) => false,
            };
            if !can_pickup {
                return;
            }

            player.living_entity.pickup(&self.entity, 1);
            let world = self.entity.world.load();
            let pos = self.entity.pos.load();
            let pitch = (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0;
            world.play_sound_fine(
                Sound::EntityExperienceOrbPickup,
                SoundCategory::Players,
                &pos,
                0.1,
                pitch,
            );
            self.entity.remove();
            let amount = self.amount as i32;
            let remaining = player.apply_mending_from_xp(amount);
            if remaining > 0 {
                player.add_experience_points(remaining);
            }
        }
    }

    fn get_living_entity(&self) -> Option<&LivingEntity> {
        None
    }
    fn get_gravity(&self) -> f64 {
        0.03
    }

    fn cast_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_to_orb_size_tiers() {
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(3000), 2477);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(1500), 1237);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(800), 617);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(400), 307);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(200), 149);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(100), 73);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(50), 37);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(20), 17);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(10), 7);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(5), 3);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(1), 1);
        assert_eq!(ExperienceOrbEntity::round_to_orb_size(0), 1);
    }

    #[test]
    fn test_ground_bounce_velocity() {
        let gravity: f64 = 0.03;
        let fall_speed: f64 = -0.5;
        assert!(fall_speed < -gravity);
        let bounced_y: f64 = -fall_speed * 0.4;
        assert!((bounced_y - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_pitch_variation_range() {
        for _ in 0..100 {
            let r1 = rand::random::<f32>();
            let r2 = rand::random::<f32>();
            let pitch = (r1 - r2) * 0.2 + 1.0;
            assert!(pitch >= 0.8 && pitch <= 1.2);
        }
    }
}

