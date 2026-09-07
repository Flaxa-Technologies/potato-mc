use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use crossbeam::atomic::AtomicCell;
use pumpkin_data::damage::DamageType;
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag::{self, Taggable};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;
use uuid::Uuid;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        escape_danger::EscapeDangerGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, swim::SwimGoal,
        water_avoiding_random_flying::WaterAvoidingRandomFlyingGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};

/// Duration in ticks of the poison a parrot gets from eating a cookie, matching
/// vanilla `Parrot.mobInteract`.
const COOKIE_POISON_DURATION: i32 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ParrotVariant {
    #[default]
    RedBlue = 0,
    Blue = 1,
    Green = 2,
    YellowBlue = 3,
    Gray = 4,
}

impl ParrotVariant {
    #[must_use]
    pub const fn from_id(id: i32) -> Self {
        match id {
            1 => Self::Blue,
            2 => Self::Green,
            3 => Self::YellowBlue,
            4 => Self::Gray,
            _ => Self::RedBlue,
        }
    }

    #[must_use]
    pub const fn id(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.strip_prefix("minecraft:").unwrap_or(name) {
            "blue" => Self::Blue,
            "green" => Self::Green,
            "yellow_blue" | "yellow" => Self::YellowBlue,
            "gray" => Self::Gray,
            _ => Self::RedBlue,
        }
    }

    #[must_use]
    pub fn random_variant() -> Self {
        let mut rng = rand::rng();
        Self::from_id(rng.random_range(0..5))
    }
}

/// Helper mapping nearby hostile mob types to the parrot's imitation sound.
/// Matches vanilla `Parrot.MOB_SOUND_MAP`.
fn get_imitation_sound(entity_type: &EntityType) -> Option<Sound> {
    if entity_type == &EntityType::BLAZE {
        Some(Sound::EntityParrotImitateBlaze)
    } else if entity_type == &EntityType::BOGGED {
        Some(Sound::EntityParrotImitateBogged)
    } else if entity_type == &EntityType::BREEZE {
        Some(Sound::EntityParrotImitateBreeze)
    } else if entity_type == &EntityType::CREAKING {
        Some(Sound::EntityParrotImitateCreaking)
    } else if entity_type == &EntityType::CREEPER {
        Some(Sound::EntityParrotImitateCreeper)
    } else if entity_type == &EntityType::DROWNED {
        Some(Sound::EntityParrotImitateDrowned)
    } else if entity_type == &EntityType::ELDER_GUARDIAN {
        Some(Sound::EntityParrotImitateElderGuardian)
    } else if entity_type == &EntityType::ENDER_DRAGON {
        Some(Sound::EntityParrotImitateEnderDragon)
    } else if entity_type == &EntityType::ENDERMITE {
        Some(Sound::EntityParrotImitateEndermite)
    } else if entity_type == &EntityType::EVOKER {
        Some(Sound::EntityParrotImitateEvoker)
    } else if entity_type == &EntityType::GHAST {
        Some(Sound::EntityParrotImitateGhast)
    } else if entity_type == &EntityType::GUARDIAN {
        Some(Sound::EntityParrotImitateGuardian)
    } else if entity_type == &EntityType::HOGLIN {
        Some(Sound::EntityParrotImitateHoglin)
    } else if entity_type == &EntityType::HUSK {
        Some(Sound::EntityParrotImitateHusk)
    } else if entity_type == &EntityType::ILLUSIONER {
        Some(Sound::EntityParrotImitateIllusioner)
    } else if entity_type == &EntityType::MAGMA_CUBE {
        Some(Sound::EntityParrotImitateMagmaCube)
    } else if entity_type == &EntityType::PHANTOM {
        Some(Sound::EntityParrotImitatePhantom)
    } else if entity_type == &EntityType::PIGLIN {
        Some(Sound::EntityParrotImitatePiglin)
    } else if entity_type == &EntityType::PIGLIN_BRUTE {
        Some(Sound::EntityParrotImitatePiglinBrute)
    } else if entity_type == &EntityType::PILLAGER {
        Some(Sound::EntityParrotImitatePillager)
    } else if entity_type == &EntityType::RAVAGER {
        Some(Sound::EntityParrotImitateRavager)
    } else if entity_type == &EntityType::SHULKER {
        Some(Sound::EntityParrotImitateShulker)
    } else if entity_type == &EntityType::SILVERFISH {
        Some(Sound::EntityParrotImitateSilverfish)
    } else if entity_type == &EntityType::SKELETON {
        Some(Sound::EntityParrotImitateSkeleton)
    } else if entity_type == &EntityType::SLIME {
        Some(Sound::EntityParrotImitateSlime)
    } else if entity_type == &EntityType::SPIDER {
        Some(Sound::EntityParrotImitateSpider)
    } else if entity_type == &EntityType::STRAY {
        Some(Sound::EntityParrotImitateStray)
    } else if entity_type == &EntityType::VEX {
        Some(Sound::EntityParrotImitateVex)
    } else if entity_type == &EntityType::VINDICATOR {
        Some(Sound::EntityParrotImitateVindicator)
    } else if entity_type == &EntityType::WARDEN {
        Some(Sound::EntityParrotImitateWarden)
    } else if entity_type == &EntityType::WITCH {
        Some(Sound::EntityParrotImitateWitch)
    } else if entity_type == &EntityType::WITHER {
        Some(Sound::EntityParrotImitateWither)
    } else if entity_type == &EntityType::WITHER_SKELETON {
        Some(Sound::EntityParrotImitateWitherSkeleton)
    } else if entity_type == &EntityType::ZOGLIN {
        Some(Sound::EntityParrotImitateZoglin)
    } else if entity_type == &EntityType::ZOMBIE {
        Some(Sound::EntityParrotImitateZombie)
    } else if entity_type == &EntityType::ZOMBIE_VILLAGER {
        Some(Sound::EntityParrotImitateZombieVillager)
    } else {
        None
    }
}

