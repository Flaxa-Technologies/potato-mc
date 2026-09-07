use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, Weak};
use uuid::Uuid;

use pumpkin_data::damage::DamageType;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tracked_data;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_nbt::tag::NbtTag;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::ai::goal::active_target::ActiveTargetGoal;
use crate::entity::ai::goal::look_around::RandomLookAroundGoal;
use crate::entity::ai::goal::look_at_entity::LookAtEntityGoal;
use crate::entity::ai::goal::revenge::RevengeGoal;
use crate::entity::ai::goal::swim::SwimGoal;
use crate::entity::{
    Entity, EntityBase,
    ai::goal::{Controls, Goal},
    mob::{Mob, MobEntity},
};

pub struct VexEntity {
    pub mob_entity: MobEntity,
    pub is_charging: AtomicBool,
    pub has_limited_life: AtomicBool,
    pub limited_life_ticks: AtomicI32,
    pub bound_origin: Mutex<Option<BlockPos>>,
    pub owner_uuid: Mutex<Option<Uuid>>,
    pub wanted_position: Mutex<Option<Vector3<f64>>>,
    pub ambient_sound_timer: AtomicI32,
}

impl VexEntity {
    pub const XP_REWARD: u32 = 3;

    pub fn new(entity: Entity) -> Arc<Self> {
        let spawn_pos = entity.pos.load();
        let initial_bound = BlockPos::new(
            spawn_pos.x.floor() as i32,
            spawn_pos.y.floor() as i32,
            spawn_pos.z.floor() as i32,
        );

        let mob_entity = MobEntity::new(entity);

        // Equip Iron Sword in main hand by default (0% drop chance)
        {
            let mut eq = mob_entity
                .living_entity
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            eq.put(&EquipmentSlot::MAIN_HAND, ItemStack::new(1, &Item::IRON_SWORD));
        }

        let vex = Self {
            mob_entity,
            is_charging: AtomicBool::new(false),
            has_limited_life: AtomicBool::new(false),
            limited_life_ticks: AtomicI32::new(0),
            bound_origin: Mutex::new(Some(initial_bound)),
            owner_uuid: Mutex::new(None),
            wanted_position: Mutex::new(None),
            ambient_sound_timer: AtomicI32::new(rand::random_range(80..160)),
        };

        let mob_arc = Arc::new(vex);
        let mob_weak: Weak<VexEntity> = Arc::downgrade(&mob_arc);
        let mob_trait_weak: Weak<dyn Mob> = {
            let dyn_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&dyn_arc)
        };

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 0: Float / swim
            goal_selector.add_goal(0, Box::new(SwimGoal::default()));

            // Priority 1: Flight movement, lifespan decay, ambient sound (every tick)
            goal_selector.add_goal(1, Box::new(VexMoveControlGoal::new(mob_weak.clone())));

            // Priority 4: Charge attack dive
            goal_selector.add_goal(4, Box::new(VexChargeAttackGoal::new(mob_weak.clone())));

            // Priority 8: Random wandering around bound origin
            goal_selector.add_goal(8, Box::new(VexRandomMoveGoal::new(mob_weak.clone())));

            // Priority 9: Look at player
            goal_selector.add_goal(
                9,
                LookAtEntityGoal::with_default(mob_trait_weak.clone(), &EntityType::PLAYER, 3.0),
            );

            // Priority 10: Look around
            goal_selector.add_goal(10, Box::new(RandomLookAroundGoal::default()));

            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 1: Revenge / HurtByTarget
            target_selector.add_goal(1, Box::new(RevengeGoal::new(true)));

            // Priority 2: Copy owner's target
            target_selector.add_goal(2, Box::new(VexCopyOwnerTargetGoal::new(mob_weak.clone())));

