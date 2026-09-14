use super::{Entity, EntityBase, living::LivingEntity};
use pumpkin_data::BlockDirection;
use pumpkin_data::entity::EntityType;
use pumpkin_protocol::java::client::play::CEntityVelocity;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};
use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
};
pub mod arrow;
pub mod egg;
pub mod ender_pearl;
pub mod evoker_fangs;
pub mod eye_of_ender;
pub mod fireball;
pub mod firework_rocket;
pub mod fishing_bobber;
pub mod lingering_potion;
pub mod llama_spit;
pub mod shulker_bullet;
pub mod small_fireball;
pub mod snowball;
pub mod splash_potion;
pub mod trident;
pub mod wind_charge;
pub mod wither_skull;

use pumpkin_data::item_stack::ItemStack;

#[must_use]
pub fn is_projectile(entity_type: &EntityType) -> bool {
    *entity_type == EntityType::ARROW
        || *entity_type == EntityType::TRIDENT
        || *entity_type == EntityType::EGG
        || *entity_type == EntityType::SNOWBALL
        || *entity_type == EntityType::FIREWORK_ROCKET
        || *entity_type == EntityType::WIND_CHARGE
        || *entity_type == EntityType::SPLASH_POTION
        || *entity_type == EntityType::LINGERING_POTION
        || *entity_type == EntityType::ENDER_PEARL
        || *entity_type == EntityType::SHULKER_BULLET
        || *entity_type == EntityType::FIREBALL
        || *entity_type == EntityType::SMALL_FIREBALL
        || *entity_type == EntityType::FISHING_BOBBER
        || *entity_type == EntityType::WITHER_SKULL
        || *entity_type == EntityType::LLAMA_SPIT
}

/// Helper to apply projectile spawned enchantment effects matching vanilla `Projectile::applyOnProjectileSpawned`.
pub fn apply_on_projectile_spawned(
    projectile_entity: &Entity,
    pickup_item_stack: &ItemStack,
    weapon: Option<&ItemStack>,
    arrow: Option<&arrow::ArrowEntity>,
) {
    crate::enchantment::EnchantmentHelper::on_projectile_spawned(
        pickup_item_stack,
        projectile_entity,
        arrow,
    );
    if let Some(weapon) = weapon
        && weapon.item_count > 0
        && weapon.item.id != pickup_item_stack.item.id
    {
        crate::enchantment::EnchantmentHelper::on_projectile_spawned(
            weapon,
            projectile_entity,
            arrow,
        );
    }
}

pub struct ThrownItemEntity {
    pub entity: Entity,
    pub owner_id: Option<i32>,
    pub collides_with_projectiles: bool,
    pub has_hit: AtomicBool,
    pub gravity: f64,
}

impl ThrownItemEntity {
    pub fn new(entity: Entity, owner: &Entity, gravity: f64) -> Self {
        Self::new_with_eye_offset(entity, owner, gravity, -0.1)
    }

    pub fn new_with_eye_offset(
        entity: Entity,
        owner: &Entity,
        gravity: f64,
        eye_offset: f64,
    ) -> Self {
        let mut owner_pos = owner.pos.load();
        owner_pos.y += owner.get_eye_height() + eye_offset;
        entity.set_pos(owner_pos);
        entity.last_pos.store(owner_pos);
        Self {
            entity,
            owner_id: Some(owner.entity_id),
            collides_with_projectiles: false,
            has_hit: AtomicBool::new(false),
            gravity,
        }
    }

    pub fn set_velocity_from(&self, pitch: f32, yaw: f32, roll: f32, speed: f32, divergence: f32) {
        let yaw_rad = yaw.to_radians();
        let pitch_rad = pitch.to_radians();
        let roll_rad = (pitch + roll).to_radians();

        let x = -yaw_rad.sin() * pitch_rad.cos();
        let y = -roll_rad.sin();
        let z = yaw_rad.cos() * pitch_rad.cos();

        self.set_velocity(
            f64::from(x),
            f64::from(y),
            f64::from(z),
            f64::from(speed),
            f64::from(divergence),
        );
    }

