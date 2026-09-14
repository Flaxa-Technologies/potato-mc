use pumpkin_util::math::position::BlockPos;

use super::{Controls, Goal};
use crate::entity::{ai::pathfinder::NavigatorGoal, mob::Mob};

pub struct WorkAtJobSiteGoal {
    speed: f64,
    target: Option<BlockPos>,
    work_timer: i32,
}

impl WorkAtJobSiteGoal {
    #[must_use]
    pub const fn new(speed: f64) -> Self {
        Self {
            speed,
            target: None,
            work_timer: 0,
        }
    }

    fn should_move_to_job_site(mob: &dyn Mob) -> bool {
        if mob.is_job_site_pending() {
            return true;
        }
        let world = mob.get_mob_entity().living_entity.entity.world.load();
        let daytime = world
            .level_time
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .query_daytime();
        (2_000..9_000).contains(&daytime)
    }
    fn find_stand_pos(
        world: &crate::world::World,
        target: BlockPos,
        current_pos: pumpkin_util::math::vector3::Vector3<f64>,
    ) -> pumpkin_util::math::vector3::Vector3<f64> {
        let offsets = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        let mut best_pos = None;
        let mut best_dist_sq = f64::MAX;

        for (dx, dz) in offsets {
            for dy in [0, -1, 1] {
                let stand_pos = BlockPos(target.0.add_raw(dx, dy, dz));
                let state = world.get_block_state(&stand_pos);
                let state_above = world.get_block_state(&stand_pos.up());
                let state_below = world.get_block_state(&stand_pos.down());

                if !state.is_solid() && !state_above.is_solid() && state_below.is_solid() {
                    let center = stand_pos.to_centered_f64();
                    let dist = center.squared_distance_to_vec(&current_pos);
                    if dist < best_dist_sq {
                        best_dist_sq = dist;
                        best_pos = Some(center);
                    }
                    break;
                }
            }
        }

        best_pos.unwrap_or_else(|| target.to_centered_f64())
    }
}

impl Goal for WorkAtJobSiteGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let Some(target) = mob.get_job_site() else {
            return false;
        };
        if !Self::should_move_to_job_site(mob) {
            return false;
        }
        self.target = Some(target);
        true
    }

    fn should_continue(&self, mob: &dyn Mob) -> bool {
        let Some(target) = self.target else {
            return false;
        };
        mob.get_job_site() == Some(target) && Self::should_move_to_job_site(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        self.work_timer = 20;
        if let Some(target) = self.target {
            let entity = &mob.get_mob_entity().living_entity.entity;
            let pos = entity.pos.load();
            let target_pos = target.to_centered_f64();
            if target_pos.squared_distance_to_vec(&pos) >= 1.73f64.powi(2) {
                let world = entity.world.load();
                let nav_target = Self::find_stand_pos(&world, target, pos);
                mob.get_mob_entity()
                    .navigator
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .set_progress(NavigatorGoal::new(
                        pos,
                        nav_target,
                        self.speed,
                    ));
            }
        }
    }

    fn stop(&mut self, mob: &dyn Mob) {
        if let Some(target) = self.target
            && target
                .to_centered_f64()
                .squared_distance_to_vec(&mob.get_mob_entity().living_entity.entity.pos.load())
                >= 2.0f64.powi(2)
            && mob.is_job_site_pending()
        {
            mob.release_pending_job_site(target);
        }
        self.target = None;
        mob.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
    }

    fn tick(&mut self, mob: &dyn Mob) {
        let Some(target) = self.target else {
            return;
        };
        let mob_entity = mob.get_mob_entity();
        let entity = &mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let target_pos = target.to_centered_f64();
        let dist_sq = target_pos.squared_distance_to_vec(&pos);

        if dist_sq < 1.73f64.powi(2) {
            mob_entity
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .stop();
            mob_entity
                .look_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .look_at(mob, target_pos.x, target_pos.y, target_pos.z);

            self.work_timer -= 1;
            if self.work_timer <= 0 {
                self.work_timer = 300;
                mob.work_at_job_site();
            }
        } else {
            let mut nav = mob_entity
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if nav.is_idle() {
                let world = entity.world.load();
                let nav_target = Self::find_stand_pos(&world, target, pos);
                nav.set_progress(NavigatorGoal::new(pos, nav_target, self.speed));
            }
        }
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }
}
