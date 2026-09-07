use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicU8, Ordering},
};

use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::Sound;
use pumpkin_data::{entity::EntityType, item::Item};
use pumpkin_protocol::codec::var_int::VarInt;
use rand::RngExt;

use crate::entity::{
    Entity,
    ageable::AgeableMob,
    ai::goal::{
        breed::BreedGoal, escape_danger::EscapeDangerGoal, follow_parent::FollowParentGoal,
        look_around::RandomLookAroundGoal, look_at_entity::LookAtEntityGoal, swim::SwimGoal,
        tempt::TemptGoal, wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    passive::animal::Animal,
    player::Player,
};
use pumpkin_nbt::compound::NbtCompound;

const PIG_FOOD: &[&Item] = &[
    &Item::CARROT,
    &Item::POTATO,
    &Item::BEETROOT,
    &Item::CARROT_ON_A_STICK,
];

use crate::entity::EntityBase;
use crate::entity::item_steerable::{ItemBasedSteering, ItemSteerable};

/// Represents a Pig, a common passive mob that provides porkchops.
///
/// Wiki: <https://minecraft.wiki/w/Pig>
pub struct PigEntity {
    pub mob_entity: MobEntity,
    pub ageable_data: crate::entity::ageable::AgeableData,
    pub steering: ItemBasedSteering,
    pub saddled: AtomicBool,
    pub variant: AtomicU8,
}

impl PigEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let pig = Self {
            mob_entity,
            ageable_data: crate::entity::ageable::AgeableData::default(),
            steering: ItemBasedSteering::default(),
            saddled: AtomicBool::new(false),
            variant: AtomicU8::new(rand::rng().random_range(0..3u8)),
        };
        let mob_arc = Arc::new(pig);
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
            goal_selector.add_goal(1, EscapeDangerGoal::new(1.25));
            goal_selector.add_goal(2, BreedGoal::new(1.0));
            goal_selector.add_goal(3, Box::new(TemptGoal::new(1.2, PIG_FOOD)));
            goal_selector.add_goal(4, Box::new(FollowParentGoal::new(1.1)));
            goal_selector.add_goal(5, Box::new(WanderAroundGoal::new(1.0)));
            goal_selector.add_goal(
                6,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(7, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }
}

impl AgeableMob for PigEntity {
    fn get_ageable_data(&self) -> &crate::entity::ageable::AgeableData {
        &self.ageable_data
    }
}

impl Animal for PigEntity {
    fn is_food(&self, item_stack: &ItemStack) -> bool {
        use pumpkin_data::tag::Taggable;
        item_stack
            .item
            .has_tag(&pumpkin_data::tag::Item::MINECRAFT_PIG_FOOD)
            || PIG_FOOD.iter().any(|i| i.id == item_stack.item.id)
    }
}

impl Mob for PigEntity {
    fn as_ageable(&self) -> Option<&dyn AgeableMob> {
        Some(self)
    }

    fn as_animal(&self) -> Option<&dyn Animal> {
        Some(self)
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("Saddle", self.is_saddled());
        let variant_str = match self.variant.load(Ordering::Relaxed) {
            0 => "minecraft:cold",
            2 => "minecraft:warm",
            _ => "minecraft:temperate",
        };
        nbt.put_string("variant", variant_str.to_string());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(saddle) = nbt.get_byte("Saddle") {
            self.set_saddled(saddle == 1);
        }
        if let Some(variant_str) = nbt.get_string("variant") {
            let variant = match variant_str.strip_prefix("minecraft:").unwrap_or(variant_str) {
                "cold" => 0,
                "warm" => 2,
                _ => 1,
            };
            self.variant.store(variant, Ordering::Relaxed);
        }
    }

    fn mob_set_variant_name(&self, name: &str) {
        let variant = match name.strip_prefix("minecraft:").unwrap_or(name) {
            "cold" => 0,
            "warm" => 2,
            _ => 1,
        };
        self.variant.store(variant, Ordering::Relaxed);
        self.get_entity().set_synced_data(
            pumpkin_data::tracked_data::pig::VARIANT,
            VarInt(variant as i32),
        );
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        let is_baby = entity.age.load(Ordering::Relaxed) < 0;
        if is_baby {
            entity.set_synced_data(pumpkin_data::tracked_data::pig::DATA_BABY_ID, true);
        }
        entity.set_synced_data(
            pumpkin_data::tracked_data::pig::VARIANT,
            VarInt(self.variant.load(Ordering::Relaxed) as i32),
        );
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn get_item_steerable(&self) -> Option<&dyn ItemSteerable> {
        Some(self)
    }

    fn is_saddled(&self) -> bool {
        self.saddled.load(Ordering::Relaxed)
    }

    fn can_be_saddled(&self) -> bool {
        use crate::entity::ageable::AgeableMob;
        self.mob_entity.living_entity.entity.is_alive() && !self.is_baby()
    }

    fn set_saddled(&self, saddled: bool) {
        self.saddled.store(saddled, Ordering::Relaxed);
    }

    fn mob_on_lightning_strike(
        &self,
        caller: &dyn EntityBase,
        lightning: &crate::entity::lightning::LightningBoltEntity,
    ) {
        let entity = self.get_entity();
        let world = entity.world.load();
        let pos = entity.pos.load();
        let zombified = crate::entity::r#type::from_type(
            &EntityType::ZOMBIFIED_PIGLIN,
            pos,
            &world,
            uuid::Uuid::new_v4(),
        );
        world.spawn_entity(zombified);
        entity.remove();
        self.mob_entity
            .living_entity
            .on_lightning_strike(caller, lightning);
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        use super::animal::Animal;
        if item_stack.get_item() == &pumpkin_data::item::Item::SADDLE
            && self.can_be_saddled()
            && !self.is_saddled()
        {
            self.set_saddled(true);
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let entity = self.get_entity();
            let world = entity.world.load();
            let pos = entity.pos.load();
            world.play_sound(
                Sound::EntityPigSaddle,
                pumpkin_data::sound::SoundCategory::Neutral,
                &pos,
            );
            return true;
        }

        if self.is_saddled() && !self.is_food(item_stack) {
            let world = player.world();
            if let Some(vehicle) = world.get_entity_by_id(self.get_entity().entity_id)
                && let Some(passenger) = world.get_player_by_id(player.entity_id())
            {
                self.get_entity()
                    .add_passenger(vehicle, passenger as Arc<dyn EntityBase>);
                return true;
            }
        }
        self.animal_interact(player, item_stack, Sound::EntityPigAmbient)
    }
}

impl ItemSteerable for PigEntity {
    fn boost(&self) -> bool {
        self.steering.boost()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