    pub fn set_velocity(&self, x: f64, y: f64, z: f64, power: f64, uncertainty: f64) {
        fn next_triangular(mode: f64, deviation: f64) -> f64 {
            deviation.mul_add(rand::random::<f64>() - rand::random::<f64>(), mode)
        }
        let velocity = Vector3::new(x, y, z)
            .normalize()
            .add_raw(
                next_triangular(0.0, 0.017_227_5 * uncertainty),
                next_triangular(0.0, 0.017_227_5 * uncertainty),
                next_triangular(0.0, 0.017_227_5 * uncertainty),
            )
            .multiply(power, power, power);

        self.entity.velocity.store(velocity);
        let len = velocity.horizontal_length();
        self.entity.set_rotation(
            velocity.x.atan2(velocity.z) as f32 * 57.295_776,
            velocity.y.atan2(len) as f32 * 57.295_776,
        );
    }
}

impl ThrownItemEntity {
    /// Process a tick for projectile movement and collisions
    pub fn process_tick(&self, caller: &dyn EntityBase) {
        let entity = self.get_entity();
        let world = entity.world.load();

        entity.update_last_pos();

        // Apply gravity and inertia
        let mut velocity = entity.velocity.load();
        velocity.y -= self.get_gravity();

        let inertia = if entity.touching_water.load(Ordering::Relaxed) {
            0.8
        } else {
            0.99
        };
        velocity = velocity.multiply(inertia, inertia, inertia);

        // Store velocity
        entity.velocity.store(velocity);

        let start_pos = entity.pos.load();
        let delta = velocity;

        // Update position
        let new_pos = start_pos.add(&delta);
        entity.set_pos(new_pos);

        // Send updated velocity to clients
        let packet = CEntityVelocity::new(entity.entity_id.into(), velocity);
        let chunk_pos = entity.chunk_pos.load();
        world.broadcast_to_chunk(chunk_pos, &packet);

        // Calculate search box for collisions
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

        // Block collisions
        let (block_cols, block_positions) = world.get_block_collisions(search_box, caller);
        for (idx, bb) in block_cols.iter().enumerate() {
            if let Some((t, face)) = clip_aabb(&start_pos, &delta, bb)
                && t < closest_t
            {
                closest_t = t;
                // Map back to block pos
                for (cum_len, pos) in &block_positions {
                    if idx < *cum_len {
                        let hit_pos = start_pos.add(&delta.multiply(t, t, t));
                        hit = Some(ProjectileHit::Block {
                            pos: *pos,
                            face,
                            hit_pos,
                            normal: delta.normalize().multiply(-1.0, -1.0, -1.0),
                        });
                        break;
                    }
                }
            }
        }

        // Entity collisions (includes players via get_all_at_box)
        let candidates = world.get_all_at_box(&search_box);
        for cand in candidates {
            if self.should_skip_collision(entity, &cand) {
                continue;
            }

            let ebb = cand.get_entity().bounding_box.load().expand(0.3, 0.3, 0.3);
            if let Some(t) = calculate_ray_intersection(&start_pos, &delta, &ebb)
                && t < closest_t
            {
                closest_t = t;
                let hit_pos = start_pos.add(&delta.multiply(t, t, t));
                hit = Some(ProjectileHit::Entity {
                    entity: cand.clone(),
                    hit_pos,
                    normal: delta.normalize().multiply(-1.0, -1.0, -1.0),
                });
            }
        }

        // Handle hit or continue
        if let Some(h) = hit {
            // Ensure hit is only processed once per projectile
            if self.has_hit.swap(true, Ordering::SeqCst) {
                return;
            }

            if let ProjectileHit::Block { pos, hit_pos, .. } = &h {
                let block = world.get_block(pos);
                let state = world.get_block_state(pos);
                if let Some(server) = world.server.upgrade() {
                    world
                        .block_registry
                        .on_projectile_hit(block, &world, caller, pos, state, hit_pos, &server);
                }
            }

            // Just trigger hit effects and remove
            caller.on_hit(h);
            entity.remove();
        }
    }

    /// Returns if collision should be skipped (e.g. owner or projectile vs projectile)
    fn should_skip_collision(&self, self_ent: &Entity, other: &Arc<dyn EntityBase>) -> bool {
        let other_ent = other.get_entity();
        if other_ent.entity_id == self_ent.entity_id {
            return true;
        }

        // Skip collision with owner
        if Some(other_ent.entity_id) == self.owner_id {
            return true;
        }

        // Projectiles should pass through lingering clouds
        if *other_ent.entity_type == EntityType::AREA_EFFECT_CLOUD {
            return true;
        }

        // Projectile vs projectile logic
        if !self.collides_with_projectiles && is_projectile(other_ent.entity_type) {
            return true;
        }

        false
    }

