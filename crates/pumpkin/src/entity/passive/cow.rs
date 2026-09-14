use std::sync::{
    Arc, Weak,
    atomic::{AtomicU8, Ordering},
};

use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::Sound;
use pumpkin_data::{entity::EntityType, item::Item};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_protocol::codec::var_int::VarInt;
use rand::RngExt;

use crate::entity::{
    Entity, EntityBase,
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

const TEMPT_ITEMS: &[&Item] = &[&Item::WHEAT];

/// Represents a Cow, a common passive mob that provides milk, leather, and beef.
///
/// Wiki: <https://minecraft.wiki/w/Cow>
pub struct CowEntity {
    pub mob_entity: MobEntity,
    pub variant: AtomicU8,
    pub ageable_data: crate::entity::ageable::AgeableData,
}

impl CowEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let cow = Self {
            mob_entity,
            variant: AtomicU8::new(rand::rng().random_range(0..3u8)),
            ageable_data: crate::entity::ageable::AgeableData::default(),
        };
        let mob_arc = Arc::new(cow);
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
            goal_selector.add_goal(4, Box::new(FollowParentGoal::new(1.25)));
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

impl AgeableMob for CowEntity {
    fn get_ageable_data(&self) -> &crate::entity::ageable::AgeableData {
        &self.ageable_data
    }
}

impl Animal for CowEntity {
    fn is_food(&self, item_stack: &ItemStack) -> bool {
        use pumpkin_data::tag::Taggable;
        item_stack
            .item
            .has_tag(&pumpkin_data::tag::Item::MINECRAFT_COW_FOOD)
            || TEMPT_ITEMS.iter().any(|i| i.id == item_stack.item.id)
    }
}

impl Mob for CowEntity {
    /// Vanilla Animal.java:128 / AbstractGolem.java:36 -- passive mobs never despawn naturally.
    fn remove_when_far_away(&self, _distance_sq: f64) -> bool { false }


    fn as_ageable(&self) -> Option<&dyn AgeableMob> {
        Some(self)
    }

    fn as_animal(&self) -> Option<&dyn Animal> {
        Some(self)
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        let variant_str = match self.variant.load(Ordering::Relaxed) {
            0 => "minecraft:cold",
            2 => "minecraft:warm",
            _ => "minecraft:temperate",
        };
        nbt.put_string("variant", variant_str.to_string());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
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
            pumpkin_data::tracked_data::cow::VARIANT,
            VarInt(variant as i32),
        );
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        let is_baby = entity.age.load(Ordering::Relaxed) < 0;
        if is_baby {
            entity.set_synced_data(pumpkin_data::tracked_data::cow::BABY_ID, true);
        }
        entity.set_synced_data(
            pumpkin_data::tracked_data::cow::VARIANT,
            VarInt(self.variant.load(Ordering::Relaxed) as i32),
        );
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        if item_stack.get_item() == &Item::BUCKET && !self.is_baby() {
            item_stack.decrement_unless_creative(player.gamemode.load(), 1);
            let entity = &self.mob_entity.living_entity.entity;
            let world = entity.world.load();
            world.play_sound(
                Sound::EntityCowMilk,
                pumpkin_data::sound::SoundCategory::Neutral,
                &entity.pos.load(),
            );
            return true;
        }
        self.animal_interact(player, item_stack, Sound::EntityCowAmbient)
    }
}
