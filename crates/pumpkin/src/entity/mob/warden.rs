use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Weak};

use pumpkin_data::attributes::Attributes;
use pumpkin_data::damage::DamageType;
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::entity::EntityType;
use pumpkin_data::particle::Particle;
use pumpkin_data::potion::Effect;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tracked_data;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::GameMode;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::ai::goal::active_target::ActiveTargetGoal;
use crate::entity::ai::goal::look_around::RandomLookAroundGoal;
use crate::entity::ai::goal::look_at_entity::LookAtEntityGoal;
use crate::entity::ai::goal::melee_attack::MeleeAttackGoal;
use crate::entity::ai::goal::revenge::RevengeGoal;
use crate::entity::ai::goal::swim::SwimGoal;
use crate::entity::ai::goal::wander_around::WanderAroundGoal;
use crate::entity::{
    Entity, EntityBase,
    ai::goal::{Controls, Goal},
    mob::{Mob, MobEntity},
};

pub struct WardenEntity {
    pub mob_entity: MobEntity,
    pub anger_level: AtomicI32,
    pub sonic_boom_cooldown: AtomicI32,
    pub sonic_charge_ticks: AtomicI32,
    pub darkness_timer: AtomicI32,
    pub heartbeat_timer: AtomicI32,
    pub sniff_timer: AtomicI32,
    pub out_of_melee_reach_ticks: AtomicI32,
}

