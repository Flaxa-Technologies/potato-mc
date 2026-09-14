use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

use pumpkin_data::attributes::Attributes;
use pumpkin_data::damage::DamageType;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag::{self, Taggable};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
    mob::{Mob, MobEntity, slime::SlimeEntity},
    player::Player,
};
use crate::world::ExplosionInteraction;

#[derive(Clone, Copy, Debug)]
pub struct SulfurCubeArchetype {
    pub name: &'static str,
    pub tag: &'static pumpkin_data::tag::Tag,
    pub knockback_res: f64,
    pub bounce: f32,
    pub friction: f32,
    pub drag: f32,
    pub buoyant: bool,
    pub fuse: Option<i32>,
    pub contact_damage: Option<f32>,
    pub knockback_horizontal: f32,
    pub knockback_vertical: f32,
    pub hit_sound: Sound,
    pub push_sound: Sound,
    pub push_threshold: f32,
    pub push_cooldown: f32,
}

pub static ARCHETYPE_REGULAR: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "regular",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_REGULAR,
    knockback_res: -1.0,
    bounce: 0.5,
    friction: 0.3,
    drag: 0.1,
    buoyant: true,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeRegularHit,
    push_sound: Sound::EntitySulfurCubeRegularPush,
    push_threshold: 0.2,
    push_cooldown: 0.5,
};

pub static ARCHETYPE_BOUNCY: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "bouncy",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_BOUNCY,
    knockback_res: -2.0,
    bounce: 0.9,
    friction: 0.3,
    drag: 0.01,
    buoyant: true,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.105,
    hit_sound: Sound::EntitySulfurCubeBouncyHit,
    push_sound: Sound::EntitySulfurCubeBouncyPush,
    push_threshold: 0.3,
    push_cooldown: 0.7,
};

pub static ARCHETYPE_SLOW_BOUNCY: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "slow_bouncy",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_SLOW_BOUNCY,
    knockback_res: 0.4,
    bounce: 0.6,
    friction: 0.3,
    drag: 0.05,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.24,
    hit_sound: Sound::EntitySulfurCubeSlowBouncyHit,
    push_sound: Sound::EntitySulfurCubeSlowBouncyPush,
    push_threshold: 0.05,
    push_cooldown: 0.5,
};

pub static ARCHETYPE_SLOW_FLAT: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "slow_flat",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_SLOW_FLAT,
    knockback_res: 0.5,
    bounce: 0.4,
    friction: 0.4,
    drag: 0.1,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.105,
    hit_sound: Sound::EntitySulfurCubeSlowFlatHit,
    push_sound: Sound::EntitySulfurCubeSlowFlatPush,
    push_threshold: 0.03,
    push_cooldown: 0.9,
};

pub static ARCHETYPE_FAST_FLAT: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "fast_flat",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_FAST_FLAT,
    knockback_res: -1.0,
    bounce: 0.5,
    friction: 0.2,
    drag: 0.01,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.9125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeFastFlatHit,
    push_sound: Sound::EntitySulfurCubeFastFlatPush,
    push_threshold: 0.03,
    push_cooldown: 0.9,
};

pub static ARCHETYPE_LIGHT: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "light",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_LIGHT,
    knockback_res: -1.0,
    bounce: 1.0,
    friction: 0.3,
    drag: 1.8,
    buoyant: true,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.18,
    hit_sound: Sound::EntitySulfurCubeLightHit,
    push_sound: Sound::EntitySulfurCubeLightPush,
    push_threshold: 0.2,
    push_cooldown: 0.7,
};

pub static ARCHETYPE_FAST_SLIDING: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "fast_sliding",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_FAST_SLIDING,
    knockback_res: 0.5,
    bounce: 0.1,
    friction: 0.05,
    drag: 0.01,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.6625,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeFastSlidingHit,
    push_sound: Sound::EntitySulfurCubeFastSlidingPush,
    push_threshold: 0.05,
    push_cooldown: 1.0,
};

