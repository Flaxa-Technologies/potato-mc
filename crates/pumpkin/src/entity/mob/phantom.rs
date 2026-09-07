use std::f64::consts::PI;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, Weak};

use pumpkin_data::attributes::Attributes;
use pumpkin_data::damage::DamageType;
use pumpkin_data::entity::EntityType;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tracked_data;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::math::wrap_degrees;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{Controls, Goal},
    mob::{Mob, MobEntity},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhantomAttackPhase {
    Circle,
    Swoop,
}

pub struct PhantomEntity {
    pub mob_entity: MobEntity,
    pub size: AtomicI32,
    pub anchor_point: Mutex<Option<BlockPos>>,
    pub move_target_point: Mutex<Vector3<f64>>,
    pub attack_phase: Mutex<PhantomAttackPhase>,
    pub flap_sound_counter: AtomicI32,
    pub ambient_sound_timer: AtomicI32,
}

impl PhantomEntity {
    pub const XP_REWARD: u32 = 5;

    pub fn new(entity: Entity) -> Arc<Self> {
        let spawn_pos = entity.pos.load();
        let initial_anchor = BlockPos::new(
            spawn_pos.x.floor() as i32,
            spawn_pos.y.floor() as i32 + 5,
            spawn_pos.z.floor() as i32,
        );

        let mob_entity = MobEntity::new(entity);
        let phantom = Self {
            mob_entity,
            size: AtomicI32::new(0),
            anchor_point: Mutex::new(Some(initial_anchor)),
            move_target_point: Mutex::new(spawn_pos),
            attack_phase: Mutex::new(PhantomAttackPhase::Circle),
            flap_sound_counter: AtomicI32::new(0),
            ambient_sound_timer: AtomicI32::new(rand::random_range(80..160)),
        };
        let mob_arc = Arc::new(phantom);
        let mob_weak: Weak<PhantomEntity> = Arc::downgrade(&mob_arc);

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Flight / movement steering and flap sound (runs every tick)
            goal_selector.add_goal(0, Box::new(PhantomMoveControlGoal::new(mob_weak.clone())));

            // Attack phase coordinator (switches between Circle and Swoop)
            goal_selector.add_goal(
                1,
                Box::new(PhantomAttackStrategyGoal::new(mob_weak.clone())),
            );

            // Swoop dive attack against target
            goal_selector.add_goal(2, Box::new(PhantomSweepAttackGoal::new(mob_weak.clone())));

            // Circle around anchor in the sky
            goal_selector.add_goal(
                3,
                Box::new(PhantomCircleAroundAnchorGoal::new(mob_weak.clone())),
            );

            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Target selector: targets highest player in range
            target_selector.add_goal(
                1,
                Box::new(PhantomAttackPlayerTargetGoal::new(mob_weak.clone())),
            );
        }