/// Represents a Parrot, a passive flying mob that can mimic nearby mob sounds.
///
/// Vanilla source: `net.minecraft.world.entity.animal.parrot.Parrot`
/// Base class:    `net.minecraft.world.entity.animal.ShoulderRidingEntity`
///
/// Behaviors implemented:
/// - Flying wander (WaterAvoidingRandomFlyingGoal, speed 1.0)
/// - Panic on danger (EscapeDangerGoal, speed 1.25)
/// - Float / Swim (SwimGoal)
/// - Zero gravity & 0.6 drag (flying mob flight mechanics)
/// - Fall damage immunity
/// - 5 color variants: RedBlue, Blue, Green, YellowBlue, Gray (DATA_VARIANT_ID)
/// - Mob sound imitation: detects nearby hostile mobs and mimics their sounds
/// - Ambient sound (Sound::EntityParrotAmbient, interval ~80 ticks)
/// - Hurt and death sounds (Sound::EntityParrotHurt, Sound::EntityParrotDeath)
/// - Cookie poisoning interaction (instant fatal poison + damage)
/// - Taming interaction with seeds (1 in 3 chance, heart/smoke particles, owner linking)
/// - Sitting toggle when tamed and clicked by owner
/// - Owner & Variant NBT persistence
///
/// Wiki: <https://minecraft.wiki/w/Parrot>
pub struct ParrotEntity {
    pub mob_entity: MobEntity,
    pub variant: AtomicI32,
    pub sitting: AtomicBool,
    pub owner: AtomicCell<Option<Uuid>>,
    ambient_sound_time: AtomicI32,
}

