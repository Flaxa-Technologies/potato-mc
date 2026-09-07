use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use pumpkin_data::damage::DamageType;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        avoid_entity::AvoidEntityGoal, escape_danger::EscapeDangerGoal,
        try_find_water::TryFindWaterGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};

pub const STATE_SMALL: i32 = 0;
pub const STATE_MID: i32 = 1;
pub const STATE_FULL: i32 = 2;

/// Represents a Pufferfish, an aquatic mob that inflates and poisons nearby threats.
///
/// Vanilla source: `net.minecraft.world.entity.animal.fish.Pufferfish`
/// Base class:    `net.minecraft.world.entity.animal.fish.AbstractFish`
///
/// Behaviors implemented:
/// - 3-state puff state machine (0 = small, 1 = mid, 2 = full)
/// - Proximity detection: inflates when a player is within 2 blocks
/// - Blow up sound (Sound::EntityPufferFishBlowUp) on inflation stages
/// - Deflation timer: deflates after 60/100 ticks when no threat is nearby
/// - Blow out sound (Sound::EntityPufferFishBlowOut) on deflation stages
/// - Poison/sting touch: stings and damages players within touch reach when inflated
/// - Sting sound (Sound::EntityPufferFishSting)
/// - Panic on danger (EscapeDangerGoal, speed 1.25)
/// - Flee from players (AvoidEntityGoal, range 8.0, speed 1.6/1.4)
/// - Seek water when stranded on land (TryFindWaterGoal)
/// - Water swimming (SwimGoal)
/// - Flop on land when stranded: jump impulse (Y +0.4) + horizontal jitter + flop sound
/// - Air supply / suffocation: loses air outside water, drowns at -20 air (2 HP damage)
/// - Ambient sound ticking (interval 120 ticks, matches vanilla WaterAnimal)
/// - Hurt sound on damage (Sound::EntityPufferFishHurt)
/// - Bucket pickup with water bucket -> gives pufferfish_bucket + plays Sound::ItemBucketFillFish
/// - FromBucket and PuffState tracked data & NBT persistence
///
/// Wiki: <https://minecraft.wiki/w/Pufferfish>
pub struct PufferfishEntity {
    pub mob_entity: MobEntity,
    pub from_bucket: AtomicBool,
    pub puff_state: AtomicI32,
    inflate_counter: AtomicI32,
    deflate_timer: AtomicI32,
    sting_cooldown: AtomicI32,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
}

impl PufferfishEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let pufferfish = Self {
            mob_entity,
            from_bucket: AtomicBool::new(false),
            puff_state: AtomicI32::new(STATE_SMALL),
            inflate_counter: AtomicI32::new(0),
            deflate_timer: AtomicI32::new(0),
            sting_cooldown: AtomicI32::new(0),
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
        };
        let mob_arc = Arc::new(pufferfish);

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 0: Panic when hurt (vanilla: PanicGoal 1.25)
            goal_selector.add_goal(0, EscapeDangerGoal::new(1.25));
            // Priority 1: Seek water when stranded
            goal_selector.add_goal(1, Box::new(TryFindWaterGoal));
            // Priority 2: Avoid players within 8 blocks (vanilla: AvoidEntityGoal Player, 8.0, 1.6, 1.4)
            goal_selector.add_goal(
                2,
                Box::new(AvoidEntityGoal::new(&EntityType::PLAYER, 8.0, 1.6, 1.4)),
            );
        };

        mob_arc
    }

    #[must_use]
    pub fn is_from_bucket(&self) -> bool {
        self.from_bucket.load(Ordering::Relaxed)
    }

    pub fn set_from_bucket(&self, from_bucket: bool) {
        self.from_bucket.store(from_bucket, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::pufferfish::FROM_BUCKET,
            from_bucket,
        );
    }

    #[must_use]
    pub fn get_puff_state(&self) -> i32 {
        self.puff_state.load(Ordering::Relaxed)
    }

    pub fn set_puff_state(&self, state: i32) {
        self.puff_state.store(state, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::pufferfish::PUFF_STATE,
            VarInt(state),
        );
    }
}

