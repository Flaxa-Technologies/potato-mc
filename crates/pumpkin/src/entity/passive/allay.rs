use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use crossbeam::atomic::AtomicCell;
use pumpkin_data::damage::DamageType;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::particle::Particle;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};
use uuid::Uuid;

use crate::entity::{
    Entity, EntityBase,
    ai::{
        goal::{
            Controls, Goal, escape_danger::EscapeDangerGoal,
            look_at_entity::LookAtEntityGoal, swim::SwimGoal,
            water_avoiding_random_flying::WaterAvoidingRandomFlyingGoal,
        },
        pathfinder::NavigatorGoal,
    },
    item::ItemEntity,
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
pub struct AllayEntity {
    pub mob_entity: MobEntity,
    pub dancing: AtomicBool,
    pub can_duplicate: AtomicBool,
    pub duplication_cooldown: AtomicI32,
    pub owner: AtomicCell<Option<Uuid>>,
    pub liked_noteblock_pos: AtomicCell<Option<BlockPos>>,
    pub liked_noteblock_ticks: AtomicI32,
    pub jukebox_pos: AtomicCell<Option<BlockPos>>,
    pub item_pickup_cooldown: AtomicI32,
    pub inventory: std::sync::Mutex<Vec<ItemStack>>,
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
            liked_noteblock_pos: AtomicCell::new(None),
            liked_noteblock_ticks: AtomicI32::new(0),
            jukebox_pos: AtomicCell::new(None),
            item_pickup_cooldown: AtomicI32::new(0),
            inventory: std::sync::Mutex::new(Vec::new()),
            tick_count: AtomicI32::new(0),
            ambient_sound_time: AtomicI32::new(-80),
        };
        let mob_arc = Arc::new(allay);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut move_control = mob_arc
                .mob_entity
                .move_control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *move_control = Box::new(crate::entity::ai::control::flying_move_control::FlyingMoveControl::new(20, true));
        }
        {
            let mut navigator = mob_arc
                .mob_entity
                .navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *navigator = crate::entity::ai::pathfinder::Navigator::flying();
        }

        {
            let mut attributes = mob_arc
                .mob_entity
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(flying_speed) = attributes.get_mut(&pumpkin_data::attributes::Attributes::FLYING_SPEED.id) {
                flying_speed.base_value = 0.1;
                flying_speed.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(movement_speed) = attributes.get_mut(&pumpkin_data::attributes::Attributes::MOVEMENT_SPEED.id) {
                movement_speed.base_value = 0.1;
                movement_speed.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(max_health) = attributes.get_mut(&pumpkin_data::attributes::Attributes::MAX_HEALTH.id) {
                max_health.base_value = 20.0;
                max_health.dirty.store(true, Ordering::Relaxed);
            }
        }
        mob_arc.mob_entity.living_entity.health.store(20.0);
        mob_arc.get_entity().set_has_no_gravity(true);

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
            // Priority 2: GoToWantedItem / GoAndGiveItemsToTarget (vanilla: speed 1.75/2.25)
            goal_selector.add_goal(
                2,
                Box::new(AllayRetrieveAndDepositGoal::new(Arc::downgrade(&mob_arc))),
            );
            // Priority 5: Look at nearby entity sometimes (vanilla: SetEntityLookTargetSometimes)
            goal_selector.add_goal(
                5,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            // Priority 6: Fly randomly (vanilla Idle: RandomStroll.fly(1.0))
            goal_selector.add_goal(6, Box::new(WaterAvoidingRandomFlyingGoal::new(1.0)));
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

pub struct AllayRetrieveAndDepositGoal {
    allay: Weak<AllayEntity>,
}

impl AllayRetrieveAndDepositGoal {
    #[must_use]
    pub const fn new(allay: Weak<AllayEntity>) -> Self {
        Self { allay }
    }
}

impl Goal for AllayRetrieveAndDepositGoal {
    fn can_start(&mut self, _mob: &dyn Mob) -> bool {
        let Some(allay) = self.allay.upgrade() else {
            return false;
        };
        allay.has_item_in_hand()
    }

    fn should_continue(&self, _mob: &dyn Mob) -> bool {
        let Some(allay) = self.allay.upgrade() else {
            return false;
        };
        allay.has_item_in_hand()
    }

    fn controls(&self) -> Controls {
        Controls::MOVE | Controls::LOOK
    }

    fn should_run_every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, _mob: &dyn Mob) {
        let Some(allay) = self.allay.upgrade() else {
            return;
        };
        let entity = allay.get_entity();
        let pos = entity.pos.load();
        let world = entity.world.load();

        // 1. Decrement liked_noteblock_ticks
        let nb_ticks = allay.liked_noteblock_ticks.load(Ordering::Relaxed);
        if nb_ticks > 0 {
            let next = nb_ticks - 1;
            allay.liked_noteblock_ticks.store(next, Ordering::Relaxed);
            if next == 0 {
                allay.liked_noteblock_pos.store(None);
            }
        }

        // 2. Validate liked noteblock
        if let Some(nb_pos) = allay.liked_noteblock_pos.load() {
            let state = world.get_block_state(&nb_pos);
            if state.is_air() {
                allay.liked_noteblock_pos.store(None);
            }
        }

        // 3. Decrement pickup cooldown
        let cd = allay.item_pickup_cooldown.load(Ordering::Relaxed);
        if cd > 0 {
            allay.item_pickup_cooldown.store(cd - 1, Ordering::Relaxed);
        }

        // 4. Check what item is held in main hand
        let held_item = {
            let living = &allay.mob_entity.living_entity;
            let equip = living
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equip.get(&EquipmentSlot::MAIN_HAND)
        };
        if held_item.is_empty() {
            return;
        }
        let held_item_id = held_item.item.id;

        // 5. Determine deposit target position
        let deposit_target = {
            if let Some(nb_pos) = allay.liked_noteblock_pos.load() {
                let center = nb_pos.to_centered_f64();
                if center.squared_distance_to_vec(&pos) <= 1024.0 {
                    Some(Vector3::new(center.x, center.y + 1.0, center.z))
                } else {
                    None
                }
            } else if let Some(owner_uuid) = allay.owner.load() {
                world.players.load().iter().find_map(|p| {
                    if p.gameprofile.id == owner_uuid
                        && p.get_entity()
                            .pos
                            .load()
                            .squared_distance_to_vec(&pos)
                            <= 4096.0
                    {
                        Some(p.get_entity().pos.load() + Vector3::new(0.0, 0.5, 0.0))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        };

        let has_inventory_items = {
            let inv = allay
                .inventory
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            !inv.is_empty()
        };

        if has_inventory_items {
            // Deliver items to deposit target
            if let Some(target_pos) = deposit_target {
                let dist_sq = target_pos.squared_distance_to_vec(&pos);
                if dist_sq <= 9.0 {
                    // Close enough (3 blocks): throw items!
                    let mut inv = allay
                        .inventory
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    for item_stack in inv.drain(..) {
                        let dir = (target_pos - pos).normalize();
                        let vel = dir * 0.2 + Vector3::new(0.0, 0.1, 0.0);
                        let item_entity = ItemEntity::new_with_velocity(
                            Entity::new(world.clone(), pos, &EntityType::ITEM),
                            item_stack,
                            vel,
                            40,
                        );
                        world.spawn_entity(Arc::new(item_entity));
                    }
                    allay.item_pickup_cooldown.store(60, Ordering::Relaxed);
                    world.play_sound(
                        Sound::EntityAllayItemThrown,
                        SoundCategory::Neutral,
                        &pos,
                    );
                    allay
                        .mob_entity
                        .navigator
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .stop();
                } else {
                    // Fly towards deposit target
                    let mut nav = allay
                        .mob_entity
                        .navigator
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    nav.set_progress(NavigatorGoal::new(pos, target_pos, 2.25));
                    allay
                        .mob_entity
                        .look_control
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .look_at(allay.as_ref(), target_pos.x, target_pos.y, target_pos.z);
                    allay
                        .mob_entity
                        .move_control
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .set_wanted_position(target_pos.x, target_pos.y, target_pos.z, 2.25);
                }
            }
        } else {
            // Inventory is empty: look for matching items on ground within 32 blocks
            if allay.item_pickup_cooldown.load(Ordering::Relaxed) == 0 {
                let nearby = world.get_nearby_entities(pos, 32.0);
                let mut closest_item = None;
                let mut closest_dist_sq = f64::MAX;

                for ent_base in nearby.values() {
                    if let Some(item_entity) = ent_base.get_item_entity() {
                        let is_match = {
                            let lock = item_entity
                                .get_item_stack()
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            lock.item.id == held_item_id
                        };
                        if is_match {
                            let item_pos = item_entity.get_entity().pos.load();
                            let d = item_pos.squared_distance_to_vec(&pos);
                            if d < closest_dist_sq {
                                closest_dist_sq = d;
                                closest_item =
                                    Some((item_entity.get_entity().entity_id, item_pos));
                            }
                        }
                    }
                }

                if let Some((item_id, item_pos)) = closest_item {
                    if closest_dist_sq <= 2.25 {
                        if let Some(ent_base) = world.get_entity_by_id(item_id) {
                            if let Some(item_entity) = ent_base.get_item_entity() {
                                let stack = {
                                    let mut lock = item_entity
                                        .get_item_stack()
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                    let taken = lock.clone();
                                    *lock = ItemStack::EMPTY.clone();
                                    taken
                                };
                                item_entity.get_entity().remove();
                                allay
                                    .inventory
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .push(stack);
                                allay.item_pickup_cooldown.store(20, Ordering::Relaxed);
                                world.play_sound(
                                    Sound::EntityItemPickup,
                                    SoundCategory::Neutral,
                                    &pos,
                                );
                            }
                        }
                    } else {
                        // Fly towards item
                        let mut nav = allay
                            .mob_entity
                            .navigator
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        nav.set_progress(NavigatorGoal::new(pos, item_pos, 1.75));
                        allay
                            .mob_entity
                            .look_control
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .look_at(allay.as_ref(), item_pos.x, item_pos.y, item_pos.z);
                        allay
                            .mob_entity
                            .move_control
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .set_wanted_position(item_pos.x, item_pos.y, item_pos.z, 1.75);
                        return;
                    }
                }
            }

            // If no item on ground to pick up, stay near deposit target (4-16 blocks)
            if let Some(target_pos) = deposit_target {
                let dist_sq = target_pos.squared_distance_to_vec(&pos);
                if dist_sq > 16.0 {
                    let mut nav = allay
                        .mob_entity
                        .navigator
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    nav.set_progress(NavigatorGoal::new(pos, target_pos, 2.25));
                    allay
                        .mob_entity
                        .look_control
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .look_at(allay.as_ref(), target_pos.x, target_pos.y, target_pos.z);
                    allay
                        .mob_entity
                        .move_control
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .set_wanted_position(target_pos.x, target_pos.y, target_pos.z, 2.25);
                }
            }
        }
    }
}

impl Mob for AllayEntity {
    /// Vanilla Animal.java:128 / AbstractGolem.java:36 -- passive mobs never despawn naturally.
    fn remove_when_far_away(&self, _distance_sq: f64) -> bool { false }

    fn mob_hear_noteblock(&self, pos: BlockPos) {
        self.liked_noteblock_pos.store(Some(pos));
        self.liked_noteblock_ticks.store(600, Ordering::Relaxed);
    }

    fn mob_hear_jukebox(&self, pos: BlockPos, is_playing: bool) {
        if is_playing {
            if !self.is_dancing() {
                self.jukebox_pos.store(Some(pos));
                self.set_dancing(true);
            }
        } else if self.is_dancing() {
            if self.jukebox_pos.load() == Some(pos) || self.jukebox_pos.load().is_none() {
                self.jukebox_pos.store(None);
                self.set_dancing(false);
            }
        }
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("CanDuplicate", self.can_duplicate());
        nbt.put_int(
            "DuplicationCooldown",
            self.duplication_cooldown.load(Ordering::Relaxed),
        );
        if let Some(owner) = self.owner.load() {
            nbt.put_uuid("Owner", owner);
        }
        if let Some(nb_pos) = self.liked_noteblock_pos.load() {
            nbt.put_int("LikedNoteblockX", nb_pos.0.x);
            nbt.put_int("LikedNoteblockY", nb_pos.0.y);
            nbt.put_int("LikedNoteblockZ", nb_pos.0.z);
            nbt.put_int(
                "LikedNoteblockTicks",
                self.liked_noteblock_ticks.load(Ordering::Relaxed),
            );
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
        if let (Some(x), Some(y), Some(z)) = (
            nbt.get_int("LikedNoteblockX"),
            nbt.get_int("LikedNoteblockY"),
            nbt.get_int("LikedNoteblockZ"),
        ) {
            self.liked_noteblock_pos.store(Some(BlockPos(Vector3::new(x, y, z))));
            let ticks = nbt.get_int("LikedNoteblockTicks").unwrap_or(600);
            self.liked_noteblock_ticks.store(ticks, Ordering::Relaxed);
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

        // Jukebox dancing validation (vanilla: check every 20 ticks if dancing)
        if ticks % 20 == 0 && self.is_dancing() {
            let mut stop = false;
            if let Some(jpos) = self.jukebox_pos.load() {
                let entity = self.get_entity();
                let pos = entity.pos.load();
                let center = jpos.to_centered_f64();
                if pos.squared_distance_to_vec(&center) > 256.0 {
                    stop = true;
                } else {
                    let world = entity.world.load();
                    let block_state = world.get_block_state(&jpos);
                    if block_state.id.to_block().id != pumpkin_data::Block::JUKEBOX.id {
                        stop = true;
                    }
                }
            } else {
                stop = true;
            }
            if stop {
                self.jukebox_pos.store(None);
                self.set_dancing(false);
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
        if self.is_dancing() {
            self.jukebox_pos.store(None);
            self.set_dancing(false);
        }
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

            // Broadcast entity event 18 to trigger client-side duplication hearts and animations
            world.broadcast_packet_all(&pumpkin_protocol::java::client::play::CEntityStatus::new(
                entity.entity_id,
                18,
            ));

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
            equipment.put(&EquipmentSlot::MAIN_HAND, given.clone());
            drop(equipment);
            living.send_equipment_changes(&[(EquipmentSlot::MAIN_HAND, given)]);

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
            living.send_equipment_changes(&[(EquipmentSlot::MAIN_HAND, ItemStack::EMPTY.clone())]);

            self.owner.store(None);
            self.liked_noteblock_pos.store(None);
            self.liked_noteblock_ticks.store(0, Ordering::Relaxed);
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
            let mut inv = self
                .inventory
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for mut item in inv.drain(..) {
                player.inventory().insert_stack_anywhere(&mut item);
                if !item.is_empty() {
                    player.drop_item(item);
                }
            }
            return true;
        }

        false
    }
}
