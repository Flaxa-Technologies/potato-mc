# Advanced Entity & LivingEntity API

PotatoMC provides control over entity flags, scoreboard tags, glowing shaders, invulnerability, and camera geometry.

---

## 1. Entity Flags & Visual Shaders

```rust
// Glowing highlight outline (visible through blocks)
entity.set_glowing(true);

// Make entity invulnerable to damage
entity.set_invulnerable(true);

// Silence all footsteps, breathing, and ambient noises
entity.set_silent(true);

// Disable gravity (makes entity hover in place)
entity.set_gravity(false);
```

---

## 2. Scoreboard Tags

Attach arbitrary string tags to entities for easy querying (used extensively in datapacks and minigames):

```rust
// Add tags
entity.add_scoreboard_tag("boss");
entity.add_scoreboard_tag("phase_2");

// Query tags
let tags = entity.scoreboard_tags();
if tags.contains(&"boss".to_string()) {
    println!("Found boss entity!");
}

// Remove tag
entity.remove_scoreboard_tag("phase_2");
```

---

## 3. LivingEntity Camera & Attributes

```rust
if let Some(living) = entity.as_living() {
    let eye = living.eye_location();
    let height = living.eye_height();
    println!("Living entity eye at Y: {:.2} (offset: {:.2}m)", eye.y, height);
}
```