pub static ARCHETYPE_SLOW_SLIDING: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "slow_sliding",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_SLOW_SLIDING,
    knockback_res: 0.8,
    bounce: 0.1,
    friction: 0.05,
    drag: 0.01,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeSlowSlidingHit,
    push_sound: Sound::EntitySulfurCubeSlowSlidingPush,
    push_threshold: 0.02,
    push_cooldown: 1.0,
};

pub static ARCHETYPE_STICKY: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "sticky",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_STICKY,
    knockback_res: -2.0,
    bounce: 0.0,
    friction: 2.0,
    drag: 0.01,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeStickyHit,
    push_sound: Sound::EntitySulfurCubeStickyPush,
    push_threshold: 0.05,
    push_cooldown: 0.5,
};

pub static ARCHETYPE_HIGH_RESISTANCE: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "high_resistance",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_HIGH_RESISTANCE,
    knockback_res: 0.7,
    bounce: 0.2,
    friction: 1.0,
    drag: 0.01,
    buoyant: false,
    fuse: None,
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeHighResistanceHit,
    push_sound: Sound::EntitySulfurCubeHighResistancePush,
    push_threshold: 0.03,
    push_cooldown: 0.7,
};

pub static ARCHETYPE_EXPLOSIVE: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "explosive",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_EXPLOSIVE,
    knockback_res: -1.0,
    bounce: 0.5,
    friction: 0.3,
    drag: 0.3,
    buoyant: true,
    fuse: Some(120),
    contact_damage: None,
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeExplosiveHit,
    push_sound: Sound::EntitySulfurCubeExplosivePush,
    push_threshold: 0.1,
    push_cooldown: 0.7,
};

pub static ARCHETYPE_HOT: SulfurCubeArchetype = SulfurCubeArchetype {
    name: "hot",
    tag: &tag::Item::MINECRAFT_SULFUR_CUBE_ARCHETYPE_HOT,
    knockback_res: -1.0,
    bounce: 0.5,
    friction: 0.3,
    drag: 0.1,
    buoyant: true,
    fuse: None,
    contact_damage: Some(1.0),
    knockback_horizontal: 0.4125,
    knockback_vertical: 0.09,
    hit_sound: Sound::EntitySulfurCubeHotHit,
    push_sound: Sound::EntitySulfurCubeHotPush,
    push_threshold: 0.2,
    push_cooldown: 0.7,
};

pub static ARCHETYPES: &[SulfurCubeArchetype] = &[
    ARCHETYPE_HOT,
    ARCHETYPE_EXPLOSIVE,
    ARCHETYPE_STICKY,
    ARCHETYPE_HIGH_RESISTANCE,
    ARCHETYPE_FAST_SLIDING,
    ARCHETYPE_SLOW_SLIDING,
    ARCHETYPE_FAST_FLAT,
    ARCHETYPE_SLOW_FLAT,
    ARCHETYPE_BOUNCY,
    ARCHETYPE_SLOW_BOUNCY,
    ARCHETYPE_LIGHT,
    ARCHETYPE_REGULAR,
];

pub fn get_archetype_for_item(item: &Item) -> Option<&'static SulfurCubeArchetype> {
    for arch in ARCHETYPES {
        if item.has_tag(arch.tag) {
            return Some(arch);
        }
    }
    if item.has_tag(&tag::Item::MINECRAFT_SULFUR_CUBE_SWALLOWABLE) {
        return Some(&ARCHETYPE_REGULAR);
    }
    None
}

pub fn is_food_item(item: &Item) -> bool {
    item.has_tag(&tag::Item::MINECRAFT_SULFUR_CUBE_FOOD) || item.id == Item::SLIME_BALL.id
}

pub fn is_swallowable_item(item: &Item) -> bool {
    item.has_tag(&tag::Item::MINECRAFT_SULFUR_CUBE_SWALLOWABLE)
        || get_archetype_for_item(item).is_some()
}

pub struct SulfurCubeEntity {
    pub slime: Arc<SlimeEntity>,
    pub held_item: Mutex<ItemStack>,
    pub is_primed: AtomicBool,
    pub fuse: AtomicI32,
    pub pickup_timer: AtomicI32,
    pub push_sound_cooldown: AtomicI32,
    pub from_bucket: AtomicBool,
}

