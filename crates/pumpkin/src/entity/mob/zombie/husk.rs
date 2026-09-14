use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::entity::mob::zombie::ZombieEntityBase;
use crate::entity::{
    Entity, EntityBase,
    mob::{Mob, MobEntity},
};
use pumpkin_data::entity::EntityType;
use pumpkin_nbt::compound::NbtCompound;

pub struct HuskEntity {
    pub entity: Arc<ZombieEntityBase>,
    conversion_time: AtomicI32,
}

impl HuskEntity {
    #[must_use]
    pub fn new(entity: Entity) -> Arc<Self> {
        let entity = ZombieEntityBase::new(entity);
        let zombie = Self {
            entity,
            conversion_time: AtomicI32::new(0),
        };
        Arc::new(zombie)
    }

    #[must_use]
    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let entity = ZombieEntityBase::with_can_break_doors(entity, can_break_doors);
        let zombie = Self {
            entity,
            conversion_time: AtomicI32::new(0),
        };
        Arc::new(zombie)
    }
}

impl Mob for HuskEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.entity.mob_entity
    }

    fn on_attack(&self, target: &dyn EntityBase) {
        if let Some(living) = target.get_living_entity() {
            let entity = &self.entity.mob_entity.living_entity.entity;
            let world = entity.world.load();
            let duration = match world.level_info.load().difficulty {
                pumpkin_util::Difficulty::Normal => 140 * 2,
                pumpkin_util::Difficulty::Hard => 140 * 3,
                pumpkin_util::Difficulty::Easy => 140,
                _ => 0,
            };
            if duration > 0 {
                living.add_effect(pumpkin_data::potion::Effect {
                    effect_type: &pumpkin_data::effect::StatusEffect::HUNGER,
                    duration,
                    amplifier: 0,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    blend: false,
                });
            }
        }
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        let entity = &self.entity.mob_entity.living_entity.entity;
        if entity.touching_water.load(Ordering::Relaxed) {
            let ticks = self.conversion_time.fetch_add(1, Ordering::Relaxed) + 1;
            if ticks >= 600 {
                // Converts to zombie after 30 seconds underwater
                let world = entity.world.load();
                let pos = entity.pos.load();
                let zombie = crate::entity::r#type::from_type(
                    &EntityType::ZOMBIE,
                    pos,
                    &world,
                    uuid::Uuid::new_v4(),
                );
                world.spawn_entity(zombie);
                entity.remove();
            }
        } else {
            self.conversion_time.store(0, Ordering::Relaxed);
        }
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.entity.mob_write_nbt(nbt);
        nbt.put_int(
            "DrownedConversionTime",
            self.conversion_time.load(Ordering::Relaxed),
        );
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.entity.mob_read_nbt(nbt);
        if let Some(time) = nbt.get_int("DrownedConversionTime") {
            self.conversion_time.store(time, Ordering::Relaxed);
        }
    }
}