        mob_arc
    }

    #[must_use]
    pub fn get_size(&self) -> i32 {
        self.size.load(Ordering::Relaxed)
    }

    pub fn set_size(&self, size: i32) {
        let clamped = size.clamp(0, 64);
        self.size.store(clamped, Ordering::Relaxed);
        let entity = &self.mob_entity.living_entity.entity;
        entity.set_synced_data(tracked_data::phantom::ID_SIZE, VarInt(clamped));

        // Damage: 6 + size
        let mut attributes = self
            .mob_entity
            .living_entity
            .attributes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(damage) = attributes.get_mut(&Attributes::ATTACK_DAMAGE.id) {
            damage.base_value = 6.0 + clamped as f64;
            damage.dirty.store(true, Ordering::Relaxed);
        }
    }

    #[must_use]
    pub fn get_anchor_point(&self) -> Option<BlockPos> {
        *self
            .anchor_point
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_anchor_point(&self, pos: Option<BlockPos>) {
        *self
            .anchor_point
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = pos;
    }

    #[must_use]
    pub fn get_move_target(&self) -> Vector3<f64> {
        *self
            .move_target_point
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_move_target(&self, target: Vector3<f64>) {
        *self
            .move_target_point
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = target;
    }

    #[must_use]
    pub fn get_attack_phase(&self) -> PhantomAttackPhase {
        *self
            .attack_phase
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn set_attack_phase(&self, phase: PhantomAttackPhase) {
        *self
            .attack_phase
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = phase;
    }
}

impl Mob for PhantomEntity {
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
        // Phantom is hurt: play hurt sound
        let entity = &self.mob_entity.living_entity.entity;
        entity.play_sound(Sound::EntityPhantomHurt);
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        let size = self.get_size();
        entity.set_synced_data(tracked_data::phantom::ID_SIZE, VarInt(size));
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        let size = self.get_size();
        nbt.put_int("size", size);
        nbt.put_int("Size", size);

        if let Some(anchor) = self.get_anchor_point() {
            nbt.put(
                "anchor_pos",
                pumpkin_nbt::tag::NbtTag::IntArray(vec![anchor.0.x, anchor.0.y, anchor.0.z]),
            );
            nbt.put_int("AX", anchor.0.x);
            nbt.put_int("AY", anchor.0.y);
            nbt.put_int("AZ", anchor.0.z);
        }
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        let size = nbt
            .get_int("size")
            .or_else(|| nbt.get_int("Size"))
            .unwrap_or(0);
        self.set_size(size);

        if let Some(anchor_arr) = nbt.get_int_array("anchor_pos") {
            if anchor_arr.len() == 3 {
                self.set_anchor_point(Some(BlockPos::new(
                    anchor_arr[0],
                    anchor_arr[1],
                    anchor_arr[2],
                )));
            }
        } else if let (Some(ax), Some(ay), Some(az)) =
            (nbt.get_int("AX"), nbt.get_int("AY"), nbt.get_int("AZ"))
        {
            self.set_anchor_point(Some(BlockPos::new(ax, ay, az)));
        }
    }
}

fn approach_degrees(from: f32, to: f32, max_delta: f32) -> f32 {
    let diff = wrap_degrees(to - from);
    from + diff.clamp(-max_delta, max_delta)
}

fn approach_f32(current: f32, target: f32, max_delta: f32) -> f32 {
    if current < target {
        (current + max_delta).min(target)
    } else {
        (current - max_delta).max(target)
    }
}

/// Movement control and flight dynamics matching vanilla `PhantomMoveControl`
pub struct PhantomMoveControlGoal {
    phantom: Weak<PhantomEntity>,
    speed: f32,
}

impl PhantomMoveControlGoal {
    #[must_use]
    pub const fn new(phantom: Weak<PhantomEntity>) -> Self {
        Self {
            phantom,
            speed: 0.1,
        }
    }
}

impl Goal for PhantomMoveControlGoal {
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
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };

        let entity = &phantom.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let target_pt = phantom.get_move_target();

        // 1. Check collision turn-around
        if entity.horizontal_collision.load(Ordering::Relaxed) {
            let current_yaw = entity.yaw.load();
            let new_yaw = wrap_degrees(current_yaw + 180.0);
            entity.yaw.store(new_yaw);
            entity.head_yaw.store(new_yaw);
            entity.body_yaw.store(new_yaw);
            self.speed = 0.1;
        }

        // 2. Flight steering calculation towards target_pt
        let mut tdx = target_pt.x - pos.x;
        let tdy = target_pt.y - pos.y;
        let mut tdz = target_pt.z - pos.z;
        let mut sd = (tdx * tdx + tdz * tdz).sqrt();

        if sd.abs() > 1e-5 {
            let y_rel_scale = 1.0 - (tdy * 0.7).abs() / sd;
            tdx *= y_rel_scale;
            tdz *= y_rel_scale;
            sd = (tdx * tdx + tdz * tdz).sqrt();
            let sd2 = (tdx * tdx + tdz * tdz + tdy * tdy).sqrt();

            let prev_yaw = entity.yaw.load();
            let angle = tdz.atan2(tdx).to_degrees() as f32;
            let a = wrap_degrees(prev_yaw + 90.0);
            let b = wrap_degrees(angle);
            let new_yaw = approach_degrees(a, b, 4.0) - 90.0;
            entity.yaw.store(new_yaw);
            entity.head_yaw.store(new_yaw);
            entity.body_yaw.store(new_yaw);

            let diff = wrap_degrees(prev_yaw - new_yaw).abs();
            if diff < 3.0 {
                let speed_ratio = if self.speed > 0.001 {
                    1.8 / self.speed
                } else {
                    1.8
                };
                self.speed = approach_f32(self.speed, 1.8, 0.005 * speed_ratio);
            } else {
                self.speed = approach_f32(self.speed, 0.2, 0.025);
            }

            let x_rot_d = -((-tdy).atan2(sd).to_degrees() as f32);
            entity.pitch.store(x_rot_d);

            let move_angle = (new_yaw + 90.0).to_radians() as f64;
            let speed_f64 = self.speed as f64;
            let txd = speed_f64 * move_angle.cos() * (tdx / sd2).abs();
            let tzd = speed_f64 * move_angle.sin() * (tdz / sd2).abs();
            let tyd = speed_f64 * (x_rot_d.to_radians() as f64).sin() * (tdy / sd2).abs();

            let current_vel = entity.velocity.load();
            let target_vel = Vector3::new(txd, tyd, tzd);
            let new_vel = current_vel + (target_vel - current_vel) * 0.2;
            entity.velocity.store(new_vel);
        }

        // 3. Flap sound every 25 ticks
        let count = phantom.flap_sound_counter.fetch_add(1, Ordering::Relaxed);
        let flap_offset = entity.entity_id * 3;
        if (count + flap_offset) % 25 == 0 {
            let world = entity.world.load();
            let mut rng = rand::rng();
            let pitch = 0.95 + rng.random::<f32>() * 0.05;
            let vol = 0.95 + rng.random::<f32>() * 0.05;
            world.play_sound_fine(
                Sound::EntityPhantomFlap,
                SoundCategory::Hostile,
                &pos,
                vol,
                pitch,
            );
        }

        // 4. Ambient sound periodically
        let ambient_time = phantom
            .ambient_sound_timer
            .fetch_sub(1, Ordering::Relaxed);
        if ambient_time <= 0 {
            phantom
                .ambient_sound_timer
                .store(rand::random_range(100..200), Ordering::Relaxed);
            let world = entity.world.load();
            let mut rng = rand::rng();
            let pitch = 0.95 + rng.random::<f32>() * 0.1;
            world.play_sound_fine(
                Sound::EntityPhantomAmbient,
                SoundCategory::Hostile,
                &pos,
                1.0,
                pitch,
            );
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }
}