impl SulfurCubeEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        entity.fire_immune.store(true, Ordering::Relaxed);
        let slime = SlimeEntity::new(entity);

        // Vanilla Sulfur cubes range from size 1 (baby) to 2 (adult)
        let raw_size = slime.get_size();
        let size = raw_size.clamp(1, 2);
        slime.set_size(size, true);

        if size <= 1 {
            slime
                .get_mob_entity()
                .living_entity
                .entity
                .age
                .store(-24000, Ordering::Relaxed);
        }

        // Sulfur Cubes are passive mobs: clear hostile targeting goals inherited from SlimeEntity
        {
            let mut target_selector = slime
                .get_mob_entity()
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            target_selector.clear();
        }

        {
            let mut attributes = slime
                .get_mob_entity()
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(max_health) = attributes.get_mut(&Attributes::MAX_HEALTH.id) {
                max_health.base_value = (4 * size) as f64;
                max_health.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(damage) = attributes.get_mut(&Attributes::ATTACK_DAMAGE.id) {
                damage.base_value = 0.0; // Passive mob
                damage.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(speed) = attributes.get_mut(&Attributes::MOVEMENT_SPEED.id) {
                speed.base_value = 0.25;
                speed.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(armor) = attributes.get_mut(&Attributes::ARMOR.id) {
                armor.base_value = 0.0;
                armor.dirty.store(true, Ordering::Relaxed);
            }
        }
        let max_hp = (4 * size) as f32;
        slime.get_mob_entity().living_entity.health.store(max_hp);

        Arc::new(Self {
            slime,
            held_item: Mutex::new(ItemStack::EMPTY.clone()),
            is_primed: AtomicBool::new(false),
            fuse: AtomicI32::new(-1),
            pickup_timer: AtomicI32::new(0),
            push_sound_cooldown: AtomicI32::new(0),
            from_bucket: AtomicBool::new(false),
        })
    }

    pub fn grow_up(&self) {
        self.slime.set_size(2, true);
        let mut attributes = self
            .slime
            .get_mob_entity()
            .living_entity
            .attributes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(max_health) = attributes.get_mut(&Attributes::MAX_HEALTH.id) {
            max_health.base_value = 8.0;
            max_health.dirty.store(true, Ordering::Relaxed);
        }
        self.slime.get_mob_entity().living_entity.health.store(8.0);
    }

    pub fn is_baby(&self) -> bool {
        self.slime.get_size() <= 1
    }

    pub fn has_body_item(&self) -> bool {
        !self
            .held_item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    }

    pub fn get_current_archetype(&self) -> Option<&'static SulfurCubeArchetype> {
        let guard = self
            .held_item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_empty() {
            None
        } else {
            get_archetype_for_item(guard.item)
        }
    }

    pub fn absorb_block(&self, stack: &ItemStack) -> bool {
        if self.is_baby() || stack.is_empty() || !is_swallowable_item(stack.item) {
            return false;
        }

        let mut guard = self
            .held_item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !guard.is_empty() && guard.item.id == stack.item.id {
            return false;
        }

        let entity = &self.slime.get_mob_entity().living_entity.entity;
        let world = entity.world.load();

        // If already holding a block, eject the old one first
        if !guard.is_empty() {
            world.drop_stack(&entity.block_pos.load(), guard.clone());
        }

        let swallowed = ItemStack::new(1, stack.item);
        *guard = swallowed.clone();
        drop(guard);

        // Update equipment slot and broadcast to clients
        {
            let mut equipment = self
                .slime
                .get_mob_entity()
                .living_entity
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equipment.put(&EquipmentSlot::BODY, swallowed.clone());
        }
        self.slime
            .get_mob_entity()
            .living_entity
            .send_equipment_changes(&[(EquipmentSlot::BODY, swallowed.clone())]);

        // Clear AI goals while holding a block (cube becomes stationary block)
        self.slime
            .get_mob_entity()
            .clear_ai_goals(&*self.slime);

        // Update archetype-specific settings
        if let Some(arch) = get_archetype_for_item(stack.item) {
            if let Some(fuse_ticks) = arch.fuse {
                self.fuse.store(fuse_ticks, Ordering::Relaxed);
            } else {
                self.fuse.store(-1, Ordering::Relaxed);
            }
            self.is_primed.store(false, Ordering::Relaxed);

            let mut attributes = self
                .slime
                .get_mob_entity()
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if arch.name == "high_resistance" {
                if let Some(armor) = attributes.get_mut(&Attributes::ARMOR.id) {
                    armor.base_value = 12.0;
                    armor.dirty.store(true, Ordering::Relaxed);
                }
            } else if let Some(armor) = attributes.get_mut(&Attributes::ARMOR.id) {
                armor.base_value = 0.0;
                armor.dirty.store(true, Ordering::Relaxed);
            }
        }

        world.play_sound(
            Sound::EntitySulfurCubeAbsorb,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
        true
    }

    pub fn ready_for_shearing(&self) -> bool {
        self.has_body_item() && !self.is_primed.load(Ordering::Relaxed)
    }

    pub fn shear_block(&self) -> bool {
        if !self.ready_for_shearing() {
            return false;
        }
        let mut guard = self
            .held_item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_empty() {
            return false;
        }

        let entity = &self.slime.get_mob_entity().living_entity.entity;
        let world = entity.world.load();

        world.drop_stack(&entity.block_pos.load(), guard.clone());
        *guard = ItemStack::EMPTY.clone();
        drop(guard);

        {
            let mut equipment = self
                .slime
                .get_mob_entity()
                .living_entity
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equipment.put(&EquipmentSlot::BODY, ItemStack::EMPTY.clone());
        }
        self.slime
            .get_mob_entity()
            .living_entity
            .send_equipment_changes(&[(EquipmentSlot::BODY, ItemStack::EMPTY.clone())]);

        self.fuse.store(-1, Ordering::Relaxed);
        self.is_primed.store(false, Ordering::Relaxed);
        self.pickup_timer.store(100, Ordering::Relaxed);

        world.play_sound(
            Sound::EntitySulfurCubeEject,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
        true
    }

    pub fn prime(&self, imminent: bool) {
        if self.has_body_item()
            && !self.is_primed.load(Ordering::Relaxed)
            && let Some(arch) = self.get_current_archetype()
            && arch.fuse.is_some()
        {
            let fuse_ticks = if imminent {
                10 + rand::rng().random_range(0..20)
            } else {
                120
            };
            self.fuse.store(fuse_ticks, Ordering::Relaxed);
            self.is_primed.store(true, Ordering::Relaxed);

            let entity = &self.slime.get_mob_entity().living_entity.entity;
            entity.world.load().play_sound(
                Sound::EntityTntPrimed,
                SoundCategory::Blocks,
                &entity.pos.load(),
            );
        }
    }

    pub fn apply_contact_damage(&self, target: &dyn EntityBase) {
        if let Some(arch) = self.get_current_archetype()
            && let Some(dmg) = arch.contact_damage
        {
            target.damage(
                target,
                dmg,
                DamageType::SULFUR_CUBE_HOT,
            );
        }
    }

    fn check_ground_item_pickup(&self) {
        if self.is_baby()
            || self.has_body_item()
            || self.pickup_timer.load(Ordering::Relaxed) > 0
        {
            return;
        }

        let entity = &self.slime.get_mob_entity().living_entity.entity;
        let world = entity.world.load();
        let box_search = entity.bounding_box.load().expand(2.5, 2.0, 2.5);

        let candidates = world.get_entities_at_box(&box_search);
        for item_base in candidates {
            if let Some(item_entity) = item_base.get_item_entity() {
                if item_entity.get_pickup_delay() > 0 {
                    continue;
                }
                let mut stack_guard = item_entity.get_item_stack().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if is_swallowable_item(stack_guard.item) && !stack_guard.is_empty() {
                    let taken = stack_guard.split(1);
                    drop(stack_guard);
                    self.absorb_block(&taken);
                    break;
                }
            }
        }
    }
}

impl Mob for SulfurCubeEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        self.slime.get_mob_entity()
    }

    fn mob_tick(&self, caller: &dyn EntityBase) {
        self.slime.mob_tick(caller);

        let entity = &self.slime.get_mob_entity().living_entity.entity;

        // Decrement cooldowns
        if self.pickup_timer.load(Ordering::Relaxed) > 0 {
            self.pickup_timer.fetch_sub(1, Ordering::Relaxed);
        }
        if self.push_sound_cooldown.load(Ordering::Relaxed) > 0 {
            self.push_sound_cooldown.fetch_sub(1, Ordering::Relaxed);
        }

        // Explosive fuse countdown
        if self.is_primed.load(Ordering::Relaxed) {
            let current = self.fuse.fetch_sub(1, Ordering::Relaxed);
            if current <= 0 {
                let world = entity.world.load();
                let pos = entity.pos.load();
                world.explode(pos, 3.0, ExplosionInteraction::Mob);
                entity.remove();
                return;
            }
        } else if self.has_body_item()
            && let Some(arch) = self.get_current_archetype()
            && arch.fuse.is_some()
        {
            // Redstone ignition check
            let block_pos = entity.block_pos.load();
            let world = entity.world.load();
            if crate::block::blocks::redstone::block_receives_redstone_power(&world, &block_pos)
                || crate::block::blocks::redstone::block_receives_redstone_power(&world, &block_pos.down())
            {
                self.prime(false);
            }
        }

        // Fluid buoyancy for buoyant archetypes
        if self.has_body_item()
            && let Some(arch) = self.get_current_archetype()
            && arch.buoyant
            && (entity.touching_water.load(Ordering::Relaxed)
                || entity.touching_lava.load(Ordering::Relaxed))
        {
            let mut velo = entity.velocity.load();
            if velo.y < 0.15 {
                velo.y += 0.04;
                entity.velocity.store(velo);
            }
        }

        // Baby cube growth
        let current_age = entity.age.load(Ordering::Relaxed);
        if current_age < 0 {
            let new_age = entity.age.fetch_add(1, Ordering::Relaxed) + 1;
            if new_age >= 0 {
                self.grow_up();
            }
        }

        // Ground item absorption
        self.check_ground_item_pickup();
    }

    fn post_tick(&self) {
        let entity = &self.slime.get_mob_entity().living_entity.entity;

        // Splitting & item drop on death
        if (self.slime.get_mob_entity().living_entity.death_time.load(Ordering::Relaxed) >= 20
            || entity.removed.load(Ordering::Relaxed))
            && self
                .slime
                .has_split
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let world = entity.world.load();
            let pos = entity.pos.load();

            // Eject held block on death
            let mut guard = self
                .held_item
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !guard.is_empty() {
                world.drop_stack(&entity.block_pos.load(), guard.clone());
                *guard = ItemStack::EMPTY.clone();
            }

            // Split into 2 baby cubes if adult and not killed by explosion
            if !self.is_baby() && !self.is_primed.load(Ordering::Relaxed) {
                let xz_offset = entity.entity_dimension.load().width / 2.0;
                for i in 0..2 {
                    let xd = ((i % 2) as f32 - 0.5) * xz_offset;
                    let zd = ((i / 2) as f32 - 0.5) * xz_offset;

                    let new_pos = Vector3::new(
                        pos.x + xd as f64,
                        pos.y + 0.5,
                        pos.z + zd as f64,
                    );
                    let new_entity = Entity::new(
                        world.clone(),
                        new_pos,
                        &EntityType::SULFUR_CUBE,
                    );
                    let baby = Self::new(new_entity);
                    baby.slime.set_size(1, true);
                    baby.slime
                        .get_mob_entity()
                        .living_entity
                        .entity
                        .age
                        .store(-24000, Ordering::Relaxed);
                    baby.slime
                        .get_mob_entity()
                        .living_entity
                        .entity
                        .yaw
                        .store(rand::rng().random_range(0.0..360.0));
                    world.spawn_entity_non_save(baby);
                }
            }
        }
    }

    fn pre_damage(&self, damage_type: DamageType, source: Option<&dyn EntityBase>) -> bool {
        // While primed: completely invulnerable
        if self.is_primed.load(Ordering::Relaxed) {
            return false;
        }

        if self.has_body_item() {
            // Check ignition of explosive archetype
            if let Some(arch) = self.get_current_archetype()
                && arch.fuse.is_some()
            {
                if damage_type.has_tag(&tag::DamageType::MINECRAFT_IS_FIRE) {
                    self.prime(false);
                } else if damage_type.has_tag(&tag::DamageType::MINECRAFT_IS_EXPLOSION) {
                    self.prime(true);
                }
            }

            // Damage immunity while holding block:
            // Vulnerable ONLY to: in_wall/suffocation, cramming, lava, fire/on_fire, out_of_world, generic_kill
            let bypasses = damage_type.id == DamageType::IN_WALL.id
                || damage_type.id == DamageType::CRAMMING.id
                || damage_type.id == DamageType::LAVA.id
                || damage_type.id == DamageType::IN_FIRE.id
                || damage_type.id == DamageType::ON_FIRE.id
                || damage_type.id == DamageType::OUT_OF_WORLD.id
                || damage_type.id == DamageType::GENERIC_KILL.id;

            if !bypasses {
                // Apply knockback scaled by archetype and damage that would have been dealt
                let entity = &self.slime.get_mob_entity().living_entity.entity;
                let cube_pos = entity.pos.load();
                let source_pos = source.map_or(cube_pos, |s| s.get_entity().pos.load());

                let mut xd = cube_pos.x - source_pos.x;
                let mut zd = cube_pos.z - source_pos.z;
                let len = (xd * xd + zd * zd).sqrt();
                if len > 0.001 {
                    xd /= len;
                    zd /= len;
                } else {
                    xd = 1.0;
                    zd = 0.0;
                }

                if let Some(arch) = self.get_current_archetype() {
                    let hp = arch.knockback_horizontal as f64;
                    let vp = arch.knockback_vertical as f64;
                    let mut velo = entity.velocity.load();
                    velo.x += xd * hp * 1.5;
                    velo.y = (velo.y + vp * 2.0).clamp(-2.0, 2.0);
                    velo.z += zd * hp * 1.5;
                    entity.velocity.store(velo);

                    entity.world.load().play_sound(
                        arch.hit_sound,
                        SoundCategory::Neutral,
                        &cube_pos,
                    );
                }

                // Return false to cancel all damage without health deduction!
                return false;
            }
        }

        true
    }

    fn mob_player_collision(&self, player: &Arc<Player>) {
        // Passive mob: DO NOT attack player!
        // Hot archetype deals contact damage
        self.apply_contact_damage(&**player);

        // Player push physics when holding block
        if self.has_body_item() {
            let entity = &self.slime.get_mob_entity().living_entity.entity;
            let cube_pos = entity.pos.load();
            let player_pos = player.get_entity().pos.load();

            let dx = cube_pos.x - player_pos.x;
            let dz = cube_pos.z - player_pos.z;
            let dist = (dx * dx + dz * dz).sqrt();

            if dist < 1.3 && dist > 0.01 {
                let p_velo = player.get_entity().velocity.load();
                let p_speed = (p_velo.x * p_velo.x + p_velo.z * p_velo.z).sqrt().clamp(0.05, 0.5);

                let dir_x = dx / dist;
                let dir_z = dz / dist;

                let mut velo = entity.velocity.load();
                velo.x += dir_x * p_speed * 0.4;
                velo.z += dir_z * p_speed * 0.4;
                entity.velocity.store(velo);

                if let Some(arch) = self.get_current_archetype()
                    && self.push_sound_cooldown.load(Ordering::Relaxed) <= 0
                {
                    self.push_sound_cooldown
                        .store((arch.push_cooldown * 20.0) as i32, Ordering::Relaxed);
                    entity.world.load().play_sound(
                        arch.push_sound,
                        SoundCategory::Neutral,
                        &cube_pos,
                    );
                }
            }
        }
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        let entity = &self.slime.get_mob_entity().living_entity.entity;
        let world = entity.world.load();

        // 1. Baby interaction: feeding slimeball speeds up growth
        if self.is_baby() {
            if is_food_item(item_stack.item) {
                let age = entity.age.load(Ordering::Relaxed);
                let current_age = if age >= 0 { -24000 } else { age };
                let delta = ((-current_age as f32 / 20.0) * 0.1).round() as i32 * 20;
                let delta = delta.max(2400);
                let new_age = (current_age + delta).min(0);
                entity.age.store(new_age, Ordering::Relaxed);

                if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                    item_stack.decrement(1);
                }
                world.play_sound(
                    Sound::EntitySmallSulfurCubeEat,
                    SoundCategory::Neutral,
                    &entity.pos.load(),
                );
                world.spawn_particle(
                    entity.pos.load() + Vector3::new(0.0, 0.4, 0.0),
                    Vector3::new(0.3, 0.3, 0.3),
                    0.02,
                    5,
                    Particle::HappyVillager,
                );
                if new_age >= 0 {
                    self.grow_up();
                }
                return true;
            }
            return false;
        }

        // 2. Primed explosive cube cannot be interacted with
        if self.is_primed.load(Ordering::Relaxed) {
            return false;
        }

        // 3. Shearing: drops held block
        if item_stack.item.id == Item::SHEARS.id && self.has_body_item() {
            if self.shear_block() {
                if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                    item_stack.decrement(1);
                }
                return true;
            }
            return false;
        }

        // 4. Flint and Steel / Fire Charge on explosive cube: prime!
        if self.has_body_item()
            && let Some(arch) = self.get_current_archetype()
            && arch.fuse.is_some()
            && (item_stack.item.id == Item::FLINT_AND_STEEL.id
                || item_stack.item.id == Item::FIRE_CHARGE.id)
        {
            self.prime(false);
            if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                item_stack.decrement(1);
            }
            return true;
        }

        // 5. Feeding swallowable block item
        if is_swallowable_item(item_stack.item) {
            let to_absorb = item_stack.clone();
            if self.absorb_block(&to_absorb) {
                if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                    item_stack.decrement(1);
                }
                return true;
            }
        }

        // 6. Bucket pickup
        if item_stack.item.id == Item::BUCKET.id {
            if player.gamemode.load() != pumpkin_util::GameMode::Creative {
                item_stack.decrement(1);
                let bucket_stack = ItemStack::new(1, &Item::SULFUR_CUBE_BUCKET);
                player.inventory.set_slot(
                    player.inventory.get_selected_slot() as usize,
                    bucket_stack,
                );
            }
            world.play_sound(
                Sound::ItemBucketFillSulfurCube,
                SoundCategory::Neutral,
                &entity.pos.load(),
            );
            entity.remove();
            return true;
        }

        // 7. Leashing: allowed when holding a block
        if self.has_body_item() {
            return self
                .slime
                .get_mob_entity()
                .mob_interact(player, item_stack);
        }

        false
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.slime.mob_write_nbt(nbt);
        nbt.put_int("fuse", self.fuse.load(Ordering::Relaxed));
        nbt.put_bool("primed", self.is_primed.load(Ordering::Relaxed));
        nbt.put_int("pickup_timer", self.pickup_timer.load(Ordering::Relaxed));
        nbt.put_bool("from_bucket", self.from_bucket.load(Ordering::Relaxed));

        let guard = self
            .held_item
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !guard.is_empty() {
            nbt.put_string("held_item", guard.item.registry_key.to_string());
        }
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.slime.mob_read_nbt(nbt);
        if let Some(fuse) = nbt.get_int("fuse") {
            self.fuse.store(fuse, Ordering::Relaxed);
        }
        if let Some(primed) = nbt.get_bool("primed") {
            self.is_primed.store(primed, Ordering::Relaxed);
        }
        if let Some(pt) = nbt.get_int("pickup_timer") {
            self.pickup_timer.store(pt, Ordering::Relaxed);
        }
        if let Some(fb) = nbt.get_bool("from_bucket") {
            self.from_bucket.store(fb, Ordering::Relaxed);
        }
        if let Some(item_name) = nbt.get_string("held_item")
            && let Some(item) = Item::from_registry_key(item_name)
        {
            let swallowed = ItemStack::new(1, item);
            *self
                .held_item
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = swallowed.clone();
            {
                let mut equipment = self
                    .slime
                    .get_mob_entity()
                    .living_entity
                    .entity_equipment
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                equipment.put(&EquipmentSlot::BODY, swallowed);
            }
        }
    }
}
