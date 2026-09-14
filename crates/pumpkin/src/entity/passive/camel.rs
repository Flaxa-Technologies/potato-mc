use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use pumpkin_data::damage::DamageType;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag::{self, Taggable};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_world::inventory::{Inventory, SimpleInventory};

use crate::entity::{
    Entity, EntityBase,
    ageable::{AgeableData, AgeableMob},
    ai::goal::{
        breed::BreedGoal, escape_danger::EscapeDangerGoal, follow_parent::FollowParentGoal,
        look_around::RandomLookAroundGoal, look_at_entity::LookAtEntityGoal, swim::SwimGoal,
        tempt::TemptGoal, wander_around::WanderAroundGoal,
    },
    item::ItemEntity,
    mob::{Mob, MobEntity},
    passive::animal::Animal,
    player::Player,
};

const TEMPT_ITEMS: &[&Item] = &[&Item::CACTUS];

pub const FLAG_TAME: u8 = 2;
pub const FLAG_SADDLE: u8 = 4;
pub const FLAG_BRED: u8 = 8;
pub const FLAG_EATING: u8 = 16;
pub const FLAG_STANDING: u8 = 32;

pub struct CamelEntity {
    pub mob_entity: MobEntity,
    pub ageable_data: AgeableData,
    pub flags: AtomicU8,
    pub dashing: AtomicBool,
    pub saddle_inventory: Arc<SimpleInventory>,
    pub armor_inventory: Arc<SimpleInventory>,
    pub mount_inventory: Arc<SimpleInventory>,
}

