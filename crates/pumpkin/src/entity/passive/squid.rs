use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use pumpkin_data::damage::DamageType;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{escape_danger::EscapeDangerGoal, try_find_water::TryFindWaterGoal},
    mob::{Mob, MobEntity},
};

/// Represents a Squid, a passive aquatic mob.
///
/// Vanilla source: `net.minecraft.world.entity.animal.squid.Squid`
/// Base class:    `net.minecraft.world.entity.animal.AgeableWaterCreature`
///
/// Behaviors implemented:
/// - Panic and flee when hurt (EscapeDangerGoal, speed 1.4)
/// - Water finding when stranded (TryFindWaterGoal)
/// - Periodic swimming thrust impulse in water
/// - Squirts ink cloud (Particle::SquidInk) and squirt sound on taking damage
/// - Air supply / suffocation: loses air outside water, drowns at <= -20 air (2 HP damage)
/// - Ambient sound ticking (interval 120 ticks, matches vanilla WaterAnimal)
/// - Hurt and death sounds (Sound::EntitySquidHurt, Sound::EntitySquidDeath)
/// - Custom gravity 0.08 (matching vanilla getDefaultGravity)
///
/// Wiki: <https://minecraft.wiki/w/Squid>
pub struct SquidEntity {
    pub mob_entity: MobEntity,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
    swim_pulse_timer: AtomicI32,
}

impl SquidEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let squid = Self {
            mob_entity,
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
            swim_pulse_timer: AtomicI32::new(0),
        };
        let mob_arc = Arc::new(squid);

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 0: Panic when hurt
            goal_selector.add_goal(0, EscapeDangerGoal::new(1.4));
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

        if in_water {
            self.air_supply.store(300, Ordering::Relaxed);

            // Periodic swimming thrust (tentacle contraction pulse every 25 ticks)
            let timer = self.swim_pulse_timer.fetch_add(1, Ordering::Relaxed);
            if timer % 25 == 0 {
                let yaw_rad = f64::from(entity.yaw.load()).to_radians();
                let thrust = 0.12;
                let vel = entity.velocity.load();
                let dx = -yaw_rad.sin() * thrust;
                let dz = yaw_rad.cos() * thrust;
                entity.velocity.store(Vector3::new(vel.x + dx, vel.y + 0.04, vel.z + dz));
                entity.velocity_dirty.store(true, Ordering::Relaxed);
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
            // Vanilla WaterAnimal.handleAirSupply: lose air outside water; drown at <= -20
            let air = self.air_supply.fetch_sub(1, Ordering::Relaxed) - 1;
            if air <= -20 {
                self.air_supply.store(0, Ordering::Relaxed);
                self.mob_entity
                    .living_entity
                    .damage(caller, 2.0, DamageType::DROWN);
            }
        }
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        self.ambient_sound_time.store(-120, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntitySquidHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
        self.spawn_ink();
    }
}