impl ParrotEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let variant = ParrotVariant::random_variant();
        let parrot = Self {
            mob_entity,
            variant: AtomicI32::new(variant.id()),
            sitting: AtomicBool::new(false),
            owner: AtomicCell::new(None),
            ambient_sound_time: AtomicI32::new(-80),
        };
        let mob_arc = Arc::new(parrot);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Priority 0: Float / swim
            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            // Priority 1: Panic when hurt
            goal_selector.add_goal(1, EscapeDangerGoal::new(1.25));
            // Priority 2: Look at player
            goal_selector.add_goal(
                2,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            // Priority 3: Flying random stroll (vanilla: WaterAvoidingRandomFlyingGoal)
            goal_selector.add_goal(3, Box::new(WaterAvoidingRandomFlyingGoal::new(1.0)));
            // Priority 4: Random look around
            goal_selector.add_goal(4, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }

    #[must_use]
    pub fn get_variant(&self) -> ParrotVariant {
        ParrotVariant::from_id(self.variant.load(Ordering::Relaxed))
    }

    pub fn set_variant(&self, variant: ParrotVariant) {
        self.variant.store(variant.id(), Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::parrot::DATA_VARIANT_ID,
            VarInt(variant.id()),
        );
    }

    #[must_use]
    pub fn is_sitting(&self) -> bool {
        self.sitting.load(Ordering::Relaxed)
    }

    pub fn set_sitting(&self, sitting: bool) {
        self.sitting.store(sitting, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_tamed(&self) -> bool {
        self.owner.load().is_some()
    }

    /// Feeds the parrot a cookie: it is poisoned and then killed, as in vanilla
    /// `Parrot.mobInteract`.
    fn eat_cookie(&self, player: &Arc<Player>, item_stack: &mut ItemStack) {
        item_stack.decrement_unless_creative(player.gamemode.load(), 1);

        self.mob_entity
            .living_entity
            .add_effect(pumpkin_data::potion::Effect {
                effect_type: &StatusEffect::POISON,
                duration: COOKIE_POISON_DURATION,
                amplifier: 0,
                ambient: false,
                show_particles: true,
                show_icon: true,
                blend: true,
            });

        self.damage_with_context(
            self,
            f32::MAX,
            DamageType::PLAYER_ATTACK,
            None,
            Some(player.as_ref()),
            Some(player.as_ref()),
        );
    }
}

impl Mob for ParrotEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn is_sitting(&self) -> bool {
        self.is_sitting()
    }

    fn get_mob_gravity(&self) -> f64 {
        0.0
    }

    fn get_mob_y_velocity_drag(&self) -> Option<f64> {
        Some(0.6)
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_int("Variant", self.get_variant().id());
        nbt.put_bool("Sitting", self.is_sitting());
        if let Some(owner) = self.owner.load() {
            nbt.put_uuid("Owner", owner);
        }
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(variant) = nbt.get_int("Variant") {
            self.set_variant(ParrotVariant::from_id(variant));
        }
        if let Some(sitting) = nbt.get_bool("Sitting") {
            self.set_sitting(sitting);
        }
        if let Some(owner) = nbt.get_uuid("Owner") {
            self.owner.store(Some(owner));
        }
    }

    fn mob_set_variant_name(&self, name: &str) {
        self.set_variant(ParrotVariant::from_name(name));
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::parrot::DATA_VARIANT_ID,
            VarInt(self.get_variant().id()),
        );
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        // Ambient sound / Mob imitation (vanilla Parrot.getAmbientSound & MOB_SOUND_MAP)
        let sound_timer = self.ambient_sound_time.fetch_add(1, Ordering::Relaxed);
        if sound_timer > 0 && rand::random_range(0..1000) < sound_timer {
            self.ambient_sound_time.store(-80, Ordering::Relaxed);
            let entity = self.get_entity();
            let world = entity.world.load();
            let pos = entity.pos.load();

            // Check if there is a nearby monster within 20 blocks to imitate
            let imitated = world
                .get_closest_entity(pos, 20.0, None)
                .and_then(|near_ent| get_imitation_sound(&near_ent.get_entity().entity_type));

            let (sound, pitch) = if let Some(imitate) = imitated {
                (imitate, (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0)
            } else {
                (Sound::EntityParrotAmbient, (rand::random::<f32>() - rand::random::<f32>()) * 0.2 + 1.0)
            };

            world.play_sound_fine(
                sound,
                SoundCategory::Neutral,
                &pos,
                1.0,
                pitch,
            );
        }
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        self.ambient_sound_time.store(-80, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntityParrotHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        let item = item_stack.get_item();

        // 1. Poisonous food check (cookies): fatal poisoning
        if item.has_tag(&tag::Item::MINECRAFT_PARROT_POISONOUS_FOOD) {
            self.eat_cookie(player, item_stack);
            return true;
        }

        // 2. Taming with seeds (wheat, melon, pumpkin, beetroot, torchflower, pitcher pod)
        let is_seed = item == &Item::WHEAT_SEEDS
            || item == &Item::MELON_SEEDS
            || item == &Item::PUMPKIN_SEEDS
            || item == &Item::BEETROOT_SEEDS
            || item == &Item::TORCHFLOWER_SEEDS
            || item == &Item::PITCHER_POD;

        if is_seed && !self.is_tamed() {
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let entity = self.get_entity();
            let world = entity.world.load();
            let pos = entity.pos.load();

            world.play_sound(Sound::EntityParrotEat, SoundCategory::Neutral, &pos);

            // 1 in 3 taming chance in vanilla
            if rand::random_range(0..3) == 0 {
                self.owner.store(Some(player.gameprofile.id));
                self.set_sitting(true);
                for _ in 0..7 {
                    world.spawn_particle(
                        pos + Vector3::new(0.0, f64::from(entity.height()) * 0.5, 0.0),
                        Vector3::new(0.5, 0.5, 0.5),
                        1.0,
                        1,
                        Particle::Heart,
                    );
                }
            } else {
                for _ in 0..5 {
                    world.spawn_particle(
                        pos + Vector3::new(0.0, f64::from(entity.height()) * 0.5, 0.0),
                        Vector3::new(0.3, 0.3, 0.3),
                        0.05,
                        1,
                        Particle::Smoke,
                    );
                }
            }
            return true;
        }

        // 3. Sitting toggle when interacted with by owner
        if self.is_tamed() && self.owner.load() == Some(player.gameprofile.id) {
            let next_sitting = !self.is_sitting();
            self.set_sitting(next_sitting);
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::COOKIE_POISON_DURATION;
    use pumpkin_data::item::Item;
    use pumpkin_data::tag::{self, Taggable};

    #[test]
    fn cookie_is_poisonous_parrot_food() {
        assert!(Item::COOKIE.has_tag(&tag::Item::MINECRAFT_PARROT_POISONOUS_FOOD));
    }

    #[test]
    fn parrot_food_is_not_poisonous() {
        assert!(!Item::WHEAT_SEEDS.has_tag(&tag::Item::MINECRAFT_PARROT_POISONOUS_FOOD));
        assert!(!Item::COOKED_CHICKEN.has_tag(&tag::Item::MINECRAFT_PARROT_POISONOUS_FOOD));
    }

    #[test]
    fn poison_lasts_45_seconds() {
        assert_eq!(COOKIE_POISON_DURATION, 900);
    }
}
