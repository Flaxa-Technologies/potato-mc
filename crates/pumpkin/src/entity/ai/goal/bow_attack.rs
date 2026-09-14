use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_util::Hand;
use rand::RngExt;
use std::sync::Arc;

use crate::entity::ai::goal::{Controls, Goal};
use crate::entity::ai::pathfinder::NavigatorGoal;
use crate::entity::mob::Mob;
use crate::entity::projectile::arrow::{ArrowEntity, ArrowPickup};
use crate::entity::{Entity, EntityBase};

/// Ranged bow attack used by skeletons and their variants.
/// Mirrors vanilla `RangedBowAttackGoal`: the mob keeps its distance, draws the bow
/// and releases an arrow once it has been drawn long enough.
pub struct BowAttackGoal {
    goal_control: Controls,
    speed: f64,
    attack_interval: i32,
    squared_range: f64,
    cooldown: i32,
    draw_ticks: i32,
    drawing: bool,
    see_time: i32,
    strafing_clockwise: bool,
    strafing_backwards: bool,
    strafing_time: i32,
}

impl BowAttackGoal {
    /// Ticks the bow has to be drawn before the arrow is released.
    const DRAW_TIME: i32 = 20;
    /// Vanilla arrow speed for mob shots.
    const ARROW_SPEED: f64 = 1.6;

    #[must_use]
    pub fn new(speed: f64, attack_interval: i32, range: f32) -> Self {
        Self {
            goal_control: Controls::MOVE | Controls::LOOK,
            speed,
            attack_interval,
            squared_range: f64::from(range * range),
            cooldown: -1,
            draw_ticks: 0,
            drawing: false,
            see_time: 0,
            strafing_clockwise: false,
            strafing_backwards: false,
            strafing_time: -1,
        }
    }

    fn main_hand_item(mob: &dyn Mob) -> ItemStack {
        mob.get_mob_entity()
            .living_entity
            .entity_equipment
            .try_lock()
            .map_or_else(
                |_| ItemStack::EMPTY.clone(),
                |eq| eq.get(&EquipmentSlot::MAIN_HAND),
            )
    }

    fn is_holding_bow(mob: &dyn Mob) -> bool {
        Self::main_hand_item(mob).item.id == Item::BOW.id
    }

    fn stop_drawing(&mut self, mob: &dyn Mob) {
        if self.drawing {
            mob.get_mob_entity().living_entity.clear_active_hand();
            self.drawing = false;
            self.draw_ticks = 0;
        }
    }