impl WardenEntity {
    pub const XP_REWARD: u32 = 5;
    pub const MAX_HEALTH: f64 = 500.0;
    pub const ATTACK_DAMAGE: f64 = 30.0;
    pub const ATTACK_KNOCKBACK: f64 = 1.5;
    pub const KNOCKBACK_RESISTANCE: f64 = 1.0;
    pub const MOVEMENT_SPEED: f64 = 0.3;

    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);

        // Configure vanilla attributes
        {
            let mut attributes = mob_entity
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            if let Some(health) = attributes.get_mut(&Attributes::MAX_HEALTH.id) {
                health.base_value = Self::MAX_HEALTH;
                health.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(damage) = attributes.get_mut(&Attributes::ATTACK_DAMAGE.id) {
                damage.base_value = Self::ATTACK_DAMAGE;
                damage.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(kb) = attributes.get_mut(&Attributes::ATTACK_KNOCKBACK.id) {
                kb.base_value = Self::ATTACK_KNOCKBACK;
                kb.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(kb_res) = attributes.get_mut(&Attributes::KNOCKBACK_RESISTANCE.id) {
                kb_res.base_value = Self::KNOCKBACK_RESISTANCE;
                kb_res.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(speed) = attributes.get_mut(&Attributes::MOVEMENT_SPEED.id) {
                speed.base_value = Self::MOVEMENT_SPEED;
                speed.dirty.store(true, Ordering::Relaxed);
            }
        }
        mob_entity
            .living_entity
            .health
            .store(Self::MAX_HEALTH as f32);

        let warden = Self {
            mob_entity,
            anger_level: AtomicI32::new(0),
            sonic_boom_cooldown: AtomicI32::new(40),
            sonic_charge_ticks: AtomicI32::new(0),
            darkness_timer: AtomicI32::new(120),
            heartbeat_timer: AtomicI32::new(40),
            sniff_timer: AtomicI32::new(rand::random_range(100..200)),
            out_of_melee_reach_ticks: AtomicI32::new(0),
        };

        let mob_arc = Arc::new(warden);
        let mob_weak: Weak<WardenEntity> = Arc::downgrade(&mob_arc);
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

            // Priority 0: Swim
            goal_selector.add_goal(0, Box::new(SwimGoal::default()));

            // Priority 1: Sonic Boom ranged attack
            goal_selector.add_goal(1, Box::new(WardenSonicBoomGoal::new(mob_weak.clone())));

            // Priority 2: Melee attack (speed 1.2, 30 damage)
            goal_selector.add_goal(2, Box::new(MeleeAttackGoal::new(1.2, false)));

            // Priority 3: Heartbeat, darkness aura, ambient sniffing, and anger decay
            goal_selector.add_goal(
                3,
                Box::new(WardenHeartbeatAndDarknessGoal::new(mob_weak.clone())),
            );

            // Priority 5: Wander around
            goal_selector.add_goal(5, Box::new(WanderAroundGoal::new(0.6)));

            // Priority 6: Look at player
            goal_selector.add_goal(
                6,
                LookAtEntityGoal::with_default(mob_trait_weak.clone(), &EntityType::PLAYER, 16.0),
            );

            // Priority 7: Look around
            goal_selector.add_goal(7, Box::new(RandomLookAroundGoal::default()));

            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 1: Revenge (on hurt: increase anger by 55 and target attacker)
            target_selector.add_goal(1, Box::new(RevengeGoal::new(true)));

            // Priority 2: Target players within range
            target_selector.add_goal(
                2,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
        }

        mob_arc
    }

    #[must_use]
    pub fn get_anger_level(&self) -> i32 {
        self.anger_level.load(Ordering::Relaxed)
    }

    pub fn set_anger_level(&self, anger: i32) {
        let clamped = anger.clamp(0, 150);
        self.anger_level.store(clamped, Ordering::Relaxed);
        let entity = &self.mob_entity.living_entity.entity;
        entity.set_synced_data(
            tracked_data::warden::CLIENT_ANGER_LEVEL,
            VarInt(clamped),
        );
    }

    pub fn increase_anger_at(&self, amount: i32) {
        let old = self.get_anger_level();
        let new_anger = (old + amount).clamp(0, 150);
        self.set_anger_level(new_anger);

        let entity = &self.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let world = entity.world.load();

        if old < 80 && new_anger >= 80 {
            // Reached Angry state: roar!
            world.play_sound_fine(
                Sound::EntityWardenRoar,
                SoundCategory::Hostile,
                &pos,
                10.0,
                1.0,
            );
        } else if new_anger >= 40 {
            world.play_sound_fine(
                Sound::EntityWardenListeningAngry,
                SoundCategory::Hostile,
                &pos,
                5.0,
                1.0,
            );
        } else {
            world.play_sound_fine(
                Sound::EntityWardenListening,
                SoundCategory::Hostile,
                &pos,
                5.0,
                1.0,
            );
        }
    }

    pub fn decrease_anger(&self, amount: i32) {
        let old = self.get_anger_level();
        let new_anger = (old - amount).max(0);
        self.set_anger_level(new_anger);
    }

    #[must_use]
    pub fn get_heartbeat_delay(&self) -> i32 {
        let anger = self.get_anger_level().min(80) as f32;
        40 - (anger / 80.0 * 30.0) as i32
    }
}

impl Mob for WardenEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_base_experience_reward(&self) -> u32 {
        Self::XP_REWARD
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        // Being attacked drastically increases anger
        self.increase_anger_at(55);
        let entity = &self.mob_entity.living_entity.entity;
        entity.play_sound(Sound::EntityWardenHurt);
    }

    fn on_attack(&self, _target: &dyn EntityBase) {
        let entity = &self.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let world = entity.world.load();
        world.play_sound_fine(
            Sound::EntityWardenAttackImpact,
            SoundCategory::Hostile,
            &pos,
            10.0,
            1.0,
        );
        // Reset sonic boom cooldown on successful melee hit
        self.sonic_boom_cooldown.store(40, Ordering::Relaxed);
        self.out_of_melee_reach_ticks.store(0, Ordering::Relaxed);
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            tracked_data::warden::CLIENT_ANGER_LEVEL,
            VarInt(self.get_anger_level()),
        );
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_int("anger", self.get_anger_level());
        nbt.put_int("Anger", self.get_anger_level());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(anger) = nbt.get_int("anger").or_else(|| nbt.get_int("Anger")) {
            self.set_anger_level(anger);
        }
    }
}

