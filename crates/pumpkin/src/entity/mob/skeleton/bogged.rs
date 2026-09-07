use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tracked_data;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;

use crate::entity::item::ItemEntity;
use crate::entity::mob::skeleton::SkeletonEntityBase;
use crate::entity::mob::{Mob, MobEntity};
use crate::entity::player::Player;
use crate::entity::Entity;

pub struct BoggedSkeletonEntity {
    pub entity: Arc<SkeletonEntityBase>,
    sheared: AtomicBool,
}

impl BoggedSkeletonEntity {
    #[must_use]
    pub fn new(entity: Entity) -> Arc<Self> {
        let entity = SkeletonEntityBase::new(entity);
        let bogged = Self {
            entity,
            sheared: AtomicBool::new(false),
        };
        Arc::new(bogged)
    }

    #[must_use]
    pub fn is_sheared(&self) -> bool {
        self.sheared.load(Ordering::Relaxed)
    }

    pub fn set_sheared(&self, sheared: bool) {
        self.sheared.store(sheared, Ordering::Relaxed);
        let entity = &self.entity.mob_entity.living_entity.entity;
        entity.set_synced_data(tracked_data::bogged::DATA_SHEARED, sheared);
    }
}

impl Mob for BoggedSkeletonEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.entity.mob_entity
    }

    fn mob_init_data_tracker(&self) {
        let entity = &self.entity.mob_entity.living_entity.entity;
        entity.set_synced_data(
            tracked_data::bogged::DATA_SHEARED,
            self.sheared.load(Ordering::Relaxed),
        );
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("sheared", self.is_sheared());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(sheared) = nbt.get_bool("sheared") {
            self.set_sheared(sheared);
        }
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        if item_stack.item.id == Item::SHEARS.id && !self.is_sheared() {
            let entity = &self.entity.mob_entity.living_entity.entity;
            let world = entity.world.load();
            let pos = entity.pos.load();

            world.play_sound(Sound::EntityBoggedShear, SoundCategory::Players, &pos);
            self.set_sheared(true);

            // Vanilla drops 2 red mushrooms and 2 brown mushrooms
            let spawn_pos = pos + Vector3::new(0.0, 1.0, 0.0);
            let red_drop = Arc::new(ItemEntity::new(
                Entity::new(world.clone(), spawn_pos, &EntityType::ITEM),
                ItemStack::new(2, &Item::RED_MUSHROOM),
            ));
            world.spawn_entity(red_drop);

            let brown_drop = Arc::new(ItemEntity::new(
                Entity::new(world.clone(), spawn_pos, &EntityType::ITEM),
                ItemStack::new(2, &Item::BROWN_MUSHROOM),
            ));
            world.spawn_entity(brown_drop);

            player.damage_held_item(1);
            return true;
        }

        self.entity.mob_interact(player, item_stack)
    }
}