    /// Spawns the arrow, matching vanilla `AbstractSkeleton::performRangedAttack`.
    fn shoot(mob: &dyn Mob, target: &Arc<dyn EntityBase>) {
        let entity = mob.get_entity();
        let world = entity.world.load();
        let world_full = entity.world.load_full();

        let mut spawn_pos = entity.get_eye_pos();
        spawn_pos.y -= 0.1;
        let arrow_entity = Entity::new(world.clone(), spawn_pos, &EntityType::ARROW);
        let projectile = if entity.entity_type == &EntityType::BOGGED {
            use pumpkin_data::data_component::DataComponent;
            use pumpkin_data::data_component_impl::{DataComponentImpl, PotionContentsImpl};
            let mut stack = ItemStack::new(1, &Item::TIPPED_ARROW);
            stack.patch.push((
                DataComponent::PotionContents,
                Some(
                    PotionContentsImpl {
                        potion_id: Some(i32::from(pumpkin_data::potion::Potion::POISON.id)),
                        custom_color: None,
                        custom_effects: Vec::new(),
                        custom_name: None,
                    }
                    .to_dyn(),
                ),
            ));
            stack
        } else if entity.entity_type == &EntityType::STRAY {
            use pumpkin_data::data_component::DataComponent;
            use pumpkin_data::data_component_impl::{DataComponentImpl, PotionContentsImpl};
            let mut stack = ItemStack::new(1, &Item::TIPPED_ARROW);
            stack.patch.push((
                DataComponent::PotionContents,
                Some(
                    PotionContentsImpl {
                        potion_id: Some(i32::from(pumpkin_data::potion::Potion::SLOWNESS.id)),
                        custom_color: None,
                        custom_effects: Vec::new(),
                        custom_name: None,
                    }
                    .to_dyn(),
                ),
            ));
            stack
        } else {
            ItemStack::new(1, &Item::ARROW)
        };
        let bow_item = Self::main_hand_item(mob);
        let arrow = ArrowEntity::new_shot_with_weapon(
            arrow_entity,
            entity,
            &projectile,
            &bow_item,
            ArrowPickup::Disallowed,
        );

        // Vanilla scales base damage with power and world difficulty
        let difficulty = world.level_info.load().difficulty as i32;
        arrow.set_base_damage_from_mob(Self::ARROW_SPEED, difficulty);

        arrow.set_base_damage(crate::enchantment::EnchantmentHelper::modify_damage(
            &bow_item,
            arrow.get_base_damage(),
        ));
        arrow.punch_level.store(
            crate::enchantment::EnchantmentHelper::modify_knockback(&bow_item, 0.0) as u8,
            std::sync::atomic::Ordering::Relaxed,
        );
        arrow.apply_on_projectile_spawned(&projectile);
        if entity.is_on_fire() {
            arrow.set_flame(true);
        }

        let shooter_eye = entity.get_eye_pos();
        let target_entity = target.get_entity();
        let target_pos = target_entity.pos.load();
        let target_height = f64::from(target_entity.entity_dimension.load().height);

        let dx = target_pos.x - shooter_eye.x;
        let dy = (target_pos.y + target_height * (1.0 / 3.0)) - shooter_eye.y;
        let dz = target_pos.z - shooter_eye.z;
        let horizontal_distance = dx.hypot(dz);

        // Vanilla scales the spread with the world difficulty: 14 - difficulty * 4.
        let divergence = f64::from(14 - difficulty * 4);

        arrow.set_velocity(
            dx,
            horizontal_distance.mul_add(0.2, dy),
            dz,
            Self::ARROW_SPEED,
            divergence,
        );

        world.play_sound(Sound::EntityArrowShoot, SoundCategory::Hostile, &shooter_eye);

        let arrow: Arc<dyn EntityBase> = Arc::new(arrow);
        let entity_id = entity.entity_id;
        if let Some(server) = world_full.server.upgrade() {
            let mut event =
                crate::plugin::api::events::entity::entity_shoot_bow::EntityShootBowEvent::new(
                    entity_id,
                    "minecraft:bow".to_string(),
                    1.0,
                );
            server.plugin_manager.fire_blocking(&server, &mut event);
            if event.cancelled {
                return;
            }
        }
        world_full.spawn_entity(arrow);
    }
}

