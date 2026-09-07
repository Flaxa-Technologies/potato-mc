use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use pumpkin_data::damage::DamageType;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{escape_danger::EscapeDangerGoal, try_find_water::TryFindWaterGoal},
    mob::{Mob, MobEntity},
};

/// Represents a Glow Squid, a deep-water passive aquatic mob that emits a glowing particle aura.
///
/// Vanilla source: `net.minecraft.world.entity.animal.squid.GlowSquid`
/// Base class:    `net.minecraft.world.entity.animal.squid.Squid`
///
/// Behaviors implemented:
/// - Panic and flee when hurt (EscapeDangerGoal, speed 1.4)
/// - Water finding when stranded (TryFindWaterGoal)
/// - Periodic swimming thrust impulse in water
/// - Squirts glow ink cloud (Particle::GlowSquidInk) and squirt sound on taking damage
/// - Dark ticks mechanism: stops glowing for 100 ticks after taking damage (DATA_DARK_TICKS_REMAINING)
/// - Emits glowing aura particles (Particle::Glow) when not darkened
/// - Air supply / suffocation: loses air outside water, drowns at <= -20 air (2 HP damage)
/// - Ambient sound ticking (interval 120 ticks, Sound::EntityGlowSquidAmbient)
/// - Hurt and death sounds (Sound::EntityGlowSquidHurt, Sound::EntityGlowSquidDeath)
/// - Custom gravity 0.08 (matching vanilla getDefaultGravity)
///
/// Wiki: <https://minecraft.wiki/w/Glow_Squid>
pub struct GlowSquidEntity {
    pub mob_entity: MobEntity,
    pub dark_ticks_remaining: AtomicI32,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
    swim_pulse_timer: AtomicI32,
}

impl GlowSquidEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let glow_squid = Self {
            mob_entity,
            dark_ticks_remaining: AtomicI32::new(0),
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
            swim_pulse_timer: AtomicI32::new(0),
        };
        let mob_arc = Arc::new(glow_squid);

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

    #[must_use]
    pub fn get_dark_ticks(&self) -> i32 {
        self.dark_ticks_remaining.load(Ordering::Relaxed)
    }

    pub fn set_dark_ticks(&self, ticks: i32) {
        self.dark_ticks_remaining.store(ticks, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::glow_squid::DATA_DARK_TICKS_REMAINING,
            VarInt(ticks),
        );
    }

    /// Spawns a glowing ink cloud around the squid and plays the glow squirt sound.
    /// Matches vanilla `GlowSquid.spawnInk()`.
    pub fn spawn_ink(&self) {
        let entity = self.get_entity();
        let world = entity.world.load();
        let pos = entity.pos.load();

        world.play_sound(
            Sound::EntityGlowSquidSquirt,
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
                Particle::GlowSquidInk,
            );
        }
    }
}

impl Mob for GlowSquidEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_mob_gravity(&self) -> f64 {
        0.08
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_int("DarkTicksRemaining", self.get_dark_ticks());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(ticks) = nbt.get_int("DarkTicksRemaining") {
            self.set_dark_ticks(ticks);
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::glow_squid::DATA_DARK_TICKS_REMAINING,
            VarInt(self.get_dark_ticks()),
        );
    }

    fn mob_tick(&self, caller: &dyn EntityBase) {
        let entity = self.get_entity();
        let in_water = entity.is_in_water() || entity.touching_water.load(Ordering::Relaxed);

        if in_water {
            self.air_supply.store(300, Ordering::Relaxed);

            // Tick down dark ticks (vanilla GlowSquid.aiStep)
            let dark = self.dark_ticks_remaining.load(Ordering::Relaxed);
            if dark > 0 {
                self.set_dark_ticks(dark - 1);
            } else {
                // Emit glowing aura particles while glowing
                let world = entity.world.load();
                let pos = entity.pos.load();
                let mut rng = rand::rng();
                let offset = Vector3::new(
                    rng.random_range(-0.4..0.4),
                    rng.random_range(0.0..f64::from(entity.height())),
                    rng.random_range(-0.4..0.4),
                );
                world.spawn_particle(
                    pos + offset,
                    Vector3::new(0.0, 0.0, 0.0),
                    0.0,
                    1,
                    Particle::Glow,
                );
            }

            // Periodic swimming thrust
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
                    Sound::EntityGlowSquidAmbient,
                    SoundCategory::Neutral,
                    &entity.pos.load(),
                    0.4,
                    (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0,
                );
            }
        } else {
            // Suffocation outside of water
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
        self.set_dark_ticks(100);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntityGlowSquidHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
        self.spawn_ink();
    }
}
