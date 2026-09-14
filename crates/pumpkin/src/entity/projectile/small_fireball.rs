use std::sync::atomic::AtomicBool;

use pumpkin_protocol::java::client::play::CEntityVelocity;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

use crate::{
    entity::{
        Entity, EntityBase,
        projectile::{ProjectileHit, ThrownItemEntity, clip_aabb},
    },
    server::Server,
};

const GRAVITY: f64 = 0.0;

pub struct SmallFireballEntity {
    pub thrown: ThrownItemEntity,
}

impl SmallFireballEntity {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        let thrown = ThrownItemEntity {
            entity,
            owner_id: None,
            collides_with_projectiles: false,
            has_hit: AtomicBool::new(false),
            gravity: GRAVITY,
        };

        Self { thrown }
    }

    #[must_use]
    pub fn new_shot(entity: Entity, shooter: &Entity) -> Self {
        let thrown = ThrownItemEntity::new(entity, shooter, GRAVITY);
        Self { thrown }
    }
}

impl EntityBase for SmallFireballEntity {
    fn get_owner_id(&self) -> Option<i32> {
        self.thrown.owner_id
    }

    fn tick(&self, caller: &dyn EntityBase, _server: &Server) {
        let entity = self.thrown.get_entity();
        let world = entity.world.load();

        if !entity.is_alive() {
            return;
        }

        entity.update_last_pos();

        // Vanilla hurting projectile acceleration:
        // movement = (movement + movement.normalize() * 0.1) * 0.95
        let mut velocity = entity.velocity.load();
        let speed_sq = velocity.length_squared();
        if speed_sq > 0.00001 {
            let norm = velocity.normalize();
            velocity = (velocity + norm * 0.1).multiply(0.95, 0.95, 0.95);
        }
        entity.velocity.store(velocity);

        let start_pos = entity.pos.load();
        let delta = velocity;
        let new_pos = start_pos.add(&delta);
        entity.set_pos(new_pos);

        // Crucial: send position & rotation sync to clients!
        entity.send_pos_rot();
        let packet = CEntityVelocity::new(entity.entity_id.into(), velocity);
        let chunk_pos = entity.chunk_pos.load();
        world.broadcast_to_chunk(chunk_pos, &packet);

        // Check collisions
        let search_box = BoundingBox::new(
            Vector3::new(
                start_pos.x.min(new_pos.x),
                start_pos.y.min(new_pos.y),
                start_pos.z.min(new_pos.z),
            ),
            Vector3::new(
                start_pos.x.max(new_pos.x),
                start_pos.y.max(new_pos.y),
                start_pos.z.max(new_pos.z),
            ),
        )
        .expand(0.3, 0.3, 0.3);

        let mut closest_t = 1.0f64;
        let mut hit = None;

        let (block_cols, _) = world.get_block_collisions(search_box, caller);
        for bb in &block_cols {
            if let Some((t, face)) = clip_aabb(&start_pos, &delta, bb) {
                if t < closest_t {
                    closest_t = t;
                    let hit_pos = start_pos.add(&(delta * t));
                    hit = Some(ProjectileHit::Block {
                        pos: BlockPos(Vector3::new(
                            hit_pos.x.floor() as i32,
                            hit_pos.y.floor() as i32,
                            hit_pos.z.floor() as i32,
                        )),
                        face,
                        hit_pos,
                        normal: delta.normalize().multiply(-1.0, -1.0, -1.0),
                    });
                }
            }
        }

        let entity_cols = world.get_entities_at_box(&search_box);
        for other in entity_cols {
            if other.get_entity().entity_id == entity.entity_id {
                continue;
            }
            if let Some(owner) = self.thrown.owner_id
                && other.get_entity().entity_id == owner
            {
                continue;
            }
            let bb = other.get_entity().bounding_box.load().expand(0.3, 0.3, 0.3);
            if let Some((t, _face)) = clip_aabb(&start_pos, &delta, &bb) {
                if t < closest_t {
                    closest_t = t;
                    let hit_pos = start_pos.add(&(delta * t));
                    hit = Some(ProjectileHit::Entity {
                        entity: other,
                        hit_pos,
                        normal: delta.normalize().multiply(-1.0, -1.0, -1.0),
                    });
                }
            }
        }

        if let Some(hit) = hit {
            self.on_hit(hit);
            world.remove_entity(entity);
        }
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

    fn damage(
        &self,
        caller: &dyn EntityBase,
        _amount: f32,
        _damage_type: pumpkin_data::damage::DamageType,
    ) -> bool {
        let entity = self.get_entity();
        let mut vel = entity.velocity.load();
        if let Some(player) = caller.get_player() {
            let look = Vector3::rotation_vector(
                f64::from(player.living_entity.entity.pitch.load()),
                f64::from(player.living_entity.entity.yaw.load()),
            );
            vel = look * 0.5;
        } else {
            vel = vel * -1.0;
        }
        entity.velocity.store(vel);
        entity.send_pos_rot();
        true
    }

    fn on_hit(&self, hit: ProjectileHit) {
        match hit {
            ProjectileHit::Entity {
                ref entity,
                hit_pos,
                ..
            } => {
                entity.get_entity().set_on_fire_for(5.0);
                let world = self.get_entity().world.load();
                let shooter = self
                    .thrown
                    .owner_id
                    .and_then(|id| world.get_entity_by_id(id));
                let _ = entity.damage_with_context(
                    entity.as_ref(),
                    5.0,
                    pumpkin_data::damage::DamageType::FIREBALL,
                    Some(hit_pos),
                    Some(self),
                    shooter.as_deref(),
                );
            }
            ProjectileHit::Block { pos, face, .. } => {
                let block_to_place = match face {
                    pumpkin_data::BlockDirection::Up => pos.up(),
                    pumpkin_data::BlockDirection::Down => pos.down(),
                    pumpkin_data::BlockDirection::North => pos.north(),
                    pumpkin_data::BlockDirection::South => pos.south(),
                    pumpkin_data::BlockDirection::West => pos.west(),
                    pumpkin_data::BlockDirection::East => pos.east(),
                };
                let world = self.get_entity().world.load();
                let (_, target_state) = world.get_block_and_state(&block_to_place);
                if target_state.is_air() {
                    let fire_state = pumpkin_data::Block::FIRE.default_state.id;
                    world.set_block_state(
                        &block_to_place,
                        fire_state,
                        pumpkin_world::world::BlockFlags::NOTIFY_ALL,
                    );
                }
            }
        }
    }
}
