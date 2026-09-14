use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use crossbeam::atomic::AtomicCell;
use pumpkin_data::damage::DamageType;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::try_find_water::TryFindWaterGoal,
    mob::{Mob, MobEntity},
};

/// Represents a Squid, a passive aquatic mob.
///
/// Vanilla source: `net.minecraft.world.entity.animal.squid.Squid`
/// Base class:    `net.minecraft.world.entity.animal.AgeableWaterCreature`
///
/// Behaviors implemented:
/// - SquidRandomMovementGoal: periodic direction change with gentle horizontal/vertical thrust
/// - SquidFleeGoal: flees from attacker on taking damage with accelerated swimming impulse
/// - Tentacle propulsion cycle (tentacle_movement 0..2*PI) with entity event 19 sync
/// - Water finding when stranded (TryFindWaterGoal)
/// - Squirts ink cloud (Particle::SquidInk) and squirt sound on taking damage
/// - Air supply / suffocation: loses air outside water, drowns at <= -20 air (2 HP damage)
/// - Ambient sound ticking (interval 120 ticks, matches vanilla WaterAnimal)
/// - Hurt and death sounds (Sound::EntitySquidHurt, Sound::EntitySquidDeath)
/// - Custom gravity 0.08 (matching vanilla getDefaultGravity) in air; overrides travel in water
///
/// Wiki: <https://minecraft.wiki/w/Squid>
pub struct SquidEntity {
    pub mob_entity: MobEntity,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
    swim_pulse_timer: AtomicI32,
    tentacle_movement: AtomicCell<f32>,
    tentacle_speed: AtomicCell<f32>,
    movement_vector: AtomicCell<Vector3<f64>>,
    rotate_speed: AtomicCell<f32>,
}

impl SquidEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mut rng = rand::rng();
        let tentacle_speed = 1.0 / (rng.random::<f32>() + 1.0) * 0.2;
        let angle = rng.random::<f32>() * std::f32::consts::PI * 2.0;
        let initial_mv = Vector3::new(
            f64::from(angle.cos()) * 0.2,
            -0.1 + f64::from(rng.random::<f32>()) * 0.2,
            f64::from(angle.sin()) * 0.2,
        );
        entity.velocity.store(initial_mv * 0.5);
        let mob_entity = MobEntity::new(entity);

        let squid = Self {
            mob_entity,
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
            swim_pulse_timer: AtomicI32::new(0),
            tentacle_movement: AtomicCell::new(0.0),
            tentacle_speed: AtomicCell::new(tentacle_speed),
            movement_vector: AtomicCell::new(initial_mv),
            rotate_speed: AtomicCell::new(0.0),
        };
        let mob_arc = Arc::new(squid);

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 1: Seek water when stranded on land
            goal_selector.add_goal(1, Box::new(TryFindWaterGoal));
        };

        mob_arc
    }

    /// Spawns an ink cloud around the squid and plays the squirt sound.
    /// Matches vanilla `Squid.spawnInk()`.
    pub fn spawn_ink(&self) {
        let entity = self.get_entity();
        let world = entity.world.load();
        let pos = entity.pos.load();

        world.play_sound(
            Sound::EntitySquidSquirt,
            SoundCategory::Neutral,
            &pos,
        );

        let mut rng = rand::rng();
        for _ in 0..20 {
            let offset = Vector3::new(
                rng.random_range(-0.4..0.4),
                rng.random_range(-0.3..0.3),
                rng.random_range(-0.4..0.4),
            );
            world.spawn_particle(
                pos + offset,
                Vector3::new(0.1, 0.1, 0.1),
                0.05,
                1,
                Particle::SquidInk,
            );
        }
    }
}

impl Mob for SquidEntity {
    /// Vanilla Animal.java:128 / AbstractGolem.java:36 -- passive mobs never despawn naturally.
    fn remove_when_far_away(&self, _distance_sq: f64) -> bool { false }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_mob_gravity(&self) -> f64 {
        // Vanilla Squid.getDefaultGravity() = 0.08
        0.08
    }

    fn mob_write_nbt(&self, _nbt: &mut NbtCompound) {}

    fn mob_read_nbt(&self, _nbt: &NbtCompound) {}

