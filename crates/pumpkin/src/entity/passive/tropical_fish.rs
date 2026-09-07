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
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        avoid_entity::AvoidEntityGoal, escape_danger::EscapeDangerGoal,
        try_find_water::TryFindWaterGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};

/// 22 common tropical fish variants predefined by vanilla Minecraft (e.g. Clownfish, Tang, etc.).
/// Packed representation: pattern | (base_color << 16) | (pattern_color << 24)
const COMMON_VARIANTS: [i32; 22] = [
    // 0: Stripey (257), Orange (1), Gray (7)
    257 | (1 << 16) | (7 << 24),
    // 1: Flopper (1), Gray (7), Gray (7)
    1 | (7 << 16) | (7 << 24),
    // 2: Flopper (1), Gray (7), Blue (11)
    1 | (7 << 16) | (11 << 24),
    // 3: Clayfish (1281), White (0), Gray (7)
    1281 | (0 << 16) | (7 << 24),
    // 4: Sunstreak (256), Blue (11), Gray (7)
    256 | (11 << 16) | (7 << 24),
    // 5: Kob (0), Orange (1), White (0)
    (1 << 16),
    // 6: Spotty (1280), Pink (6), LightBlue (3)
    1280 | (6 << 16) | (3 << 24),
    // 7: Blockfish (769), Purple (10), Yellow (4)
    769 | (10 << 16) | (4 << 24),
    // 8: Clayfish (1281), White (0), Red (14)
    1281 | (0 << 16) | (14 << 24),
    // 9: Spotty (1280), White (0), Yellow (4)
    1280 | (0 << 16) | (4 << 24),
    // 10: Glitter (513), White (0), Gray (7)
    513 | (0 << 16) | (7 << 24),
    // 11: Clayfish (1281), White (0), Orange (1)
    1281 | (0 << 16) | (1 << 24),
    // 12: Dasher (768), Cyan (9), Pink (6)
    768 | (9 << 16) | (6 << 24),
    // 13: Brinely (1024), Lime (5), LightBlue (3)
    1024 | (5 << 16) | (3 << 24),
    // 14: Betty (1025), Red (14), White (0)
    1025 | (14 << 16),
    // 15: Snooper (512), Gray (7), Red (14)
    512 | (7 << 16) | (14 << 24),
    // 16: Blockfish (769), Red (14), White (0)
    769 | (14 << 16),
    // 17: Flopper (1), White (0), Yellow (4)
    1 | (4 << 24),
    // 18: Kob (0), Red (14), White (0)
    (14 << 16),
    // 19: Sunstreak (256), Gray (7), White (0)
    256 | (7 << 16),
    // 20: Dasher (768), Cyan (9), Yellow (4)
    768 | (9 << 16) | (4 << 24),
    // 21: Flopper (1), Yellow (4), Yellow (4)
    1 | (4 << 16) | (4 << 24),
];

const PATTERNS: [u16; 12] = [
    0,    // Kob (small 0)
    256,  // Sunstreak (small 1)
    512,  // Snooper (small 2)
    768,  // Dasher (small 3)
    1024, // Brinely (small 4)
    1280, // Spotty (small 5)
    1,    // Flopper (large 0)
    257,  // Stripey (large 1)
    513,  // Glitter (large 2)
    769,  // Blockfish (large 3)
    1025, // Betty (large 4)
    1281, // Clayfish (large 5)
];

fn random_tropical_fish_variant() -> i32 {
    let mut rng = rand::rng();
    // Vanilla: 90% spawn as one of the 22 common predefined variants
    if rng.random_range(0..10) < 9 {
        COMMON_VARIANTS[rng.random_range(0..COMMON_VARIANTS.len())]
    } else {
        let pattern = PATTERNS[rng.random_range(0..PATTERNS.len())];
        let base_color = rng.random_range(0..16u8);
        let pattern_color = rng.random_range(0..16u8);
        (pattern as i32) | ((base_color as i32) << 16) | ((pattern_color as i32) << 24)
    }
}