/// Heartbeat, Darkness aura, ambient sniffing, and anger decay goal
pub struct WardenHeartbeatAndDarknessGoal {
    warden: Weak<WardenEntity>,
}

impl WardenHeartbeatAndDarknessGoal {
    #[must_use]
    pub const fn new(warden: Weak<WardenEntity>) -> Self {
        Self { warden }
    }
}

impl Goal for WardenHeartbeatAndDarknessGoal {
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
        let Some(warden) = self.warden.upgrade() else {
            return;
        };

        let entity = &warden.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pos = entity.pos.load();

        // 1. Heartbeat sound (delay decreases from 40 down to 10 ticks as anger increases)
        let hb_remaining = warden.heartbeat_timer.fetch_sub(1, Ordering::Relaxed) - 1;
        if hb_remaining <= 0 {
            warden
                .heartbeat_timer
                .store(warden.get_heartbeat_delay(), Ordering::Relaxed);
            world.play_sound_fine(
                Sound::EntityWardenHeartbeat,
                SoundCategory::Hostile,
                &pos,
                5.0,
                1.0,
            );
        }

        // 2. Darkness aura: every 120 ticks, applies Darkness effect to all players within 20 blocks
        let dark_remaining = warden.darkness_timer.fetch_sub(1, Ordering::Relaxed) - 1;
        if dark_remaining <= 0 {
            warden.darkness_timer.store(120, Ordering::Relaxed);
            for player in world.get_nearby_players(pos, 20.0) {
                let gm = player.gamemode.load();
                if gm == GameMode::Creative || gm == GameMode::Spectator {
                    continue;
                }
                player.living_entity.add_effect(Effect {
                    effect_type: &StatusEffect::DARKNESS,
                    duration: 260,
                    amplifier: 0,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    blend: false,
                });
            }
        }

        // 3. Ambient sniff sound when calm
        if warden.get_anger_level() < 40 {
            let sniff_rem = warden.sniff_timer.fetch_sub(1, Ordering::Relaxed) - 1;
            if sniff_rem <= 0 {
                warden
                    .sniff_timer
                    .store(rand::random_range(120..240), Ordering::Relaxed);
                world.play_sound_fine(
                    Sound::EntityWardenSniff,
                    SoundCategory::Hostile,
                    &pos,
                    5.0,
                    1.0,
                );
            }
        }

        // 4. Anger decay: decrements by 1 every 20 ticks (1 second) when no target
        if warden.mob_entity.get_target().is_none() && entity.age.load(Ordering::Relaxed) % 20 == 0 {
            warden.decrease_anger(1);
        }

        // 5. Out of melee reach tracker: when target is far, increments counter
        if let Some(target) = warden.mob_entity.get_target() {
            let target_pos = target.get_entity().pos.load();
            if pos.squared_distance_to_vec(&target_pos) > 16.0 {
                warden
                    .out_of_melee_reach_ticks
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                warden
                    .out_of_melee_reach_ticks
                    .store(0, Ordering::Relaxed);
            }
        }
    }

    fn controls(&self) -> Controls {
        Controls::empty()
    }
}

/// Sonic Boom ranged attack goal
pub struct WardenSonicBoomGoal {
    warden: Weak<WardenEntity>,
}

impl WardenSonicBoomGoal {
    #[must_use]
    pub const fn new(warden: Weak<WardenEntity>) -> Self {
        Self { warden }
    }
}