    fn mob_tick(&self, caller: &dyn EntityBase) {
        let entity = self.get_entity();
        let in_water = entity.is_in_water() || entity.touching_water.load(Ordering::Relaxed);

        // Vanilla: tentacle movement progresses each tick
        let tentacle_speed = self.tentacle_speed.load();
        let mut tentacle_movement = self.tentacle_movement.load() + tentacle_speed;
        if tentacle_movement > std::f32::consts::PI * 2.0 {
            tentacle_movement -= std::f32::consts::PI * 2.0;
            if rand::random_range(0..10) == 0 {
                self.tentacle_speed.store(1.0 / (rand::random::<f32>() + 1.0) * 0.2);
            }
            let world = entity.world.load();
            world.broadcast_packet_all(&pumpkin_protocol::java::client::play::CEntityStatus::new(
                entity.entity_id,
                19,
            ));
        }
        self.tentacle_movement.store(tentacle_movement);

        if in_water {
            self.air_supply.store(300, Ordering::Relaxed);

            let mut rotate_speed = self.rotate_speed.load();
            if tentacle_movement < std::f32::consts::PI {
                let tentacle_scale = tentacle_movement / std::f32::consts::PI;
                if tentacle_scale > 0.75 {
                    let mv = self.movement_vector.load();
                    entity.velocity.store(mv);
                    entity.velocity_dirty.store(true, Ordering::Relaxed);
                    rotate_speed = 1.0;
                } else {
                    rotate_speed *= 0.8;
                }
            } else {
                let mut vel = entity.velocity.load();
                vel.x *= 0.9;
                vel.y *= 0.9;
                vel.z *= 0.9;
                entity.velocity.store(vel);
                entity.velocity_dirty.store(true, Ordering::Relaxed);
                rotate_speed *= 0.99;
            }
            self.rotate_speed.store(rotate_speed);

            // Periodic random movement vector selection matching SquidRandomMovementGoal
            let timer = self.swim_pulse_timer.fetch_add(1, Ordering::Relaxed);
            let mv = self.movement_vector.load();
            let has_mv = mv.length_squared() > 1.0e-5;
            if timer % 50 == 0 || !has_mv {
                let mut rng = rand::rng();
                let angle = rng.random::<f32>() * std::f32::consts::PI * 2.0;
                let new_mv = Vector3::new(
                    f64::from(angle.cos()) * 0.2,
                    -0.1 + f64::from(rng.random::<f32>()) * 0.2,
                    f64::from(angle.sin()) * 0.2,
                );
                self.movement_vector.store(new_mv);
            }

            // Orientation: smooth yaw tracking movement direction
            let vel = entity.velocity.load();
            let horiz = (vel.x * vel.x + vel.z * vel.z).sqrt();
            if horiz > 0.005 {
                let target_yaw = (-vel.x.atan2(vel.z).to_degrees()) as f32;
                let cur_yaw = entity.yaw.load();
                let diff = ((target_yaw - cur_yaw + 180.0).rem_euclid(360.0) - 180.0) * 0.1;
                let new_yaw = cur_yaw + diff;
                entity.set_rotation(new_yaw, 0.0);
                entity.head_yaw.store(new_yaw);
            }

            // Ambient sound (vanilla WaterAnimal.AMBIENT_SOUND_INTERVAL = 120)
            let sound_timer = self.ambient_sound_time.fetch_add(1, Ordering::Relaxed);
            if sound_timer > 0 && rand::random_range(0..1000) < sound_timer {
                self.ambient_sound_time.store(-120, Ordering::Relaxed);
                entity.world.load().play_sound_fine(
                    Sound::EntitySquidAmbient,
                    SoundCategory::Neutral,
                    &entity.pos.load(),
                    0.4,
                    (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0,
                );
            }
        } else {
            // Out of water: lose air; drown at <= -20 air (vanilla WaterAnimal.handleAirSupply)
            let air = self.air_supply.fetch_sub(1, Ordering::Relaxed) - 1;
            if air <= -20 {
                self.air_supply.store(0, Ordering::Relaxed);
                self.mob_entity
                    .living_entity
                    .damage(caller, 2.0, DamageType::DROWN);
            }
        }
    }

    fn on_damage(&self, _damage_type: DamageType, source: Option<&dyn EntityBase>) {
        self.ambient_sound_time.store(-120, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntitySquidHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
        self.spawn_ink();

        // Flee from attacker matching vanilla SquidFleeGoal
        if let Some(src) = source {
            let src_pos = src.get_entity().pos.load();
            let pos = entity.pos.load();
            let mut flee = pos - src_pos;
            let len = flee.length();
            if len > 0.001 {
                flee = flee.normalize();
                let mut speed = 3.0;
                if len > 5.0 {
                    speed -= (len - 5.0) / 5.0;
                }
                if speed > 0.0 {
                    flee = flee * speed;
                }
                self.movement_vector.store(Vector3::new(flee.x / 20.0, flee.y / 20.0, flee.z / 20.0));
            }
        }
    }
}