/// Attack Strategy Goal: coordinates Circle vs Swoop phases
pub struct PhantomAttackStrategyGoal {
    phantom: Weak<PhantomEntity>,
    next_sweep_tick: i32,
}

impl PhantomAttackStrategyGoal {
    #[must_use]
    pub const fn new(phantom: Weak<PhantomEntity>) -> Self {
        Self {
            phantom,
            next_sweep_tick: 0,
        }
    }

    fn set_anchor_above_target(&self, phantom: &PhantomEntity) {
        if let Some(target) = phantom.mob_entity.get_target() {
            let target_pos = target.get_entity().pos.load();
            let mut rng = rand::rng();
            let height_above = 20 + rng.random_range(0..20);
            let anchor_y = (target_pos.y.floor() as i32 + height_above).max(64);
            phantom.set_anchor_point(Some(BlockPos::new(
                target_pos.x.floor() as i32,
                anchor_y,
                target_pos.z.floor() as i32,
            )));
        }
    }
}

impl Goal for PhantomAttackStrategyGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        phantom
            .mob_entity
            .get_target()
            .is_some_and(|t| t.get_entity().is_alive())
    }

    fn start(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };
        self.next_sweep_tick = 10;
        phantom.set_attack_phase(PhantomAttackPhase::Circle);
        self.set_anchor_above_target(&phantom);
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        phantom
            .mob_entity
            .get_target()
            .is_some_and(|t| t.get_entity().is_alive())
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };
        if let Some(anchor) = phantom.get_anchor_point() {
            let mut rng = rand::rng();
            let new_anchor = BlockPos::new(
                anchor.0.x,
                anchor.0.y + 10 + rng.random_range(0..20),
                anchor.0.z,
            );
            phantom.set_anchor_point(Some(new_anchor));
        }
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };

        if phantom.get_attack_phase() == PhantomAttackPhase::Circle {
            self.next_sweep_tick -= 1;
            if self.next_sweep_tick <= 0 {
                phantom.set_attack_phase(PhantomAttackPhase::Swoop);
                self.set_anchor_above_target(&phantom);
                let mut rng = rand::rng();
                self.next_sweep_tick = (8 + rng.random_range(0..4)) * 20;

                // Play Swoop sound
                let entity = &phantom.mob_entity.living_entity.entity;
                let world = entity.world.load();
                let pos = entity.pos.load();
                world.play_sound_fine(
                    Sound::EntityPhantomSwoop,
                    SoundCategory::Hostile,
                    &pos,
                    10.0,
                    0.95 + rng.random::<f32>() * 0.1,
                );
            }
        }
    }

    fn controls(&self) -> Controls {
        Controls::empty()
    }
}

