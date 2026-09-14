# Raytracing, BoundingBoxes & Line-of-Sight API

PotatoMC provides native vector math, axis-aligned bounding boxes (AABB), and raymarching algorithms matching Paper's `LivingEntity#getTargetBlock` and raytracing utilities.

---

## 1. Getting Targeted Blocks & Entities

Find the exact block or entity a player is looking at:

```rust
// 1. Target block within 15 blocks
if let Some(target_block) = player.get_target_block(15.0) {
    player.send_message(&format!("Looking at block: {}", target_block.name()));
}

// 2. Target entity within 20 blocks
if let Some(target_entity) = player.get_target_entity(20.0) {
    player.send_message(&format!("Targeted entity: {}", target_entity.entity_type()));
}
```

---

## 2. Player Eye Location & Facing Direction

Query exact camera origins and unit direction vectors:

```rust
let eye = player.eye_location();
let direction = player.facing_direction(); // Normalized unit Vector3

println!("Player eye at ({}, {}, {}) facing ({}, {}, {})",
    eye.x, eye.y, eye.z,
    direction.x, direction.y, direction.z
);
```

---

## 3. Custom Raymarching (`RayTrace`)

Raymarch arbitrary trajectories through custom voxel grids or worlds:

```rust
use potato_api::raytrace::RayTrace;
use potato_api::types::Vector3;

let start = eye.position();
let dir = player.facing_direction();

let result = RayTrace::trace_blocks(start, dir, 50.0, 0.2, |x, y, z| {
    world.get_block(x, y, z)
});

if let Some((hit_block, hit_loc, trace)) = result {
    println!("Ray hit {} at distance {:.2}m", hit_block.name(), trace.distance);
}
```

---

## 4. Axis-Aligned Bounding Box (`BoundingBox`)

Test bounding box containment, intersections, and slab-method ray hits:

```rust
use potato_api::types::{BoundingBox, Vector3};

let aabb = BoundingBox::new(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);

// Point containment
if aabb.contains(1.0, 1.0, 1.0) {
    println!("Point is inside the box!");
}

// Ray collision
let ray_start = Vector3::new(1.0, 1.0, -5.0);
let ray_dir = Vector3::new(0.0, 0.0, 1.0);
if let Some(distance) = aabb.raytrace(ray_start, ray_dir, 10.0) {
    println!("Ray intersected box at distance: {:.2}m", distance);
}
```