impl Goal for BowAttackGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        if mob.is_sitting() {
            return false;
        }
        let target = mob.get_mob_entity().get_target().clone();
        let Some(target) = target else {
            return false;
        };
        if !target.get_entity().is_alive() {
            return false;
        }
        if !mob.can_attack(target.as_ref()) {
            return false;
        }
        Self::is_holding_bow(mob)
    }

    fn should_continue(&self, mob: &dyn Mob) -> bool {
        if mob.is_sitting() {
            return false;
        }
        let target = mob.get_mob_entity().get_target().clone();
        let Some(target) = target else {
            return false;
        };
        target.get_entity().is_alive()
            && mob.can_attack(target.as_ref())
            && Self::is_holding_bow(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        mob.get_mob_entity().set_attacking(true);
        self.cooldown = -1;
        self.draw_ticks = 0;
        self.drawing = false;
        self.see_time = 0;
        self.strafing_time = -1;
    }

    fn stop(&mut self, mob: &dyn Mob) {
        mob.get_mob_entity().set_attacking(false);
        self.stop_drawing(mob);
        self.cooldown = -1;
        self.see_time = 0;
        self.strafing_time = -1;
        if let Some(living) = mob.get_living_entity() {
            living
                .movement_input
                .store(pumpkin_util::math::vector3::Vector3::default());
        }
        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
    }

    fn tick(&mut self, mob: &dyn Mob) {
        let target = mob.get_mob_entity().get_target().clone();
        let Some(target) = target else {
            if let Some(living) = mob.get_living_entity() {
                living
                    .movement_input
                    .store(pumpkin_util::math::vector3::Vector3::default());
            }
            return;
        };

        let mob_pos = mob.get_entity().pos.load();
        let target_entity = target.get_entity();
        let target_pos = target_entity.pos.load();
        let distance_sq = mob_pos.squared_distance_to_vec(&target_pos);

        let world = mob.get_entity().world.load();
        let has_line_of_sight = world
            .raycast(
                mob.get_entity().get_eye_pos(),
                target_entity.get_eye_pos(),
                |block_pos, w| w.get_block_state(block_pos).is_solid(),
            )
            .is_none();

        let had_line_of_sight = self.see_time > 0;
        if has_line_of_sight != had_line_of_sight {
            self.see_time = 0;
        }
        if has_line_of_sight {
            self.see_time += 1;
        } else {
            self.see_time -= 1;
        }

        // Vanilla 26.2 strafe and kiting:
        if distance_sq <= self.squared_range && self.see_time >= 20 {
            mob.get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .stop();
            self.strafing_time += 1;
        } else {
            mob.get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_progress(NavigatorGoal {
                    current_progress: mob_pos,
                    destination: target_pos,
                    speed: self.speed,
                });
            self.strafing_time = -1;
        }

        if self.strafing_time >= 20 {
            let mut rng = rand::rng();
            if rng.random::<f32>() < 0.3 {
                self.strafing_clockwise = !self.strafing_clockwise;
            }
            if rng.random::<f32>() < 0.3 {
                self.strafing_backwards = !self.strafing_backwards;
            }
            self.strafing_time = 0;
        }

        if self.strafing_time > -1 {
            if distance_sq > self.squared_range * 0.75 {
                self.strafing_backwards = false;
            } else if distance_sq < self.squared_range * 0.25 {
                // Kite backwards away from the target when close
                self.strafing_backwards = true;
            }

            let forward_dir = if self.strafing_backwards { -0.5 } else { 0.5 };
            let right_dir = if self.strafing_clockwise { 0.5 } else { -0.5 };
            mob.get_mob_entity()
                .move_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .strafe(forward_dir, right_dir);

            mob.get_mob_entity()
                .look_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .look_at_entity_with_range(&target, 30.0, 30.0);
            mob.get_mob_entity().look_at(target.as_ref(), 30.0, 30.0);
        } else {
            mob.get_mob_entity()
                .look_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .look_at_entity_with_range(&target, 30.0, 30.0);
            mob.get_mob_entity().look_at(target.as_ref(), 30.0, 30.0);
        }

        if self.drawing {
            if !has_line_of_sight && self.see_time < -60 {
                self.stop_drawing(mob);
            } else if has_line_of_sight {
                self.draw_ticks += 1;
                if self.draw_ticks >= Self::DRAW_TIME {
                    self.stop_drawing(mob);
                    Self::shoot(mob, &target);
                    let difficulty = world.level_info.load().difficulty;
                    self.cooldown = match difficulty {
                        pumpkin_util::Difficulty::Hard => 20,
                        _ => self.attack_interval,
                    };
                }
            }
        } else {
            self.cooldown -= 1;
            if self.cooldown <= 0 && self.see_time >= -60 && distance_sq <= self.squared_range {
                let stack = Self::main_hand_item(mob);
                mob.get_mob_entity()
                    .living_entity
                    .set_active_hand(Hand::Right, stack, 72000);
                self.drawing = true;
                self.draw_ticks = 0;
            }
        }
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        self.goal_control
    }
}
