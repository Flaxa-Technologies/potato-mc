use std::sync::{Arc, Weak};

use crate::entity::ai::goal::ranged_attack::RangedAttackGoal;
use crate::entity::ai::goal::Goal;
use crate::entity::mob::equipment::RegionalDifficulty;
use crate::entity::mob::zombie::ZombieEntityBase;
use crate::entity::mob::{Mob, MobEntity, RangedAttackMob};
use crate::entity::projectile::arrow::ArrowPickup;
use crate::entity::projectile::trident::TridentEntity;
use crate::entity::{Entity, EntityBase};
use crate::world::World;
use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;

struct DrownedTridentAttackGoal {
    inner: RangedAttackGoal,
}

impl DrownedTridentAttackGoal {
    fn new(mob: Weak<dyn RangedAttackMob>, speed: f64, attack_interval: i32, attack_radius: f32) -> Self {
        Self {
            inner: RangedAttackGoal::new(mob, speed, attack_interval, attack_radius),
        }
    }
}

impl Goal for DrownedTridentAttackGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        let is_holding_trident = mob
            .get_mob_entity()
            .living_entity
            .entity_equipment
            .try_lock()
            .map_or(false, |eq| eq.get(&EquipmentSlot::MAIN_HAND).item.id == Item::TRIDENT.id);

        if !is_holding_trident {
            return false;
        }

        self.inner.can_start(mob)
    }

    fn should_continue(&self, mob: &dyn Mob) -> bool {
        let is_holding_trident = mob
            .get_mob_entity()
            .living_entity
            .entity_equipment
            .try_lock()
            .map_or(false, |eq| eq.get(&EquipmentSlot::MAIN_HAND).item.id == Item::TRIDENT.id);

        if !is_holding_trident {
            return false;
        }

        self.inner.should_continue(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        self.inner.start(mob);
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.inner.stop(mob);
    }

    fn tick(&mut self, mob: &dyn Mob) {
        self.inner.tick(mob);
    }

    fn should_run_every_tick(&self) -> bool {
        self.inner.should_run_every_tick()
    }

    fn controls(&self) -> crate::entity::ai::goal::Controls {
        self.inner.controls()
    }
}

pub struct DrownedEntity {
    pub entity: Arc<ZombieEntityBase>,
}

impl DrownedEntity {
    #[must_use]
    pub fn new(entity: Entity) -> Arc<Self> {
        let entity = ZombieEntityBase::new(entity);
        let zombie = Self { entity };
        let mob_arc = Arc::new(zombie);

        let ranged_weak: Weak<dyn RangedAttackMob> = {
            let ranged_arc: Arc<dyn RangedAttackMob> = mob_arc.clone();
            Arc::downgrade(&ranged_arc)
        };

        {
            let mut goal_selector = mob_arc
                .entity
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            goal_selector.add_goal(
                2,
                Box::new(DrownedTridentAttackGoal::new(ranged_weak, 1.0, 40, 10.0)),
            );
        };

        mob_arc
    }

    #[must_use]
    pub fn with_can_break_doors(entity: Entity, can_break_doors: bool) -> Arc<Self> {
        let entity = ZombieEntityBase::with_can_break_doors(entity, can_break_doors);
        let zombie = Self { entity };
        Arc::new(zombie)
    }
}

impl Mob for DrownedEntity {
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

        let mut equipment = self
            .entity
            .mob_entity
            .living_entity
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Vanilla: 6.25% chance to spawn holding a trident in main hand
        if rand::random::<f32>() < 0.0625 {
            equipment.put(&EquipmentSlot::MAIN_HAND, ItemStack::new(1, &Item::TRIDENT));
        }

        // Vanilla: 3% chance to spawn holding a nautilus shell in off hand
        if rand::random::<f32>() < 0.03 {
            equipment.put(&EquipmentSlot::OFF_HAND, ItemStack::new(1, &Item::NAUTILUS_SHELL));
        }
    }

    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        self.entity.mob_write_nbt(nbt);
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        self.entity.mob_read_nbt(nbt);
    }
}

impl RangedAttackMob for DrownedEntity {
    fn perform_ranged_attack(&self, target: &Arc<dyn EntityBase>, _power: f32) {
        let entity = &self.entity.mob_entity.living_entity.entity;
        let world = entity.world.load_full();
        let drowned_pos = entity.pos.load();
        let target_entity = target.get_entity();
        let target_pos = target_entity.pos.load();

        let trident_entity = Entity::new(world.clone(), drowned_pos, &EntityType::TRIDENT);
        let trident_stack = ItemStack::new(1, &Item::TRIDENT);
        let trident = TridentEntity::new_shot(
            trident_entity,
            entity,
            trident_stack,
            ArrowPickup::Disallowed,
        );

        let dx = target_pos.x - drowned_pos.x;
        let dy = (target_pos.y + f64::from(target_entity.entity_dimension.load().height) / 3.0)
            - trident.entity.pos.load().y;
        let dz = target_pos.z - drowned_pos.z;
        let horizontal_dist = dx.hypot(dz);

        let diff = world.level_info.load().difficulty as u8 as f64;
        trident.set_velocity(
            dx,
            horizontal_dist.mul_add(0.2, dy),
            dz,
            1.6,
            14.0 - diff * 4.0,
        );

        world.play_sound(
            Sound::ItemTridentThrow,
            SoundCategory::Hostile,
            &drowned_pos,
        );
        world.spawn_entity(Arc::new(trident));
    }
}
