use pumpkin_data::tag::{self, Taggable};
use pumpkin_util::math::position::BlockPos;
use std::sync::atomic::Ordering;

use super::{Controls, Goal};
use crate::block::blocks::doors::DoorBlock;
use crate::entity::mob::Mob;

pub struct DoorInteractGoal {
    pub door_pos: BlockPos,
    pub has_door: bool,
    pub passed: bool,
    pub door_open_dir_x: f32,
    pub door_open_dir_z: f32,
}

impl Default for DoorInteractGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl DoorInteractGoal {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            door_pos: BlockPos::ZERO,
            has_door: false,
            passed: false,
            door_open_dir_x: 0.0,
            door_open_dir_z: 0.0,
        }
    }

    pub fn is_open(&mut self, mob: &dyn Mob) -> bool {
        if !self.has_door {
            return false;
        }
        let world = mob.get_entity().world.load();
        let (block, _) = world.get_block_and_state_id(&self.door_pos);
        if !block.has_tag(&tag::Block::MINECRAFT_DOORS) {
            self.has_door = false;
            return false;
        }
        DoorBlock::is_open(&world, &self.door_pos)
    }

    pub fn set_open(&mut self, mob: &dyn Mob, open: bool) {
        if self.has_door {
            let world = mob.get_entity().world.load_full();
            let (block, _) = world.get_block_and_state_id(&self.door_pos);
            if block.has_tag(&tag::Block::MINECRAFT_DOORS) {
                DoorBlock::set_open(&world, &self.door_pos, open);
            }
        }
    }

    pub fn can_use(&mut self, mob: &dyn Mob) -> bool {
        let has_collision = mob
            .get_entity()
            .horizontal_collision
            .load(Ordering::Relaxed);

        let navigator = mob
            .get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let Some(path) = navigator.get_path() else {
            return false;
        };

        if path.is_done() {
            return false;
        }

        let world = mob.get_entity().world.load();
        let mob_pos = mob.get_entity().pos.load();
        let limit = (path.get_next_node_index() + 2).min(path.get_node_count());

        for i in 0..limit {
            let Some(node) = path.get_node(i) else {
                continue;
            };
            let dx = mob_pos.x - (f64::from(node.pos.0.x) + 0.5);
            let dz = mob_pos.z - (f64::from(node.pos.0.z) + 0.5);
            let dist_sqr = dx * dx + dz * dz;

            if has_collision || dist_sqr <= 4.0 {
                let door_pos_lower = BlockPos::new(node.pos.0.x, node.pos.0.y, node.pos.0.z);
                if DoorBlock::is_wooden_door(&world, &door_pos_lower) {
                    self.door_pos = door_pos_lower;
                    self.has_door = true;
                    return true;
                }
                let door_pos_upper = BlockPos::new(node.pos.0.x, node.pos.0.y + 1, node.pos.0.z);
                if DoorBlock::is_wooden_door(&world, &door_pos_upper) {
                    self.door_pos = door_pos_upper;
                    self.has_door = true;
                    return true;
                }
            }
        }

        let mob_block = mob.get_entity().block_pos.load();
        for check_pos in [mob_block, mob_block.up()] {
            if DoorBlock::is_wooden_door(&world, &check_pos) {
                self.door_pos = check_pos;
                self.has_door = true;
                return true;
            }
        }
        false
    }

    #[must_use]
    pub const fn can_continue_to_use(&self) -> bool {
        !self.passed
    }

    pub fn start_interaction(&mut self, mob: &dyn Mob) {
        self.passed = false;
        let mob_pos = mob.get_entity().pos.load();
        self.door_open_dir_x = (self.door_pos.0.x as f32 + 0.5) - mob_pos.x as f32;
        self.door_open_dir_z = (self.door_pos.0.z as f32 + 0.5) - mob_pos.z as f32;
    }

    pub fn tick_interaction(&mut self, mob: &dyn Mob) {
        let mob_pos = mob.get_entity().pos.load();
        let new_door_dir_x = (self.door_pos.0.x as f32 + 0.5) - mob_pos.x as f32;
        let new_door_dir_z = (self.door_pos.0.z as f32 + 0.5) - mob_pos.z as f32;
        let dot = self.door_open_dir_x * new_door_dir_x + self.door_open_dir_z * new_door_dir_z;
        if dot < 0.0 {
            self.passed = true;
        }
    }
}

impl Goal for DoorInteractGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        self.can_use(mob)
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        self.can_continue_to_use()
    }

    fn start(&mut self, mob: &dyn Mob) {
        self.start_interaction(mob);
    }

    fn tick(&mut self, mob: &dyn Mob) {
        self.tick_interaction(mob);
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn controls(&self) -> Controls {
        Controls::empty()
    }
}
