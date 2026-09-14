use pumpkin_protocol::java::client::play::CWorldEvent;
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{Controls, Goal},
    ai::pathfinder::NavigatorGoal,
    mob::Mob,
    mob::blaze::BlazeEntity,
    projectile::small_fireball::SmallFireballEntity,
};

pub struct BlazeShootFireballGoal {
    blaze: std::sync::Weak<BlazeEntity>,
    attack_step: i32,
    attack_time: i32,
    melee_cooldown: i32,
    last_seen: i32,
}

impl BlazeShootFireballGoal {
    #[must_use]
    pub const fn new(blaze: std::sync::Weak<BlazeEntity>) -> Self {
        Self {
            blaze,
            attack_step: 0,
            attack_time: 0,
            melee_cooldown: 0,
            last_seen: 0,
        }
    }

    const fn get_follow_distance() -> f64 {
        48.0
    }
}

impl Goal for BlazeShootFireballGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(blaze) = self.blaze.upgrade() else {
            return false;
        };
        let target = blaze.entity.get_target();
        target.is_some_and(|t| t.get_entity().is_alive())
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(blaze) = self.blaze.upgrade() else {
            return false;
        };
        let target = blaze.entity.get_target();
        target.is_some_and(|t| t.get_entity().is_alive())
    }

    fn start(&mut self, _mob: &dyn Mob) {
        self.attack_step = 0;
        self.melee_cooldown = 0;
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        if let Some(blaze) = self.blaze.upgrade() {
            blaze.set_charged(false);
        }
        self.last_seen = 0;
        self.attack_step = 0;
        self.melee_cooldown = 0;
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        self.attack_time -= 1;
        self.melee_cooldown -= 1;

        let Some(blaze) = self.blaze.upgrade() else {
            return;
        };

        let target = blaze.entity.get_target();
        let Some(target) = target else {
            return;
        };

        let has_line_of_sight = true;

        if has_line_of_sight {
            self.last_seen = 0;
        } else {
            self.last_seen += 1;
        }

        let blaze_pos = blaze.entity.living_entity.entity.pos.load();
        let target_pos = target.get_entity().pos.load();

        let dx = target_pos.x - blaze_pos.x;
        let dy = target_pos.y - blaze_pos.y;
        let dz = target_pos.z - blaze_pos.z;

        let distance_sq = dx * dx + dy * dy + dz * dz;
        let in_melee_range = blaze.entity.is_in_attack_range(&*target) || distance_sq < 9.0;

        if in_melee_range {
            if has_line_of_sight && self.melee_cooldown <= 0 {
                self.melee_cooldown = 20;
                blaze.entity.living_entity.swing_hand();
                blaze.entity.try_attack(&*blaze, &*target);
                target.get_entity().set_on_fire_for(5.0);
            }

            blaze
                .entity
                .move_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_wanted_position(target_pos.x, target_pos.y, target_pos.z, 1.0);

            let mob_pos = blaze.entity.living_entity.entity.pos.load();
            blaze
                .entity
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_progress(NavigatorGoal::new(mob_pos, target_pos, 1.0));
        }

        if distance_sq < Self::get_follow_distance().powi(2) && has_line_of_sight {
            if distance_sq >= 9.0 {
                blaze
                    .entity
                    .move_control
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .set_wanted_position(target_pos.x, target_pos.y, target_pos.z, 1.0);

                let mob_pos = blaze.entity.living_entity.entity.pos.load();
                blaze
                    .entity
                    .navigator
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .set_progress(NavigatorGoal::new(mob_pos, target_pos, 1.0));
            }

            let blaze_ent = &blaze.entity.living_entity.entity;
            let shooter_eye = blaze_ent.get_eye_pos();
            let target_ent = target.get_entity();
            let target_pos = target_ent.pos.load();
            let target_height = f64::from(target_ent.entity_dimension.load().height);
            let target_aim_y = target_pos.y + target_height * 0.65;

            let target_vel = target_ent.velocity.load();
            let distance = distance_sq.sqrt();
            let travel_time = (distance / 1.0).clamp(0.0, 15.0);
            let lead_x = target_pos.x + target_vel.x * travel_time * 0.5;
            let lead_y = target_aim_y + target_vel.y * travel_time * 0.5;
            let lead_z = target_pos.z + target_vel.z * travel_time * 0.5;

            if self.attack_time <= 0 {
                self.attack_step += 1;
                if self.attack_step == 1 {
                    self.attack_time = 60;
                    blaze.set_charged(true);
                } else if self.attack_step <= 4 {
                    self.attack_time = 6;
                } else {
                    self.attack_time = 100;
                    self.attack_step = 0;
                    blaze.set_charged(false);
                }

                if self.attack_step > 1 {
                    let chunk_pos = blaze.entity.living_entity.entity.chunk_pos.load();
                    blaze
                        .entity
                        .living_entity
                        .entity
                        .world
                        .load()
                        .broadcast_to_chunk(
                            chunk_pos,
                            &CWorldEvent::new(
                                1018,
                                blaze.entity.living_entity.entity.block_pos.load(),
                                0,
                                false,
                            ),
                        );

                    let world = blaze.entity.living_entity.entity.world.load_full();
                    let uuid = uuid::Uuid::new_v4();

                    let sqd = distance.sqrt() * 0.5;
                    let spread_factor = if self.attack_step == 2 {
                        0.05
                    } else {
                        0.22
                    };
                    let spread = spread_factor * sqd;
                    let rand_x = (rand::random::<f64>() - rand::random::<f64>()) * spread;
                    let rand_y = (rand::random::<f64>() - rand::random::<f64>()) * (spread * 0.5);
                    let rand_z = (rand::random::<f64>() - rand::random::<f64>()) * spread;

                    let dir_x = (lead_x - shooter_eye.x) + rand_x;
                    let dir_y = (lead_y - shooter_eye.y) + rand_y;
                    let dir_z = (lead_z - shooter_eye.z) + rand_z;
                    let dir = Vector3::new(dir_x, dir_y, dir_z).normalize();
                    let spawn_pos = shooter_eye + dir * 0.75;

                    let base_entity = Entity::from_uuid(
                        uuid,
                        world.clone(),
                        spawn_pos,
                        &pumpkin_data::entity::EntityType::SMALL_FIREBALL,
                    );

                    let fireball = SmallFireballEntity::new_shot(
                        base_entity,
                        blaze_ent,
                    );
                    fireball.thrown.set_velocity(dir.x, dir.y, dir.z, 1.0, 0.0);
                    world.spawn_entity(Arc::new(fireball));
                }
            }

            blaze
                .entity
                .look_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .look_at_with_range(lead_x, lead_y, lead_z, 30.0, 40.0);
        } else if self.last_seen < 5 {
            blaze
                .entity
                .move_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_wanted_position(target_pos.x, target_pos.y, target_pos.z, 1.0);

            let mob_pos = blaze.entity.living_entity.entity.pos.load();
            blaze
                .entity
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_progress(NavigatorGoal::new(mob_pos, target_pos, 1.0));
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }
}
