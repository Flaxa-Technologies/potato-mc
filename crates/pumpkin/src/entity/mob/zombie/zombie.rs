use crate::entity::Entity;
use crate::entity::mob::equipment::RegionalDifficulty;
use crate::entity::mob::zombie::ZombieEntityBase;
use crate::entity::mob::{Mob, MobEntity};
use crate::world::World;
use pumpkin_nbt::compound::NbtCompound;
use std::sync::Arc;

pub struct ZombieEntity {
    entity: Arc<ZombieEntityBase>,
}

impl ZombieEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let entity = ZombieEntityBase::new(entity);
        let zombie = Self { entity };
        let zombie_arc = Arc::new(zombie);

        // Vanilla parity: 5% chance of baby zombie, 5% of baby zombies spawn as Chicken Jockey
        if rand::random::<f32>() < 0.05 {
            zombie_arc.set_baby(true);
            if rand::random::<f32>() < 0.05 {
                let world = zombie_arc.entity.mob_entity.living_entity.entity.world.load();
                let pos = zombie_arc.entity.mob_entity.living_entity.entity.pos.load();
                let chicken_entity = Entity::new(world.clone(), pos, &pumpkin_data::entity::EntityType::CHICKEN);
                let chicken = crate::entity::passive::chicken::ChickenEntity::new(chicken_entity);
                chicken.is_chicken_jockey.store(true, std::sync::atomic::Ordering::Relaxed);
                world.spawn_entity_non_save(chicken.clone() as Arc<dyn crate::entity::EntityBase>);
                let chicken_base: Arc<dyn crate::entity::EntityBase> = chicken.clone();
                let zombie_base: Arc<dyn crate::entity::EntityBase> = zombie_arc.clone();
                chicken.mob_entity.living_entity.entity.add_passenger(
                    chicken_base,
                    zombie_base,
                );
            }
        }

        zombie_arc
    }

    #[must_use]
    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let entity = ZombieEntityBase::with_can_break_doors(entity, can_break_doors);
        let zombie = Self { entity };
        Arc::new(zombie)
    }
}

impl Mob for ZombieEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.entity.mob_entity
    }

    fn populate_default_equipment_slots(
        &self,
        world: &Arc<World>,
        difficulty: &RegionalDifficulty,
    ) {
        self.entity
            .populate_default_equipment_slots(world, difficulty);
    }

    fn populate_default_equipment_enchantments(&self, difficulty: &RegionalDifficulty) {
        self.entity
            .populate_default_equipment_enchantments(difficulty);
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.entity.mob_write_nbt(nbt);
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.entity.mob_read_nbt(nbt);
    }
}

impl ZombieEntity {
    #[must_use]
    pub fn can_break_doors(&self) -> bool {
        self.entity
            .can_break_doors
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_can_break_doors(&self, can_break: bool) {
        self.entity
            .can_break_doors
            .store(can_break, std::sync::atomic::Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_baby(&self) -> bool {
        self.entity
            .mob_entity
            .living_entity
            .entity
            .age
            .load(std::sync::atomic::Ordering::Relaxed)
            < 0
    }

    pub fn set_baby(&self, baby: bool) {
        let age = if baby { -24000 } else { 0 };
        self.entity
            .mob_entity
            .living_entity
            .entity
            .age
            .store(age, std::sync::atomic::Ordering::Relaxed);
        self.entity
            .mob_entity
            .living_entity
            .entity
            .set_synced_data(pumpkin_data::tracked_data::zombie::BABY, baby);
    }
}
