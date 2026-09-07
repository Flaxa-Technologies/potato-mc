use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use pumpkin_data::entity::{EntityStatus, EntityType};
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::GameMode;

use crate::entity::{
    Entity, EntityBase,
    ai::goal::{
        active_target::ActiveTargetGoal, look_around::RandomLookAroundGoal,
        look_at_entity::LookAtEntityGoal, melee_attack::MeleeAttackGoal,
        move_towards_target::MoveTowardsTargetGoal, offer_flower::OfferFlowerGoal,
        revenge::RevengeGoal, wander_around::WanderAroundGoal,
    },
    mob::{Mob, MobEntity},
    player::Player,
};

/// Represents an Iron Golem, a powerful neutral mob that protects villagers and players.
///
/// Wiki: <https://minecraft.wiki/w/Iron_Golem>
pub struct IronGolemEntity {
    pub mob_entity: MobEntity,
    pub player_created: AtomicBool,
    pub attack_animation_tick: AtomicI32,
    pub offer_flower_tick: AtomicI32,
}

impl IronGolemEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mob_entity = MobEntity::new(entity);
        let iron_golem = Self {
            mob_entity,
            player_created: AtomicBool::new(false),
            attack_animation_tick: AtomicI32::new(0),
            offer_flower_tick: AtomicI32::new(0),
        };
        let mob_arc = Arc::new(iron_golem);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut attributes = mob_arc
                .mob_entity
                .living_entity
                .attributes
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(health) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::MAX_HEALTH.id)
            {
                health.base_value = 100.0;
                health.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(damage) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::ATTACK_DAMAGE.id)
            {
                damage.base_value = 14.0;
                damage.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(knockback_res) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::KNOCKBACK_RESISTANCE.id)
            {
                knockback_res.base_value = 1.0;
                knockback_res.dirty.store(true, Ordering::Relaxed);
            }
            if let Some(speed) =
                attributes.get_mut(&pumpkin_data::attributes::Attributes::MOVEMENT_SPEED.id)
            {
                speed.base_value = 0.25;
                speed.dirty.store(true, Ordering::Relaxed);
            }
        }
        mob_arc.mob_entity.living_entity.health.store(100.0);

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

            goal_selector.add_goal(1, Box::new(MeleeAttackGoal::new(1.0, true)));
            goal_selector.add_goal(2, Box::new(MoveTowardsTargetGoal::new(0.9, 32.0)));
            goal_selector.add_goal(4, Box::new(WanderAroundGoal::new(0.6)));
            goal_selector.add_goal(5, Box::new(OfferFlowerGoal::new()));
            goal_selector.add_goal(
                7,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(8, Box::new(RandomLookAroundGoal::default()));

            target_selector.add_goal(1, Box::new(RevengeGoal::new(true)));
            target_selector.add_goal(
                2,
                ActiveTargetGoal::with_default(&mob_arc.mob_entity, &EntityType::ZOMBIE, true),
            );
        };

        mob_arc
    }

    #[must_use]
    pub fn is_player_created(&self) -> bool {
        self.player_created.load(Ordering::Relaxed)
    }

    pub fn set_player_created(&self, value: bool) {
        self.player_created.store(value, Ordering::Relaxed);
        let entity = self.get_entity();
        let flag: u8 = u8::from(value);
        entity.set_synced_data(pumpkin_data::tracked_data::iron_golem::FLAGS_ID, flag);
    }

    pub fn offer_flower(&self, offer: bool) {
        let entity = self.get_entity();
        let world = entity.world.load();
        if offer {
            self.offer_flower_tick.store(400, Ordering::Relaxed);
            world.send_entity_status(entity, EntityStatus::OfferFlower, None);
        } else {
            self.offer_flower_tick.store(0, Ordering::Relaxed);
            world.send_entity_status(entity, EntityStatus::StopOfferFlower, None);
        }
    }

    #[must_use]
    pub fn get_offer_flower_tick(&self) -> i32 {
        self.offer_flower_tick.load(Ordering::Relaxed)
    }
}

impl Mob for IronGolemEntity {
    fn mob_write_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("PlayerCreated", self.is_player_created());
    }

    fn mob_read_nbt(&self, nbt: &NbtCompound) {
        if let Some(created) = nbt.get_bool("PlayerCreated") {
            self.set_player_created(created);
        }
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }

    fn mob_tick(&self, _caller: &dyn EntityBase) {
        let attack_tick = self.attack_animation_tick.load(Ordering::Relaxed);
        if attack_tick > 0 {
            self.attack_animation_tick.fetch_sub(1, Ordering::Relaxed);
        }

        let flower_tick = self.offer_flower_tick.load(Ordering::Relaxed);
        if flower_tick > 0 {
            self.offer_flower_tick.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn mob_init_data_tracker(&self) {
        let entity = self.get_entity();
        let flag: u8 = u8::from(self.is_player_created());
        entity.set_synced_data(pumpkin_data::tracked_data::iron_golem::FLAGS_ID, flag);
    }

    fn mob_interact(&self, player: &Arc<Player>, item_stack: &mut ItemStack) -> bool {
        if item_stack.item.id == Item::IRON_INGOT.id {
            let living = &self.mob_entity.living_entity;
            let current_health = living.health.load();
            let max_health = living.get_max_health();
            if current_health < max_health {
                living.set_health((current_health + 25.0).min(max_health));
                let entity = self.get_entity();
                let world = entity.world.load();
                let pos = entity.pos.load();
                world.play_sound(Sound::EntityIronGolemRepair, SoundCategory::Neutral, &pos);
                if player.gamemode.load() != GameMode::Creative {
                    item_stack.item_count = item_stack.item_count.saturating_sub(1);
                }
                return true;
            }
        }
        false
    }

    fn on_damage(
        &self,
        _damage_type: pumpkin_data::damage::DamageType,
        source: Option<&dyn EntityBase>,
    ) {
        if let Some(attacker) = source {
            if let Some(player) = attacker.get_player() {
                if !player.is_creative() && !player.is_spectator() {
                    let world = self.mob_entity.living_entity.entity.world.load();
                    if let Some(player_arc) =
                        world.get_player_by_id(player.living_entity.entity.entity_id)
                    {
                        self.mob_entity.set_target(Some(player_arc));
                    }
                }
            } else if let Some(living) = attacker.get_living_entity() {
                let world = self.mob_entity.living_entity.entity.world.load();
                if let Some(ent_arc) = world.get_entity_by_id(living.entity.entity_id) {
                    self.mob_entity.set_target(Some(ent_arc));
                }
            }
        }
    }

    fn on_attack(&self, target: &dyn EntityBase) {
        let entity = self.get_entity();
        let world = entity.world.load();
        let pos = entity.pos.load();
        world.play_sound(Sound::EntityIronGolemAttack, SoundCategory::Neutral, &pos);
        world.send_entity_status(entity, EntityStatus::StartAttacking, None);
        self.attack_animation_tick.store(10, Ordering::Relaxed);

        let target_entity = target.get_entity();
        let current_vel = target_entity.velocity.load();
        target_entity.velocity.store(pumpkin_util::math::vector3::Vector3::new(
            current_vel.x,
            current_vel.y + 0.4,
            current_vel.z,
        ));
        target_entity.send_velocity();
    }
}
