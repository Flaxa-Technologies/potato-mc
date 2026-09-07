use std::sync::{Arc, Weak};

use pumpkin_data::data_component_impl::EquipmentSlot;
use pumpkin_data::damage::DamageType;
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::entity::EntityType;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::potion::Effect;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        active_target::ActiveTargetGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, melee_attack::MeleeAttackGoal, revenge::RevengeGoal,
        swim::SwimGoal, wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity, equipment::RegionalDifficulty},
};
use crate::world::World;

pub struct WitherSkeletonEntity {
    pub mob_entity: MobEntity,
}

impl WitherSkeletonEntity {
    #[must_use]
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        {
            let mut equipment = mob_entity
                .living_entity
                .entity_equipment
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            equipment.put(&EquipmentSlot::MAIN_HAND, ItemStack::new(1, &Item::STONE_SWORD));
        }

        let mob = Self { mob_entity };
        let mob_arc = Arc::new(mob);
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
            let mut target_selector = mob_arc
                .mob_entity
                .target_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Vanilla WitherSkeleton goals: MeleeAttackGoal only, no ranged bow goal!
            goal_selector.add_goal(0, Box::new(SwimGoal::default()));
            goal_selector.add_goal(2, Box::new(MeleeAttackGoal::new(1.2, false)));
            goal_selector.add_goal(5, Box::new(WanderAroundGoal::new(1.0)));
            goal_selector.add_goal(
                6,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 8.0),
            );
            goal_selector.add_goal(6, Box::new(RandomLookAroundGoal::default()));

            // Target selector:
            target_selector.add_goal(1, Box::new(RevengeGoal::new(true)));
            target_selector.add_goal(
                2,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PLAYER, true),
            );
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::IRON_GOLEM, true),
            );
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::PIGLIN, true),
            );
            target_selector.add_goal(
                3,
                ActiveTargetGoal::with_default(
                    &mob_arc.mob_entity,
                    &EntityType::PIGLIN_BRUTE,
                    true,
                ),
            );
        };

        mob_arc
    }
}

impl Mob for WitherSkeletonEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn on_attack(&self, target: &dyn EntityBase) {
        // Vanilla WitherSkeleton: Inflict Wither effect for 200 ticks (10s) on hit
        if let Some(living) = target.get_living_entity() {
            living.add_effect(Effect {
                effect_type: &StatusEffect::WITHER,
                duration: 200,
                amplifier: 0,
                ambient: false,
                show_particles: true,
                show_icon: true,
                blend: false,
            });
        }
    }

    fn modify_incoming_damage(&self, amount: f32, damage_type: DamageType) -> f32 {
        // Vanilla WitherSkeleton: Immune to Wither effect damage
        if damage_type == DamageType::WITHER {
            0.0
        } else {
            amount
        }
    }

    fn populate_default_equipment_slots(
        &self,
        _world: &Arc<World>,
        _difficulty: &RegionalDifficulty,
    ) {
        let living = &self.mob_entity.living_entity;
        let mut equipment = living
            .entity_equipment
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        equipment.put(&EquipmentSlot::MAIN_HAND, ItemStack::new(1, &Item::STONE_SWORD));
    }

    fn populate_default_equipment_enchantments(&self, _difficulty: &RegionalDifficulty) {
        // Vanilla WitherSkeleton: Empty (does not get default enchanted equipment)
    }
}