    const fn get_entity(&self) -> &Entity {
        &self.entity
    }

    #[allow(dead_code, clippy::unused_self)]
    const fn get_living_entity(&self) -> Option<&LivingEntity> {
        None
    }
    const fn get_gravity(&self) -> f64 {
        self.gravity
    }
}

/// Ray intersection algorithm for AABBs, returning a t value
fn calculate_ray_intersection(
    start: &Vector3<f64>,
    dir: &Vector3<f64>,
    bb: &BoundingBox,
) -> Option<f64> {
    let mut t_min = 0.0f64;
    let mut t_max = 1.0f64;

    let b_min = [bb.min.x, bb.min.y, bb.min.z];
    let b_max = [bb.max.x, bb.max.y, bb.max.z];
    let s = [start.x, start.y, start.z];
    let d = [dir.x, dir.y, dir.z];

    for i in 0..3 {
        if d[i].abs() < 1e-9 {
            if s[i] < b_min[i] || s[i] > b_max[i] {
                return None;
            }
        } else {
            let t1 = (b_min[i] - s[i]) / d[i];
            let t2 = (b_max[i] - s[i]) / d[i];
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        }
    }

    (0.0..=1.0).contains(&t_min).then_some(t_min)
}

fn clip_point(
    scale_reference: &mut f64,
    direction: &mut Option<BlockDirection>,
    da: f64,
    db: f64,
    dc: f64,
    point: f64,
    min_b: f64,
    max_b: f64,
    min_c: f64,
    max_c: f64,
    new_direction: BlockDirection,
    from_a: f64,
    from_b: f64,
    from_c: f64,
) {
    let s = (point - from_a) / da;
    let pb = from_b + s * db;
    let pc = from_c + s * dc;
    if s >= -1.0e-5
        && s < *scale_reference
        && min_b - 1.0e-5 <= pb
        && pb <= max_b + 1.0e-5
        && min_c - 1.0e-5 <= pc
        && pc <= max_c + 1.0e-5
    {
        *scale_reference = s.max(0.0);
        *direction = Some(new_direction);
    }
}

/// Vanilla 26.2 `AABB.clip` implementation to determine ray intersection `t` and collision face.
pub fn clip_aabb(
    start: &Vector3<f64>,
    delta: &Vector3<f64>,
    bb: &BoundingBox,
) -> Option<(f64, BlockDirection)> {
    let dx = delta.x;
    let dy = delta.y;
    let dz = delta.z;

    // If ray start is already inside or right on the boundary of the bounding box,
    // immediately hit at t = 0.0 on the entry face matching the movement vector.
    if bb.min.x - 1.0e-5 <= start.x
        && start.x <= bb.max.x + 1.0e-5
        && bb.min.y - 1.0e-5 <= start.y
        && start.y <= bb.max.y + 1.0e-5
        && bb.min.z - 1.0e-5 <= start.z
        && start.z <= bb.max.z + 1.0e-5
    {
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();
        let abs_dz = dz.abs();
        let face = if abs_dy >= abs_dx && abs_dy >= abs_dz {
            if dy < 0.0 {
                BlockDirection::Up
            } else {
                BlockDirection::Down
            }
        } else if abs_dx >= abs_dz {
            if dx < 0.0 {
                BlockDirection::East
            } else {
                BlockDirection::West
            }
        } else if dz < 0.0 {
            BlockDirection::South
        } else {
            BlockDirection::North
        };
        return Some((0.0, face));
    }

    let mut scale = 1.0f64;
    let mut direction = None;

    if dx > 1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dx,
            dy,
            dz,
            bb.min.x,
            bb.min.y,
            bb.max.y,
            bb.min.z,
            bb.max.z,
            BlockDirection::West,
            start.x,
            start.y,
            start.z,
        );
    } else if dx < -1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dx,
            dy,
            dz,
            bb.max.x,
            bb.min.y,
            bb.max.y,
            bb.min.z,
            bb.max.z,
            BlockDirection::East,
            start.x,
            start.y,
            start.z,
        );
    }

    if dy > 1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dy,
            dz,
            dx,
            bb.min.y,
            bb.min.z,
            bb.max.z,
            bb.min.x,
            bb.max.x,
            BlockDirection::Down,
            start.y,
            start.z,
            start.x,
        );
    } else if dy < -1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dy,
            dz,
            dx,
            bb.max.y,
            bb.min.z,
            bb.max.z,
            bb.min.x,
            bb.max.x,
            BlockDirection::Up,
            start.y,
            start.z,
            start.x,
        );
    }

    if dz > 1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dz,
            dx,
            dy,
            bb.min.z,
            bb.min.x,
            bb.max.x,
            bb.min.y,
            bb.max.y,
            BlockDirection::North,
            start.z,
            start.x,
            start.y,
        );
    } else if dz < -1.0e-7 {
        clip_point(
            &mut scale,
            &mut direction,
            dz,
            dx,
            dy,
            bb.max.z,
            bb.min.x,
            bb.max.x,
            bb.min.y,
            bb.max.y,
            BlockDirection::South,
            start.z,
            start.x,
            start.y,
        );
    }

    direction.map(|dir| (scale, dir))
}

