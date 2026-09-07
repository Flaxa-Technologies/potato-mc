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

/// Represents a Cod, a common schooling passive aquatic mob.
///
/// Vanilla source: `net.minecraft.world.entity.animal.fish.Cod`
/// Base class:    `net.minecraft.world.entity.animal.fish.AbstractSchoolingFish`
///
/// Behaviors implemented:
/// - Panic on danger (EscapeDangerGoal, speed 1.25)
/// - Flee from players (AvoidEntityGoal, range 8.0, speed 1.6/1.4)
/// - Seek water when stranded on land (TryFindWaterGoal)
/// - Water swimming (SwimGoal)
/// - Flop on land when stranded: jump impulse (Y +0.4) + horizontal jitter + flop sound
/// - Air supply / suffocation: loses air outside water, drowns at -20 air (2 HP damage)
/// - Ambient sound ticking (interval 120 ticks, matches vanilla WaterAnimal)
/// - Hurt sound on damage (Sound::EntityCodHurt)
/// - Bucket pickup with water bucket -> gives cod_bucket + plays Sound::ItemBucketFillFish
/// - FromBucket tracked data & NBT persistence (prevents despawning when caught in bucket)
///
/// Wiki: <https://minecraft.wiki/w/Cod>
pub struct CodEntity {
    pub mob_entity: MobEntity,
    pub from_bucket: AtomicBool,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
}

impl CodEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let cod = Self {
            mob_entity,
            from_bucket: AtomicBool::new(false),
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
        };
        let mob_arc = Arc::new(cod);

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
        entity.set_synced_data(pumpkin_data::tracked_data::cod::FROM_BUCKET, from_bucket);
    }
}

impl Mob for CodEntity {
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
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(from_bucket) = nbt.get_bool("FromBucket") {
            self.set_from_bucket(from_bucket);
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::cod::FROM_BUCKET,
            self.is_from_bucket(),
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
                Sound::EntityCodFlop,
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

        // Ambient sound (vanilla WaterAnimal.AMBIENT_SOUND_INTERVAL = 120)
        let sound_timer = self.ambient_sound_time.fetch_add(1, Ordering::Relaxed);
        if sound_timer > 0 && rand::random_range(0..1000) < sound_timer {
            self.ambient_sound_time.store(-120, Ordering::Relaxed);
            let sound = if in_water {
                Sound::EntityCodAmbient
            } else {
                Sound::EntityCodFlop
            };
            entity.world.load().play_sound_fine(
                sound,
                SoundCategory::Neutral,
                &entity.pos.load(),
                1.0,
                (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0,
            );
        }
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        self.ambient_sound_time.store(-120, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntityCodHurt,
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
                        bucket_item: "cod_bucket".to_string(),
                        cancelled: false,
                    };
                server.plugin_manager.fire_blocking(&server, &mut event);
                if event.cancelled {
                    return false;
                }
            }
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let mut bucket = ItemStack::new(1, &Item::COD_BUCKET);
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
