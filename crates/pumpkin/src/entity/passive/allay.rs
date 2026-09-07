use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use crossbeam::atomic::AtomicCell;
use pumpkin_data::damage::DamageType;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
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

// Vanilla: Allay.DUPLICATION_COOLDOWN_TICKS = 6000
const DUPLICATION_COOLDOWN_TICKS: i32 = 6000;
// Vanilla: Allay heals 1 HP every 10 ticks while alive (aiStep)
const HEAL_INTERVAL_TICKS: u32 = 10;

/// Represents an Allay — a flying mob that collects matching items and
/// delivers them to its liked player or a nearby note block.
///
/// Vanilla source: `net.minecraft.world.entity.animal.allay.Allay`
/// Vanilla AI:     `net.minecraft.world.entity.animal.allay.AllayAi`
///
/// # Partially Implemented
/// The full item-tracking brain behaviors (GoToWantedItem, GoAndGiveItemsToTarget,
/// StayCloseToTarget near note block) are not yet ported because they require
/// goal types that don't exist in Pumpkin's goal-selector system.  The existing
/// Pumpkin codebase uses goal-selector, not Brain/Sensor — porting those goals
/// is deferred until dedicated goal implementations exist.
///
/// Behaviours that ARE implemented:
/// - Flying wander (WaterAvoidingRandomFlyingGoal, speed 1.0)
/// - EscapeDanger (panic speed 2.5, matching AllayAi core SPEED_MULTIPLIER_WHEN_PANICKING)
/// - SwimGoal (Swim behaviour present in vanilla Core activity)
/// - Zero gravity (flying mob — matches Allay.travel() travelFlying)
/// - Fall damage immunity (vanilla checkFallDamage is empty)
/// - Heal 1 HP every 10 ticks (vanilla Allay.aiStep)
/// - Duplication cooldown tick-down
/// - Ambient sound with/without held item differentiation
/// - DATA_DANCING and DATA_CAN_DUPLICATE synced data
/// - Owner (liked player) NBT persistence
/// - mob_interact: give item to link owner; interaction logic
///
/// Wiki: <https://minecraft.wiki/w/Allay>
pub struct AllayEntity {
    pub mob_entity: MobEntity,
    pub dancing: AtomicBool,
    pub can_duplicate: AtomicBool,
    pub duplication_cooldown: AtomicI32,
    pub owner: AtomicCell<Option<Uuid>>,
    tick_count: AtomicI32,
    ambient_sound_time: AtomicI32,
}