pub enum ProjectileHit {
    Block {
        pos: BlockPos,
        face: BlockDirection,
        hit_pos: Vector3<f64>,
        normal: Vector3<f64>,
    },
    Entity {
        entity: Arc<dyn EntityBase>,
        hit_pos: Vector3<f64>,
        normal: Vector3<f64>,
    },
}

impl ProjectileHit {
    /// Returns the exact impact coordinates regardless of what was hit.
    #[must_use]
    pub const fn hit_pos(&self) -> Vector3<f64> {
        match self {
            Self::Block { hit_pos, .. } | Self::Entity { hit_pos, .. } => *hit_pos,
        }
    }

    /// Returns the surface normal of the impact.
    #[must_use]
    pub const fn normal(&self) -> Vector3<f64> {
        match self {
            Self::Block { normal, .. } | Self::Entity { normal, .. } => *normal,
        }
    }

    /// Safely returns the face hit if it was a block, otherwise None.
    #[must_use]
    pub const fn face(&self) -> Option<BlockDirection> {
        match self {
            Self::Block { face, .. } => Some(*face),
            Self::Entity { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_util::math::boundingbox::BoundingBox;

    #[test]
    fn point_blank_shooting_down_hits_top_face() {
        // Ground block spanning Y: 63.0 to 64.0
        let ground_bb = BoundingBox::new(
            Vector3::new(0.0, 63.0, 0.0),
            Vector3::new(1.0, 64.0, 1.0),
        );

        // Projectile ray starting at top surface Y=64.0 shooting downward towards Y=62.5
        let start = Vector3::new(0.5, 64.0, 0.5);
        let end = Vector3::new(0.5, 62.5, 0.5);

        let delta = end - start;
        let hit = clip_aabb(&start, &delta, &ground_bb);
        assert!(hit.is_some(), "Point-blank ray at surface must hit the ground block");

        let (scale, face) = hit.unwrap();
        assert_eq!(face, BlockDirection::Up, "Must hit the top face (Up), not punch through to Down");
        assert!(scale <= 1e-4, "Scale must be ~0.0 at the point-blank surface");

        let hit_pos = start + (end - start) * scale;
        assert!((hit_pos.y - 64.0).abs() < 1e-4, "Impact point Y must be on ground surface (64.0)");
    }

    #[test]
    fn point_blank_just_above_surface_hits_top_face() {
        let ground_bb = BoundingBox::new(
            Vector3::new(0.0, 63.0, 0.0),
            Vector3::new(1.0, 64.0, 1.0),
        );

        // Ray starting just above surface Y=64.02 shooting downward towards Y=62.0
        let start = Vector3::new(0.5, 64.02, 0.5);
        let end = Vector3::new(0.5, 62.0, 0.5);

        let delta = end - start;
        let hit = clip_aabb(&start, &delta, &ground_bb);
        assert!(hit.is_some());

        let (scale, face) = hit.unwrap();
        assert_eq!(face, BlockDirection::Up);

        let hit_pos = start + (end - start) * scale;
        assert!((hit_pos.y - 64.0).abs() < 1e-4);
    }
}