            // Priority 3: Nearest attackable player
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
        }

        mob_arc
    }

    #[must_use]
    pub fn is_charging(&self) -> bool {
        self.is_charging.load(Ordering::Relaxed)
    }

    pub fn set_charging(&self, charging: bool) {
        self.is_charging.store(charging, Ordering::Relaxed);
        let entity = &self.mob_entity.living_entity.entity;
        entity.set_synced_data(
            tracked_data::vex::DATA_FLAGS_ID,
            if charging { 1u8 } else { 0u8 },
        );
    }

    #[must_use]
    pub fn has_limited_life(&self) -> bool {
        self.has_limited_life.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn get_limited_life_ticks(&self) -> i32 {
        self.limited_life_ticks.load(Ordering::Relaxed)
    }

    pub fn set_limited_life(&self, life_ticks: i32) {
        self.has_limited_life.store(true, Ordering::Relaxed);
        self.limited_life_ticks.store(life_ticks, Ordering::Relaxed);
    }

    #[must_use]
    pub fn get_bound_origin(&self) -> Option<BlockPos> {
        *self
            .bound_origin
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_bound_origin(&self, pos: Option<BlockPos>) {
        *self
            .bound_origin
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = pos;
    }

    #[must_use]
    pub fn get_owner_uuid(&self) -> Option<Uuid> {
        *self
            .owner_uuid
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_owner_uuid(&self, uuid: Option<Uuid>) {
        *self
            .owner_uuid
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = uuid;
    }

    #[must_use]
    pub fn get_wanted_pos(&self) -> Option<Vector3<f64>> {
        *self
            .wanted_position
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_wanted_pos(&self, pos: Option<Vector3<f64>>) {
        *self
            .wanted_position
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = pos;
    }
}

impl Mob for VexEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_mob_gravity(&self) -> f64 {
        0.0 // Flying mob, zero gravity
    }

    fn get_mob_y_velocity_drag(&self) -> Option<f64> {
        Some(0.95)
    }

    fn get_base_experience_reward(&self) -> u32 {
        Self::XP_REWARD
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        let entity = &self.mob_entity.living_entity.entity;
        entity.play_sound(Sound::EntityVexHurt);
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            tracked_data::vex::DATA_FLAGS_ID,
            if self.is_charging() { 1u8 } else { 0u8 },
        );
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        if let Some(bound) = self.get_bound_origin() {
            nbt.put(
                "bound_pos",
                NbtTag::IntArray(vec![bound.0.x, bound.0.y, bound.0.z]),
            );
            nbt.put_int("BoundX", bound.0.x);
            nbt.put_int("BoundY", bound.0.y);
            nbt.put_int("BoundZ", bound.0.z);
        }

        if self.has_limited_life() {
            let ticks = self.get_limited_life_ticks();
            nbt.put_int("life_ticks", ticks);
            nbt.put_int("LifeTicks", ticks);
        }

        if let Some(owner) = self.get_owner_uuid() {
            nbt.put_uuid("owner", owner);
            nbt.put_uuid("Owner", owner);
        }
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(arr) = nbt.get_int_array("bound_pos") {
            if arr.len() == 3 {
                self.set_bound_origin(Some(BlockPos::new(arr[0], arr[1], arr[2])));
            }
        } else if let (Some(x), Some(y), Some(z)) =
            (nbt.get_int("BoundX"), nbt.get_int("BoundY"), nbt.get_int("BoundZ"))
        {
            self.set_bound_origin(Some(BlockPos::new(x, y, z)));
        }

        if let Some(ticks) = nbt.get_int("life_ticks").or_else(|| nbt.get_int("LifeTicks")) {
            self.set_limited_life(ticks);
        }

        if let Some(owner) = nbt.get_uuid("owner").or_else(|| nbt.get_uuid("Owner")) {
            self.set_owner_uuid(Some(owner));
        }
    }
}

/// Movement control, noclip flight, lifespan decay, and ambient sound
pub struct VexMoveControlGoal {
    vex: Weak<VexEntity>,
}

impl VexMoveControlGoal {
    #[must_use]
    pub const fn new(vex: Weak<VexEntity>) -> Self {
        Self { vex }
    }
}

impl Goal for VexMoveControlGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        true
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        true
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(vex) = self.vex.upgrade() else {
            return;
        };

        let entity = &vex.mob_entity.living_entity.entity;

        // 1. Lifespan decay (red flash damage every 20 ticks when expired)
        if vex.has_limited_life() {
            let remaining = vex.limited_life_ticks.fetch_sub(1, Ordering::Relaxed) - 1;
            if remaining <= 0 {
                vex.limited_life_ticks.store(20, Ordering::Relaxed);
                entity.damage(&*vex, 1.0, DamageType::GENERIC);
            }
        }

        // 2. Ambient sound
        let ambient = vex.ambient_sound_timer.fetch_sub(1, Ordering::Relaxed);
        if ambient <= 0 {
            vex.ambient_sound_timer
                .store(rand::random_range(100..200), Ordering::Relaxed);
            entity.play_sound(Sound::EntityVexAmbient);
        }

        // 3. Movement towards wanted position (vanilla VexMoveControl)
        let pos = entity.pos.load();
        if let Some(wanted) = vex.get_wanted_pos() {
            let delta = wanted - pos;
            let dist = delta.length();

            if dist < 0.5 {
                vex.set_wanted_pos(None);
                let vel = entity.velocity.load();
                entity.velocity.store(vel * 0.5);
            } else {
                let speed_mod = if vex.is_charging() { 1.0 } else { 0.25 };
                let step = delta * (speed_mod * 0.05 / dist);
                let current_vel = entity.velocity.load();
                entity.velocity.store(current_vel + step);

                // Update yaw facing target or movement direction
                if let Some(target) = vex.mob_entity.get_target() {
                    let tpos = target.get_entity().pos.load();
                    let dx = tpos.x - pos.x;
                    let dz = tpos.z - pos.z;
                    let yaw = (-dx.atan2(dz).to_degrees()) as f32;
                    entity.yaw.store(yaw);
                    entity.head_yaw.store(yaw);
                    entity.body_yaw.store(yaw);
                } else if delta.x.abs() > 0.01 || delta.z.abs() > 0.01 {
                    let yaw = (-delta.x.atan2(delta.z).to_degrees()) as f32;
                    entity.yaw.store(yaw);
                    entity.head_yaw.store(yaw);
                    entity.body_yaw.store(yaw);
                }
            }
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

/// Charge Attack Goal: darts towards target, turns red, deals damage
pub struct VexChargeAttackGoal {
    vex: Weak<VexEntity>,
}

impl VexChargeAttackGoal {
    #[must_use]
    pub const fn new(vex: Weak<VexEntity>) -> Self {
        Self { vex }
    }
}

impl Goal for VexChargeAttackGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(vex) = self.vex.upgrade() else {
            return false;
        };

        if vex.get_wanted_pos().is_some() {
            return false;
        }

        let Some(target) = vex.mob_entity.get_target() else {
            return false;
        };

        if !target.get_entity().is_alive() {
            return false;
        }

        let mut rng = rand::rng();
        if rng.random_range(0..7) != 0 {
            return false;
        }

        let my_pos = vex.mob_entity.living_entity.entity.pos.load();
        let target_pos = target.get_entity().pos.load();
        my_pos.squared_distance_to_vec(&target_pos) > 4.0
    }

    fn start(&mut self, _mob: &dyn Mob) {
        let Some(vex) = self.vex.upgrade() else {
            return;
        };
        let Some(target) = vex.mob_entity.get_target() else {
            return;
        };

        let target_pos = target.get_entity().pos.load();
        vex.set_wanted_pos(Some(Vector3::new(
            target_pos.x,
            target_pos.y + 1.2,
            target_pos.z,
        )));
        vex.set_charging(true);

        let entity = &vex.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pos = entity.pos.load();
        world.play_sound_fine(
            Sound::EntityVexCharge,
            SoundCategory::Hostile,
            &pos,
            1.0,
            1.0,
        );
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(vex) = self.vex.upgrade() else {
            return false;
        };
        vex.is_charging()
            && vex.get_wanted_pos().is_some()
            && vex
                .mob_entity
                .get_target()
                .is_some_and(|t| t.get_entity().is_alive())
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        if let Some(vex) = self.vex.upgrade() {
            vex.set_charging(false);
        }
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(vex) = self.vex.upgrade() else {
            return;
        };
        let Some(target) = vex.mob_entity.get_target() else {
            return;
        };

        let entity = &vex.mob_entity.living_entity.entity;
        let vex_bb = entity.bounding_box.load();
        let target_bb = target.get_entity().bounding_box.load();

        if vex_bb.intersects(&target_bb) {
            vex.mob_entity.try_attack(&*vex, target.get_entity());
            vex.set_charging(false);
            vex.set_wanted_pos(None);
        } else {
            let pos = entity.pos.load();
            let target_pos = target.get_entity().pos.load();
            if pos.squared_distance_to_vec(&target_pos) < 9.0 {
                vex.set_wanted_pos(Some(Vector3::new(
                    target_pos.x,
                    target_pos.y + 1.2,
                    target_pos.z,
                )));
            }
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

/// Random wandering around bound origin within [-7..7, -5..5, -7..7]
pub struct VexRandomMoveGoal {
    vex: Weak<VexEntity>,
}

impl VexRandomMoveGoal {
    #[must_use]
    pub const fn new(vex: Weak<VexEntity>) -> Self {
        Self { vex }
    }
}

impl Goal for VexRandomMoveGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(vex) = self.vex.upgrade() else {
            return false;
        };
        if vex.get_wanted_pos().is_some() {
            return false;
        }

        let mut rng = rand::rng();
        rng.random_range(0..7) == 0
    }

    fn start(&mut self, _mob: &dyn Mob) {
        let Some(vex) = self.vex.upgrade() else {
            return;
        };

        let entity = &vex.mob_entity.living_entity.entity;
        let bound = vex.get_bound_origin().unwrap_or_else(|| {
            let p = entity.pos.load();
            BlockPos::new(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)
        });

        let world = entity.world.load();
        let mut rng = rand::rng();

        for _ in 0..3 {
            let offset_x = rng.random_range(-7..=7);
            let offset_y = rng.random_range(-5..=5);
            let offset_z = rng.random_range(-7..=7);
            let test_pos = BlockPos::new(
                bound.0.x + offset_x,
                bound.0.y + offset_y,
                bound.0.z + offset_z,
            );

            if world.get_block_state(&test_pos).is_air() {
                vex.set_wanted_pos(Some(Vector3::new(
                    test_pos.0.x as f64 + 0.5,
                    test_pos.0.y as f64 + 0.5,
                    test_pos.0.z as f64 + 0.5,
                )));
                break;
            }
        }
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        false
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

/// Target Goal: Copy target of summoner (Evoker)
pub struct VexCopyOwnerTargetGoal {
    vex: Weak<VexEntity>,
}

impl VexCopyOwnerTargetGoal {
    #[must_use]
    pub const fn new(vex: Weak<VexEntity>) -> Self {
        Self { vex }
    }
}

impl Goal for VexCopyOwnerTargetGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(vex) = self.vex.upgrade() else {
            return false;
        };
        let Some(owner_uuid) = vex.get_owner_uuid() else {
            return false;
        };

        if vex.mob_entity.get_target().is_some() {
            return false;
        }

        let entity = &vex.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pos = entity.pos.load();

        for nearby in world.get_nearby_entities(pos, 32.0).into_values() {
            if nearby.get_entity().entity_uuid == owner_uuid {
                if let Some(mob) = nearby.get_mob() {
                    if let Some(target) = mob.get_mob_entity().get_target() {
                        if target.get_entity().is_alive() {
                            vex.mob_entity.set_target(Some(target));
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(vex) = self.vex.upgrade() else {
            return false;
        };
        vex.mob_entity
            .get_target()
            .is_some_and(|t| t.get_entity().is_alive())
    }

    fn controls(&self) -> Controls {
        Controls::empty()
    }
}