/// Represents a Tropical Fish, a colorful schooling aquatic mob.
///
/// Vanilla source: `net.minecraft.world.entity.animal.fish.TropicalFish`
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
/// - Hurt sound on damage (Sound::EntityTropicalFishHurt)
/// - 22 common variants + arbitrary pattern/color generation (DATA_ID_TYPE_VARIANT)
/// - Bucket pickup with water bucket -> gives tropical_fish_bucket + plays Sound::ItemBucketFillFish
/// - FromBucket tracked data & NBT persistence
///
/// Wiki: <https://minecraft.wiki/w/Tropical_Fish>
pub struct TropicalFishEntity {
    pub mob_entity: MobEntity,
    pub from_bucket: AtomicBool,
    pub variant: AtomicI32,
    air_supply: AtomicI32,
    ambient_sound_time: AtomicI32,
}

impl TropicalFishEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let variant = random_tropical_fish_variant();
        let tropical_fish = Self {
            mob_entity,
            from_bucket: AtomicBool::new(false),
            variant: AtomicI32::new(variant),
            air_supply: AtomicI32::new(300),
            ambient_sound_time: AtomicI32::new(-120),
        };
        let mob_arc = Arc::new(tropical_fish);

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
            pumpkin_data::tracked_data::tropical_fish::FROM_BUCKET,
            from_bucket,
        );
    }

    #[must_use]
    pub fn get_variant(&self) -> i32 {
        self.variant.load(Ordering::Relaxed)
    }

    pub fn set_variant(&self, variant: i32) {
        self.variant.store(variant, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::tropical_fish::DATA_ID_TYPE_VARIANT,
            VarInt(variant),
        );
    }
}

impl Mob for TropicalFishEntity {
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
        nbt.put_int("BucketVariantTag", self.get_variant());
        nbt.put_int("Variant", self.get_variant());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(from_bucket) = nbt.get_bool("FromBucket") {
            self.set_from_bucket(from_bucket);
        }
        if let Some(variant) = nbt.get_int("Variant").or_else(|| nbt.get_int("BucketVariantTag")) {
            self.set_variant(variant);
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::tropical_fish::FROM_BUCKET,
            self.is_from_bucket(),
        );
        entity.set_synced_data(
            pumpkin_data::tracked_data::tropical_fish::DATA_ID_TYPE_VARIANT,
            VarInt(self.get_variant()),
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
                Sound::EntityTropicalFishFlop,
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
                Sound::EntityTropicalFishAmbient
            } else {
                Sound::EntityTropicalFishFlop
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
            Sound::EntityTropicalFishHurt,
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
                        bucket_item: "tropical_fish_bucket".to_string(),
                        cancelled: false,
                    };
                server.plugin_manager.fire_blocking(&server, &mut event);
                if event.cancelled {
                    return false;
                }
            }
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let mut bucket = ItemStack::new(1, &Item::TROPICAL_FISH_BUCKET);
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

    fn mob_set_variant_name(&self, name: &str) {
        let clean = name.strip_prefix("minecraft:").unwrap_or(name);
        let variant = match clean {
            "kob" => Some(COMMON_VARIANTS[5]),
            "sunstreak" => Some(COMMON_VARIANTS[4]),
            "snooper" => Some(COMMON_VARIANTS[15]),
            "dasher" => Some(COMMON_VARIANTS[12]),
            "brinely" => Some(COMMON_VARIANTS[13]),
            "spotty" => Some(COMMON_VARIANTS[6]),
            "flopper" => Some(COMMON_VARIANTS[1]),
            "stripey" => Some(COMMON_VARIANTS[0]),
            "glitter" => Some(COMMON_VARIANTS[10]),
            "blockfish" => Some(COMMON_VARIANTS[7]),
            "betty" => Some(COMMON_VARIANTS[14]),
            "clayfish" => Some(COMMON_VARIANTS[3]),
            s => s.parse::<i32>().ok(),
        };
        if let Some(v) = variant {
            self.set_variant(v);
        }
    }
}