impl AllayEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let allay = Self {
            mob_entity,
            dancing: AtomicBool::new(false),
            can_duplicate: AtomicBool::new(true),
            duplication_cooldown: AtomicI32::new(0),
            owner: AtomicCell::new(None),
            tick_count: AtomicI32::new(0),
            ambient_sound_time: AtomicI32::new(-80),
        };
        let mob_arc = Arc::new(allay);
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

            // Priority 0: Float / swim (vanilla Core activity: Swim(0.8))
            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            // Priority 1: Panic (vanilla Core: AnimalPanic, speed 2.5)
            goal_selector.add_goal(1, EscapeDangerGoal::new(2.5));
            // TODO: Priority 2: GoToWantedItem (speed 1.75, range 32) — needs GoToWantedItem goal
            // TODO: Priority 3: GoAndGiveItemsToTarget (speed 2.25, timeout 20) — needs GoAndGiveItemsToTarget goal
            // TODO: Priority 4: StayCloseToTarget near note-block/liked-player (4-16 blocks, speed 2.25) — needs StayCloseToTarget goal
            // Priority 5: Look at nearby entity sometimes (vanilla: SetEntityLookTargetSometimes)
            goal_selector.add_goal(
                5,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            // Priority 6: Fly randomly (vanilla Idle: RandomStroll.fly(1.0))
            goal_selector.add_goal(6, Box::new(WaterAvoidingRandomFlyingGoal::new(1.0)));
            // Priority 7: Random look-around
            goal_selector.add_goal(7, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }

    #[must_use]
    pub fn is_dancing(&self) -> bool {
        self.dancing.load(Ordering::Relaxed)
    }

    pub fn set_dancing(&self, dancing: bool) {
        self.dancing.store(dancing, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(pumpkin_data::tracked_data::allay::DATA_DANCING, dancing);
    }

    #[must_use]
    pub fn can_duplicate(&self) -> bool {
        self.can_duplicate.load(Ordering::Relaxed)
    }

    pub fn set_can_duplicate(&self, can_duplicate: bool) {
        self.can_duplicate.store(can_duplicate, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::allay::DATA_CAN_DUPLICATE,
            can_duplicate,
        );
    }

    /// Returns whether this Allay is holding an item in its main hand.
    /// Matches vanilla `Allay.hasItemInHand()`.
    #[must_use]
    fn has_item_in_hand(&self) -> bool {
        let living = &self.mob_entity.living_entity;
        let equipment = living
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !equipment.get(&pumpkin_data::data_component_impl::EquipmentSlot::MAIN_HAND).is_empty()
    }
}

impl Mob for AllayEntity {
    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("CanDuplicate", self.can_duplicate());
        nbt.put_int(
            "DuplicationCooldown",
            self.duplication_cooldown.load(Ordering::Relaxed),
        );
        if let Some(owner) = self.owner.load() {
            nbt.put_uuid("Owner", owner);
        }
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(can) = nbt.get_bool("CanDuplicate") {
            self.set_can_duplicate(can);
        }
        if let Some(cd) = nbt.get_int("DuplicationCooldown") {
            self.duplication_cooldown.store(cd, Ordering::Relaxed);
            if cd > 0 {
                self.set_can_duplicate(false);
            }
        }
        if let Some(owner) = nbt.get_uuid("Owner") {
            self.owner.store(Some(owner));
        }
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        let ticks = self.tick_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Vanilla Allay.aiStep: heal 1 HP every 10 ticks while alive.
        if ticks as u32 % HEAL_INTERVAL_TICKS == 0 {
            let living = &self.mob_entity.living_entity;
            let dead = living.dead.load(Ordering::Relaxed);
            if !dead {
                living.heal(1.0);
            }
        }

        // Tick down duplication cooldown.
        let cd = self.duplication_cooldown.load(Ordering::Relaxed);
        if cd > 0 {
            let next = cd - 1;
            self.duplication_cooldown.store(next, Ordering::Relaxed);
            if next == 0 {
                self.set_can_duplicate(true);
            }
        }

        // Ambient sound (vanilla Mob.baseTick: ambientSoundTime++ and rand(1000) < ambientSoundTime)
        let sound_timer = self.ambient_sound_time.fetch_add(1, Ordering::Relaxed);
        if sound_timer > 0 && rand::random_range(0..1000) < sound_timer {
            self.ambient_sound_time.store(-80, Ordering::Relaxed);
            let sound = if self.has_item_in_hand() {
                Sound::EntityAllayAmbientWithItem
            } else {
                Sound::EntityAllayAmbientWithoutItem
            };
            let entity = self.get_entity();
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
        self.ambient_sound_time.store(-80, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.world.load().play_sound(
            Sound::EntityAllayHurt,
            SoundCategory::Neutral,
            &entity.pos.load(),
        );
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::allay::DATA_DANCING,
            self.is_dancing(),
        );
        entity.set_synced_data(
            pumpkin_data::tracked_data::allay::DATA_CAN_DUPLICATE,
            self.can_duplicate(),
        );
    }

    /// Flying mob: no gravity applied by the movement system.
    /// Matches vanilla `Allay.travel()` which calls `travelFlying`.
    fn get_mob_gravity(&self) -> f64 {
        0.0
    }

    /// Dampen vertical velocity to 0.6× each tick (flying drag).
    fn get_mob_y_velocity_drag(&self) -> Option<f64> {
        Some(0.6)
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        use pumpkin_data::data_component_impl::EquipmentSlot;
        use pumpkin_data::item::Item;
        let item = item_stack.get_item();

        // 1. Vanilla: duplicate when dancing + can_duplicate + holding DUPLICATES_ALLAYS (amethyst shard)
        if self.is_dancing() && self.can_duplicate() && item == &Item::AMETHYST_SHARD {
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            self.set_can_duplicate(false);
            self.duplication_cooldown
                .store(DUPLICATION_COOLDOWN_TICKS, Ordering::Relaxed);

            let entity = self.get_entity();
            let world = entity.world.load();
            let pos = entity.pos.load();

            // Vanilla: spawn 3 heart particles + play amethyst block chime.
            for _ in 0..3 {
                world.spawn_particle(
                    pos + Vector3::new(0.0, f64::from(entity.height()), 0.0),
                    Vector3::new(0.5, 0.5, 0.5),
                    1.0,
                    1,
                    Particle::Heart,
                );
            }
            world.play_sound(
                Sound::BlockAmethystBlockChime,
                SoundCategory::Neutral,
                &pos,
            );

            // Spawn the new Allay at the same position.
            let new_allay = Self::new(Entity::new(world.clone(), pos, &EntityType::ALLAY));
            world.spawn_entity(new_allay);
            return true;
        }

        // 2. Vanilla: give item to Allay if empty-handed and player has an item
        if !self.has_item_in_hand() && !item_stack.is_empty() {
            let given = item_stack.split_unless_creative(player.gamemode.load(), 1);
            let living = &self.mob_entity.living_entity;
            let mut equipment = living
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equipment.put(&EquipmentSlot::MAIN_HAND, given);
            drop(equipment);

            self.owner.store(Some(player.gameprofile.id));
            let entity = self.get_entity();
            entity.world.load().play_sound(
                Sound::EntityAllayItemGiven,
                SoundCategory::Neutral,
                &entity.pos.load(),
            );
            return true;
        }

        // 3. Vanilla: take item back from Allay if player is empty-handed
        if self.has_item_in_hand() && item_stack.is_empty() {
            let living = &self.mob_entity.living_entity;
            let mut equipment = living
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut held = equipment.get(&EquipmentSlot::MAIN_HAND);
            equipment.put(&EquipmentSlot::MAIN_HAND, ItemStack::EMPTY.clone());
            drop(equipment);

            self.owner.store(None);
            let entity = self.get_entity();
            entity.world.load().play_sound(
                Sound::EntityAllayItemTaken,
                SoundCategory::Neutral,
                &entity.pos.load(),
            );
            player.inventory().insert_stack_anywhere(&mut held);
            if !held.is_empty() {
                player.drop_item(held);
            }
            return true;
        }

        false
    }
}