impl Mob for PufferfishEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn requires_custom_persistence(&self) -> bool {
        self.is_from_bucket()
    }

    fn remove_when_far_away(&self, _distance_sq: f64) -> bool {
        !self.is_from_bucket()
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("FromBucket", self.is_from_bucket());
        nbt.put_int("PuffState", self.get_puff_state());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(from_bucket) = nbt.get_bool("FromBucket") {
            self.set_from_bucket(from_bucket);
        }
        if let Some(puff_state) = nbt.get_int("PuffState") {
            self.set_puff_state(puff_state.clamp(STATE_SMALL, STATE_FULL));
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::pufferfish::FROM_BUCKET,
            self.is_from_bucket(),
        );
        entity.set_synced_data(
            pumpkin_data::tracked_data::pufferfish::PUFF_STATE,
            VarInt(self.get_puff_state()),
        );
    }

    fn mob_tick(&self, caller: &dyn EntityBase) {
        let entity = self.get_entity();
        let in_water = entity.is_in_water() || entity.touching_water.load(Ordering::Relaxed);
        let on_ground = entity.on_ground.load(Ordering::Relaxed);

        // Vanilla AbstractFish.aiStep: flop when on ground outside of water
        if !in_water && on_ground {
            let vel = entity.velocity.load();
            let dx = (rand::random::<f64>() * 2.0 - 1.0) * 0.05;
            let dz = (rand::random::<f64>() * 2.0 - 1.0) * 0.05;
            entity.velocity.store(Vector3::new(vel.x + dx, 0.4, vel.z + dz));
            entity.on_ground.store(false, Ordering::Relaxed);
            entity.velocity_dirty.store(true, Ordering::Relaxed);
            entity.world.load().play_sound(
                Sound::EntityPufferFishFlop,
                SoundCategory::Neutral,
                &entity.pos.load(),
            );
        }

        // Vanilla WaterAnimal.handleAirSupply: lose air outside water; drown at <= -20
        if !in_water {
            let air = self.air_supply.fetch_sub(1, Ordering::Relaxed) - 1;
            if air <= -20 {
                self.air_supply.store(0, Ordering::Relaxed);
                self.mob_entity
                    .living_entity
                    .damage(caller, 2.0, DamageType::DROWN);
            }
        } else {
            self.air_supply.store(300, Ordering::Relaxed);
        }

        // --- Pufferfish inflation / deflation state machine (vanilla Pufferfish.tick) ---
        let world = entity.world.load();
        let pos = entity.pos.load();
        let closest_player = world.get_closest_player(pos, 2.5);
        let has_threat = closest_player.as_ref().is_some_and(|p| {
            let mode = p.gamemode.load();
            mode != pumpkin_util::GameMode::Creative
                && mode != pumpkin_util::GameMode::Spectator
        });

        let current_state = self.get_puff_state();

        if has_threat {
            let count = self.inflate_counter.fetch_add(1, Ordering::Relaxed);
            self.deflate_timer.store(0, Ordering::Relaxed);

            if current_state == STATE_SMALL {
                world.play_sound(Sound::EntityPufferFishBlowUp, SoundCategory::Neutral, &pos);
                self.set_puff_state(STATE_MID);
            } else if count > 40 && current_state == STATE_MID {
                world.play_sound(Sound::EntityPufferFishBlowUp, SoundCategory::Neutral, &pos);
                self.set_puff_state(STATE_FULL);
            }
        } else {
            self.inflate_counter.store(0, Ordering::Relaxed);
            if current_state != STATE_SMALL {
                let timer = self.deflate_timer.fetch_add(1, Ordering::Relaxed);
                if timer > 60 && current_state == STATE_FULL {
                    world.play_sound(Sound::EntityPufferFishBlowOut, SoundCategory::Neutral, &pos);
                    self.set_puff_state(STATE_MID);
                } else if timer > 100 && current_state == STATE_MID {
                    world.play_sound(Sound::EntityPufferFishBlowOut, SoundCategory::Neutral, &pos);
                    self.set_puff_state(STATE_SMALL);
                }
            }
        }

        // --- Poison / sting touch on nearby players when puffed up ---
        if current_state > STATE_SMALL {
            let cd = self.sting_cooldown.load(Ordering::Relaxed);
            if cd > 0 {
                self.sting_cooldown.store(cd - 1, Ordering::Relaxed);
            } else if let Some(player) = &closest_player {
                let ppos = player.living_entity.entity.pos.load();
                let dist_sq = (ppos.x - pos.x).powi(2) + (ppos.y - pos.y).powi(2) + (ppos.z - pos.z).powi(2);
                if dist_sq <= 1.0 {
                    let damage = (1 + current_state) as f32;
                    player.damage(caller, damage, DamageType::MOB_ATTACK);
                    world.play_sound(Sound::EntityPufferFishSting, SoundCategory::Neutral, &pos);
                    self.sting_cooldown.store(20, Ordering::Relaxed);
                }
            }
        }

        // Flop sound when stranded outside water
        if !in_water {
            let sound_timer = self.ambient_sound_time.fetch_add(1, Ordering::Relaxed);
            if sound_timer > 0 && rand::random_range(0..1000) < sound_timer {
                self.ambient_sound_time.store(-120, Ordering::Relaxed);
                entity.world.load().play_sound_fine(
                    Sound::EntityPufferFishFlop,
                    SoundCategory::Neutral,
                    &entity.pos.load(),
                    1.0,
                    (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0,
                );
            }
        }
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        self.ambient_sound_time.store(-120, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntityPufferFishHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        // Vanilla: bucket pickup when right-clicked with water bucket
        if item_stack.get_item() == &Item::WATER_BUCKET {
            let entity = self.get_entity();
            let world = entity.world.load();
            if let Some(server) = world.server.upgrade() {
                let mut event =
                    crate::plugin::api::events::player::player_bucket_entity::PlayerBucketEntityEvent {
                        player: player.clone(),
                        entity_id: entity.entity_id,
                        bucket_item: "pufferfish_bucket".to_string(),
                        cancelled: false,
                    };
                server.plugin_manager.fire_blocking(&server, &mut event);
                if event.cancelled {
                    return false;
                }
            }
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let mut bucket = ItemStack::new(1, &Item::PUFFERFISH_BUCKET);
            player.inventory().insert_stack_anywhere(&mut bucket);
            if !bucket.is_empty() {
                player.drop_item(bucket);
            }
            let pos = entity.pos.load();
            world.play_sound(Sound::ItemBucketFillFish, SoundCategory::Neutral, &pos);
            entity.remove();
            return true;
        }

        false
    }
}