impl Goal for WardenSonicBoomGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(warden) = self.warden.upgrade() else {
            return false;
        };

        if warden.sonic_charge_ticks.load(Ordering::Relaxed) > 0 {
            return true;
        }

        if warden.sonic_boom_cooldown.load(Ordering::Relaxed) > 0 {
            warden
                .sonic_boom_cooldown
                .fetch_sub(1, Ordering::Relaxed);
            return false;
        }

        let Some(target) = warden.mob_entity.get_target() else {
            return false;
        };

        if !target.get_entity().is_alive() {
            return false;
        }

        let entity = &warden.mob_entity.living_entity.entity;
        let pos = entity.pos.load();
        let target_pos = target.get_entity().pos.load();

        let dx = target_pos.x - pos.x;
        let dy = (target_pos.y - pos.y).abs();
        let dz = target_pos.z - pos.z;
        let horizontal_dist_sq = dx * dx + dz * dz;

        // Condition: horizontal dist between 4.0 and 15.0 blocks, vertical <= 20.0 blocks,
        // OR target has been unreachable by melee for > 200 ticks
        let out_of_reach = warden.out_of_melee_reach_ticks.load(Ordering::Relaxed) > 200;
        let in_range = (16.0..=225.0).contains(&horizontal_dist_sq) && dy <= 20.0;

        in_range || (out_of_reach && horizontal_dist_sq <= 400.0)
    }

    fn start(&mut self, _mob: &dyn Mob) {
        let Some(warden) = self.warden.upgrade() else {
            return;
        };

        // Start 34-tick charging animation and sound
        warden.sonic_charge_ticks.store(34, Ordering::Relaxed);

        let entity = &warden.mob_entity.living_entity.entity;
        let world = entity.world.load();
        let pos = entity.pos.load();
        world.play_sound_fine(
            Sound::EntityWardenSonicCharge,
            SoundCategory::Hostile,
            &pos,
            3.0,
            1.0,
        );
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(warden) = self.warden.upgrade() else {
            return false;
        };
        warden.sonic_charge_ticks.load(Ordering::Relaxed) > 0
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(warden) = self.warden.upgrade() else {
            return;
        };

        let ticks = warden.sonic_charge_ticks.fetch_sub(1, Ordering::Relaxed) - 1;
        if ticks <= 0 {
            // Unleash Sonic Boom!
            let Some(target) = warden.mob_entity.get_target() else {
                warden.sonic_boom_cooldown.store(40, Ordering::Relaxed);
                return;
            };

            let entity = &warden.mob_entity.living_entity.entity;
            let world = entity.world.load();
            let pos = entity.pos.load();
            let target_pos = target.get_entity().pos.load();

            // 1. Play Sonic Boom sound
            world.play_sound_fine(
                Sound::EntityWardenSonicBoom,
                SoundCategory::Hostile,
                &pos,
                3.0,
                1.0,
            );

            // 2. Spawn beam of SonicBoom particles from chest to target
            let chest_pos = Vector3::new(pos.x, pos.y + 1.6, pos.z);
            let target_eye = Vector3::new(target_pos.x, target_pos.y + 1.6, target_pos.z);
            let delta = target_eye - chest_pos;
            let len = delta.length();
            if len > 0.001 {
                let norm = delta / len;
                let steps = len.floor() as i32 + 7;
                for i in 1..steps {
                    let particle_pos = chest_pos + norm * (i as f64);
                    world.spawn_particle(
                        particle_pos,
                        Vector3::new(0.0, 0.0, 0.0),
                        0.0,
                        1,
                        Particle::SonicBoom,
                    );
                }
            }

            // 3. Deal 10.0 armor-bypassing generic damage
            target.damage(&*warden, 10.0, DamageType::GENERIC);

            // 4. Knockback target
            let knockback_h = 2.5;
            let knockback_v = 0.5;
            if len > 0.001 {
                let norm = delta / len;
                let vel = target.get_entity().velocity.load();
                target.get_entity().velocity.store(Vector3::new(
                    vel.x + norm.x * knockback_h,
                    vel.y + knockback_v,
                    vel.z + norm.z * knockback_h,
                ));
            }

            // Reset cooldowns
            warden.sonic_boom_cooldown.store(40, Ordering::Relaxed);
            warden.out_of_melee_reach_ticks.store(0, Ordering::Relaxed);
        }
    }

    fn controls(&self) -> Controls {
        Controls::LOOK | Controls::MOVE
    }
}