impl CamelEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let saddle_inventory = Arc::new(SimpleInventory::new(1));
        let armor_inventory = Arc::new(SimpleInventory::new(1));
        let mount_inventory = Arc::new(SimpleInventory::new(0));

        let camel = Self {
            mob_entity,
            ageable_data: AgeableData::default(),
            flags: AtomicU8::new(0),
            dashing: AtomicBool::new(false),
            saddle_inventory,
            armor_inventory,
            mount_inventory,
        };
        let mob_arc = Arc::new(camel);
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

            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            goal_selector.add_goal(1, EscapeDangerGoal::new(2.0));
            goal_selector.add_goal(2, BreedGoal::new(1.0));
            goal_selector.add_goal(3, Box::new(TemptGoal::new(1.25, TEMPT_ITEMS)));
            goal_selector.add_goal(4, Box::new(FollowParentGoal::new(1.0)));
            goal_selector.add_goal(6, Box::new(WanderAroundGoal::new(0.7)));
            goal_selector.add_goal(
                7,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(8, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }

    #[must_use]
    pub fn has_flag(&self, flag: u8) -> bool {
        (self.flags.load(Ordering::Relaxed) & flag) != 0
    }

    pub fn set_flag(&self, flag: u8, val: bool) {
        let current = self.flags.load(Ordering::Relaxed);
        let new_flags = if val { current | flag } else { current & !flag };
        self.flags.store(new_flags, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(
            pumpkin_data::tracked_data::camel::DATA_ID_FLAGS,
            new_flags as i8,
        );
    }

    #[must_use]
    pub fn is_saddled(&self) -> bool {
        self.has_flag(FLAG_SADDLE) || !self.saddle_inventory.is_empty()
    }

    pub fn set_saddled(&self, val: bool) {
        if val {
            if self.saddle_inventory.is_empty() {
                self.saddle_inventory
                    .set_stack(0, ItemStack::new(1, &Item::SADDLE));
            }
        } else {
            self.saddle_inventory
                .set_stack(0, ItemStack::EMPTY.clone());
        }
        self.set_flag(FLAG_SADDLE, val);
    }

    #[must_use]
    pub fn is_dashing(&self) -> bool {
        self.dashing.load(Ordering::Relaxed)
    }

    pub fn set_dashing(&self, dashing: bool) {
        self.dashing.store(dashing, Ordering::Relaxed);
        let entity = self.get_entity();
        entity.set_synced_data(pumpkin_data::tracked_data::camel::DASH, dashing);
    }

    pub fn open_custom_inventory_screen(&self, player: &Arc<Player>) {
        let entity = self.get_entity();
        player.open_mount_inventory(
            entity.entity_id,
            self.saddle_inventory.clone(),
            self.armor_inventory.clone(),
            self.mount_inventory.clone(),
            0,
        );
    }
}

impl AgeableMob for CamelEntity {
    fn get_ageable_data(&self) -> &AgeableData {
        &self.ageable_data
    }
}

impl Animal for CamelEntity {
    fn is_food(&self, item_stack: &ItemStack) -> bool {
        item_stack.item.has_tag(&tag::Item::MINECRAFT_CAMEL_FOOD)
            || item_stack.item == &Item::CACTUS
    }
}

impl Mob for CamelEntity {
    /// Vanilla Animal.java:128 / AbstractGolem.java:36 -- passive mobs never despawn naturally.
    fn remove_when_far_away(&self, _distance_sq: f64) -> bool { false }

    fn as_ageable(&self) -> Option<&dyn AgeableMob> {
        Some(self)
    }

    fn as_animal(&self) -> Option<&dyn Animal> {
        Some(self)
    }

    fn mob_open_custom_inventory_screen(&self, player: &Arc<Player>) {
        self.open_custom_inventory_screen(player);
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.write_ageable_nbt(nbt);
        nbt.put_bool("Saddle", self.is_saddled());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.read_ageable_nbt(nbt);
        if let Some(saddle) = nbt.get_bool("Saddle") {
            self.set_saddled(saddle);
        }
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        self.ageable_ai_step();

        // Sync saddle flag with saddle inventory
        let has_saddle = !self.saddle_inventory.is_empty();
        if has_saddle != self.has_flag(FLAG_SADDLE) {
            self.set_flag(FLAG_SADDLE, has_saddle);
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        let is_baby = entity.age.load(Ordering::Relaxed) < 0;
        if is_baby {
            entity.set_synced_data(pumpkin_data::tracked_data::camel::DATA_BABY_ID, true);
        }
        entity.set_synced_data(
            pumpkin_data::tracked_data::camel::DATA_ID_FLAGS,
            self.flags.load(Ordering::Relaxed) as i8,
        );
        entity.set_synced_data(pumpkin_data::tracked_data::camel::DASH, self.is_dashing());
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        let item = item_stack.get_item();

        // 1. Shift + right-click opens camel GUI if adult
        if player.get_entity().is_sneaking() && !self.is_baby() {
            self.open_custom_inventory_screen(player);
            return true;
        }

        // 2. Right click with saddle on unsaddled adult camel equips saddle
        if item == &Item::SADDLE && !self.is_saddled() && !self.is_baby() {
            self.set_saddled(true);
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let entity = self.get_entity();
            let world = entity.world.load();
            world.play_sound(
                Sound::EntityCamelSaddle,
                SoundCategory::Neutral,
                &entity.pos.load(),
            );
            return true;
        }

        // 3. Cactus feeds the camel (heals / love mode)
        if self.is_food(item_stack) {
            return self.animal_interact(player, item_stack, Sound::EntityCamelAmbient);
        }

        // 4. Right click mounts adult camel (up to 2 passengers, saddled or unsaddled)
        let ent = &self.mob_entity.living_entity.entity;
        let passenger_count = ent
            .passengers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        if passenger_count < 2 && !self.is_baby() {
            let world = player.world();
            if let Some(vehicle) = world.get_entity_by_id(ent.entity_id)
                && let Some(passenger) = world.get_player_by_id(player.entity_id())
            {
                ent.add_passenger(vehicle, passenger as Arc<dyn EntityBase>);
                return true;
            }
        }

        false
    }

    fn on_damage(&self, _damage_type: DamageType, _source: Option<&dyn EntityBase>) {
        if self.mob_entity.living_entity.dead.load(Ordering::Relaxed) && self.is_saddled() {
            let entity = self.get_entity();
            let world = entity.world.load();
            let pos = entity.pos.load();
            let item_entity = Arc::new(ItemEntity::new(
                Entity::new(world.clone(), pos, &EntityType::ITEM),
                ItemStack::new(1, &Item::SADDLE),
            ));
            world.spawn_entity(item_entity);
            self.set_saddled(false);
        }
    }
}
