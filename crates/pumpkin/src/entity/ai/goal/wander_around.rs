use super::{Controls, Goal, to_goal_ticks};
use crate::entity::{ai::pathfinder::NavigatorGoal, mob::Mob};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};
use rand::RngExt;

pub struct WanderAroundGoal {
    goal_control: Controls,
    speed: f64,
    target: Option<Vector3<f64>>,
    chance: i32,
}

impl WanderAroundGoal {
    #[must_use]
    pub const fn new(speed: f64) -> Self {
        Self {
            goal_control: Controls::MOVE,
            speed,
            target: None,
            chance: to_goal_ticks(120),
        }
    }

    fn find_wander_target(mob: &dyn Mob) -> Option<Vector3<f64>> {
        let mob_entity = mob.get_mob_entity();
        let entity = &mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let world = entity.world.load();
        let mut rng = mob.get_random();

        let base_x = pos.x.floor() as i32;
        let base_y = pos.y.floor() as i32;
        let base_z = pos.z.floor() as i32;

        let horizontal_range = 10;
        let vertical_range = 7;

        // Try up to 10 random candidates like vanilla RandomPos.RANDOM_POS_ATTEMPTS
        for _ in 0..10 {
            let dx = rng.random_range(-horizontal_range..=horizontal_range);
            let dy = rng.random_range(-vertical_range..=vertical_range);
            let dz = rng.random_range(-horizontal_range..=horizontal_range);

            if dx == 0 && dz == 0 {
                continue;
            }

            let target_x = base_x + dx;
            let target_y = (base_y + dy).clamp(-64, 319);
            let target_z = base_z + dz;

            let mut check_pos = BlockPos::new(target_x, target_y, target_z);
            let block = world.get_block(&check_pos);

            // If inside a solid block, scan up to find open air
            if block.is_solid() {
                let mut found_surface = false;
                for _ in 0..=vertical_range {
                    let up = check_pos.up();
                    if up.0.y >= 319 {
                        break;
                    }
                    if !world.get_block(&up).is_solid() {
                        check_pos = up;
                        found_surface = true;
                        break;
                    }
                    check_pos = up;
                }
                if !found_surface {
                    continue;
                }
            } else {
                // If in air, scan down to find solid ground
                let mut found_ground = false;
                for _ in 0..=vertical_range {
                    let below = check_pos.down();
                    if below.0.y < -64 {
                        break;
                    }
                    let below_block = world.get_block(&below);
                    if below_block.is_solid() {
                        found_ground = true;
                        break;
                    }
                    check_pos = below;
                }
                if !found_ground {
                    continue;
                }
            }

            // At this point, check_pos is a non-solid block, and check_pos.down() should be solid.
            let below = check_pos.down();
            let below_block = world.get_block(&below);
            if !below_block.is_solid() {
                continue;
            }

            // Headroom check: check_pos and check_pos.up() must be passable (not solid)
            let feet_block = world.get_block(&check_pos);
            let head_block = world.get_block(&check_pos.up());
            if feet_block.is_solid() || head_block.is_solid() {
                continue;
            }

            // Avoid liquids (water/lava) for standard land wandering
            if feet_block.id == pumpkin_data::Block::WATER.id
                || feet_block.id == pumpkin_data::Block::LAVA.id
                || below_block.id == pumpkin_data::Block::WATER.id
                || below_block.id == pumpkin_data::Block::LAVA.id
            {
                continue;
            }

            // Return centered position on the target block
            return Some(Vector3::new(
                f64::from(check_pos.0.x) + 0.5,
                f64::from(check_pos.0.y),
                f64::from(check_pos.0.z) + 0.5,
            ));
        }

        None
    }
}

impl Goal for WanderAroundGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        if mob.is_sitting()
            || mob.get_mob_entity().living_entity.entity.pose.load()
                == pumpkin_data::entity::EntityPose::Sleeping
        {
            return false;
        }

        if mob.get_random().random_range(0..self.chance) != 0 {
            return false;
        }

        if let Some(target) = Self::find_wander_target(mob) {
            self.target = Some(target);
            true
        } else {
            false
        }
    }

    fn should_continue(&self, mob: &dyn Mob) -> bool {
        if mob.is_sitting()
            || mob.get_mob_entity().living_entity.entity.pose.load()
                == pumpkin_data::entity::EntityPose::Sleeping
        {
            return false;
        }

        let navigator = mob
            .get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !navigator.is_idle()
    }

    fn start(&mut self, mob: &dyn Mob) {
        if let Some(target) = self.target {
            let pos = mob.get_mob_entity().living_entity.entity.pos.load();
            let mut navigator = mob
                .get_mob_entity()
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            navigator.set_progress(NavigatorGoal::new(pos, target, self.speed));
        }
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        self.target = None;
    }

    fn controls(&self) -> Controls {
        self.goal_control
    }
}
