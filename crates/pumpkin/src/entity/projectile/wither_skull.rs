use std::sync::atomic::{AtomicBool, Ordering};

use pumpkin_data::{damage::DamageType, effect::StatusEffect, potion::Effect, tracked_data};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::{Difficulty, math::vector3::Vector3};

use crate::{
    entity::{
        Entity, EntityBase,
        projectile::{ProjectileHit, ThrownItemEntity},
    },
    server::Server,
    world::ExplosionInteraction,
};

const GRAVITY: f64 = 0.0;

pub struct WitherSkullEntity {
    pub thrown: ThrownItemEntity,
    pub dangerous: AtomicBool,
}

impl WitherSkullEntity {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        let thrown = ThrownItemEntity {
            entity,
            owner_id: None,
            collides_with_projectiles: false,
            has_hit: AtomicBool::new(false),
            gravity: GRAVITY,
        };

        Self {
            thrown,
            dangerous: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub fn new_shot(
        entity: Entity,
        shooter: &Entity,
        dangerous: bool,
        direction: Vector3<f64>,
    ) -> Self {
        let spawn_pos = entity.pos.load();
        let thrown = ThrownItemEntity::new(entity, shooter, GRAVITY);
        thrown.entity.pos.store(spawn_pos);

        let speed = 0.95;
        let vel = direction.normalize().multiply(speed, speed, speed);
        thrown.entity.velocity.store(vel);

        let len = vel.horizontal_length();
        let yaw = (vel.z.atan2(vel.x).to_degrees() as f32) - 90.0;
        let pitch = -(vel.y.atan2(len).to_degrees() as f32);
        thrown.entity.set_rotation(yaw, pitch);

        Self {
            thrown,
            dangerous: AtomicBool::new(dangerous),
        }
    }

    #[must_use]
    pub fn is_dangerous(&self) -> bool {
        self.dangerous.load(Ordering::Relaxed)
    }

    pub fn set_dangerous(&self, dangerous: bool) {
        self.dangerous.store(dangerous, Ordering::Relaxed);
        self.thrown
            .entity
            .set_synced_data(tracked_data::wither_skull::DATA_DANGEROUS, dangerous);
    }
}

impl EntityBase for WitherSkullEntity {
    fn get_owner_id(&self) -> Option<i32> {
        self.thrown.owner_id
    }

    fn write_custom_nbt(&self, nbt: &mut NbtCompound) {
        nbt.put_bool("dangerous", self.is_dangerous());
    }

    fn read_custom_nbt(&self, nbt: &NbtCompound) {
        if let Some(dangerous) = nbt.get_bool("dangerous") {
            self.dangerous.store(dangerous, Ordering::Relaxed);
        }
    }

    fn init_data_tracker(&self) {
        let entity = self.get_entity();
        entity.set_synced_data(
            tracked_data::wither_skull::DATA_DANGEROUS,
            self.is_dangerous(),
        );
    }

    fn tick(&self, caller: &dyn EntityBase, _server: &Server) {
        self.thrown.process_tick(caller);
    }

    fn get_entity(&self) -> &Entity {
        self.thrown.get_entity()
    }

    fn get_living_entity(&self) -> Option<&crate::entity::living::LivingEntity> {
        None
    }

    fn cast_any(&self) -> &dyn std::any::Any {
        self
    }

    fn on_hit(&self, hit: ProjectileHit) {
        let world = self.get_entity().world.load();

        if let ProjectileHit::Entity { ref entity, .. } = hit {
            let difficulty = world.level_info.load().difficulty;

            let owner_opt = self
                .get_owner_id()
                .and_then(|id| world.get_entity_by_id(id));
            let damage_dealer: &dyn EntityBase = match &owner_opt {
                Some(owner) => owner.as_ref(),
                None => self,
            };

            let was_hurt = entity.damage(damage_dealer, 8.0, DamageType::WITHER_SKULL);

            if was_hurt {
                if !entity.get_entity().is_alive() {
                    // Vanilla: If entity was killed by wither skull, heal wither by 5.0 HP
                    if let Some(owner_id) = self.get_owner_id()
                        && let Some(owner) = world.get_entity_by_id(owner_id)
                        && let Some(living_owner) = owner.get_living_entity()
                    {
                        living_owner.heal(5.0);
                    }
                }

                let duration = match difficulty {
                    Difficulty::Hard => 800,   // 40 seconds
                    Difficulty::Normal => 200, // 10 seconds
                    Difficulty::Easy | Difficulty::Peaceful => 0,
                };

                if duration > 0 {
                    let effect = Effect {
                        effect_type: &StatusEffect::WITHER,
                        duration,
                        amplifier: 1,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        blend: true,
                    };
                    if let Some(player) = entity.get_player() {
                        player.add_effect(effect);
                    } else if let Some(living) = entity.get_living_entity() {
                        living.add_effect(effect);
                    }
                }
            }
        }

        let hit_pos = hit.hit_pos();
        world.explode(hit_pos, 1.0, ExplosionInteraction::Mob);
    }
}