/// Circling Goal: Circles around the anchor point in the sky
pub struct PhantomCircleAroundAnchorGoal {
    phantom: Weak<PhantomEntity>,
    angle: f64,
    distance: f64,
    height: f64,
    clockwise: f64,
}

impl PhantomCircleAroundAnchorGoal {
    #[must_use]
    pub fn new(phantom: Weak<PhantomEntity>) -> Self {
        Self {
            phantom,
            angle: 0.0,
            distance: 5.0,
            height: 0.0,
            clockwise: 1.0,
        }
    }

    fn select_next(&mut self, phantom: &PhantomEntity) {
        let anchor = phantom.get_anchor_point().unwrap_or_else(|| {
            let pos = phantom.mob_entity.living_entity.entity.pos.load();
            BlockPos::new(
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            )
        });

        self.angle += self.clockwise * 15.0 * (PI / 180.0);
        let target = Vector3::new(
            anchor.0.x as f64 + 0.5 + self.distance * self.angle.cos(),
            anchor.0.y as f64 - 4.0 + self.height,
            anchor.0.z as f64 + 0.5 + self.distance * self.angle.sin(),
        );
        phantom.set_move_target(target);
    }
}

impl Goal for PhantomCircleAroundAnchorGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        phantom.mob_entity.get_target().is_none()
            || phantom.get_attack_phase() == PhantomAttackPhase::Circle
    }

    fn start(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };
        let mut rng = rand::rng();
        self.distance = 5.0 + rng.random::<f64>() * 10.0;
        self.height = -4.0 + rng.random::<f64>() * 9.0;
        self.clockwise = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        self.angle = rng.random::<f64>() * 2.0 * PI;
        self.select_next(&phantom);
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        phantom.mob_entity.get_target().is_none()
            || phantom.get_attack_phase() == PhantomAttackPhase::Circle
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };

        let mut rng = rand::rng();
        if rng.random_range(0..350) == 0 {
            self.height = -4.0 + rng.random::<f64>() * 9.0;
        }

        if rng.random_range(0..250) == 0 {
            self.distance += 1.0;
            if self.distance > 15.0 {
                self.distance = 5.0;
                self.clockwise = -self.clockwise;
            }
        }

        if rng.random_range(0..450) == 0 {
            self.angle = rng.random::<f64>() * 2.0 * PI;
            self.select_next(&phantom);
        }

        let entity = &phantom.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let target = phantom.get_move_target();

        if pos.squared_distance_to_vec(&target) < 4.0 {
            self.select_next(&phantom);
        }

        // Adjust height if colliding with blocks
        let world = entity.world.load();
        let current_block_pos = BlockPos::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );

        if target.y < pos.y {
            let below_pos = BlockPos::new(
                current_block_pos.0.x,
                current_block_pos.0.y - 1,
                current_block_pos.0.z,
            );
            if !world.get_block_state(&below_pos).is_air() {
                self.height = self.height.max(1.0);
                self.select_next(&phantom);
            }
        }

        if target.y > pos.y {
            let above_pos = BlockPos::new(
                current_block_pos.0.x,
                current_block_pos.0.y + 1,
                current_block_pos.0.z,
            );
            if !world.get_block_state(&above_pos).is_air() {
                self.height = self.height.min(-1.0);
                self.select_next(&phantom);
            }
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

/// Swoop Attack Goal: dives down, bites player, fears cats
pub struct PhantomSweepAttackGoal {
    phantom: Weak<PhantomEntity>,
    cat_search_tick: i32,
    is_scared_of_cat: bool,
}

impl PhantomSweepAttackGoal {
    #[must_use]
    pub const fn new(phantom: Weak<PhantomEntity>) -> Self {
        Self {
            phantom,
            cat_search_tick: 0,
            is_scared_of_cat: false,
        }
    }
}

impl Goal for PhantomSweepAttackGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        phantom
            .mob_entity
            .get_target()
            .is_some_and(|t| t.get_entity().is_alive())
            && phantom.get_attack_phase() == PhantomAttackPhase::Swoop
    }

    fn start(&mut self, _mob: &dyn Mob) {
        self.is_scared_of_cat = false;
        self.cat_search_tick = 0;
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };

        if self.is_scared_of_cat {
            return false;
        }

        let Some(target) = phantom.mob_entity.get_target() else {
            return false;
        };

        if !target.get_entity().is_alive() {
            return false;
        }

        phantom.get_attack_phase() == PhantomAttackPhase::Swoop
    }

    fn stop(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };
        if self.is_scared_of_cat {
            phantom.mob_entity.set_target(None);
        }
        phantom.set_attack_phase(PhantomAttackPhase::Circle);
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(phantom) = self.phantom.upgrade() else {
            return;
        };
        let Some(target) = phantom.mob_entity.get_target() else {
            return;
        };

        let entity = &phantom.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let target_pos = target.get_entity().pos.load();

        // 1. Cat fear check every 20 ticks
        self.cat_search_tick -= 1;
        if self.cat_search_tick <= 0 {
            self.cat_search_tick = 20;
            let my_pos = entity.pos.load();
            let cat_detected = world
                .get_nearby_entities(my_pos, 16.0)
                .into_values()
                .find(|ent| {
                    let id = ent.get_entity().entity_type.id;
                    id == EntityType::CAT.id || id == EntityType::OCELOT.id
                });

            if let Some(cat) = cat_detected {
                let cat_pos = cat.get_entity().pos.load();
                world.play_sound_fine(
                    Sound::EntityCatHiss,
                    SoundCategory::Neutral,
                    &cat_pos,
                    1.0,
                    1.0,
                );
                self.is_scared_of_cat = true;
                phantom.set_attack_phase(PhantomAttackPhase::Circle);
                phantom.mob_entity.set_target(None);
                return;
            }
        }

        // 2. Steer dive toward target
        let target_dive_point = Vector3::new(target_pos.x, target_pos.y + 0.9, target_pos.z);
        phantom.set_move_target(target_dive_point);

        // 3. Attack check: does phantom touch target?
        let phantom_bb = entity.bounding_box.load().expand(0.2, 0.2, 0.2);
        let target_bb = target.get_entity().bounding_box.load();

        if phantom_bb.intersects(&target_bb) {
            // Hurt target
            phantom
                .mob_entity
                .try_attack(&*phantom, target.get_entity());

            // Play phantom bite sound
            let pos = entity.pos.load();
            world.play_sound_fine(
                Sound::EntityPhantomBite,
                SoundCategory::Hostile,
                &pos,
                1.0,
                1.0,
            );

            phantom.set_attack_phase(PhantomAttackPhase::Circle);
        } else if entity.horizontal_collision.load(Ordering::Relaxed)
            || phantom
                .mob_entity
                .living_entity
                .hurt_cooldown
                .load(Ordering::Relaxed)
                > 0
        {
            phantom.set_attack_phase(PhantomAttackPhase::Circle);
        }
    }

    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

/// Attack Player Target Goal: targets highest player in range
pub struct PhantomAttackPlayerTargetGoal {
    phantom: Weak<PhantomEntity>,
    next_scan_tick: i32,
}

impl PhantomAttackPlayerTargetGoal {
    #[must_use]
    pub const fn new(phantom: Weak<PhantomEntity>) -> Self {
        Self {
            phantom,
            next_scan_tick: 20,
        }
    }
}

impl Goal for PhantomAttackPlayerTargetGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        self.next_scan_tick -= 1;
        if self.next_scan_tick > 0 {
            return false;
        }
        self.next_scan_tick = 40;

        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };

        if phantom.mob_entity.get_target().is_some() {
            return false;
        }

        let entity = &phantom.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let world = entity.world.load();

        let mut candidate_players = Vec::new();
        for player in world.get_nearby_players(pos, 64.0) {
            let gm = player.gamemode.load();
            if gm == GameMode::Creative || gm == GameMode::Spectator {
                continue;
            }
            if !player.living_entity.entity.is_alive() {
                continue;
            }
            let p_pos = player.living_entity.entity.pos.load();
            if (p_pos.x - pos.x).abs() <= 16.0 && (p_pos.z - pos.z).abs() <= 16.0 {
                candidate_players.push(player);
            }
        }

        if candidate_players.is_empty() {
            return false;
        }

        // Sort by Y descending (highest player first)
        candidate_players.sort_by(|a, b| {
            let ay = a.living_entity.entity.pos.load().y;
            let by = b.living_entity.entity.pos.load().y;
            by.partial_cmp(&ay).unwrap_or(std::cmp::Ordering::Equal)
        });

        let highest_player = candidate_players.remove(0);
        phantom.mob_entity.set_target(Some(highest_player));
        true
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(phantom) = self.phantom.upgrade() else {
            return false;
        };
        let Some(target) = phantom.mob_entity.get_target() else {
            return false;
        };
        target.get_entity().is_alive()
    }

    fn controls(&self) -> Controls {
        Controls::empty()
    }
}
