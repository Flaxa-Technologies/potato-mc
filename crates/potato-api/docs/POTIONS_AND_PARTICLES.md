# Potions & Particle Effects API

PotatoMC provides native bindings for applying potion/status effects and spawning server-side particles for both Java and Bedrock clients.

---

## 1. Potion / Status Effects

The `PotionEffect` struct provides a fluent builder to configure status effects mirroring Paper's `PotionEffect`:

```rust
use potato_api::types::PotionEffect;

// 30 seconds of Speed II (ticks = 30 * 20 = 600, amplifier = 1)
let speed = PotionEffect::new("speed", 600, 1)
    .ambient(false)
    .show_particles(true)
    .show_icon(true);

// Apply to player
player.add_potion_effect(&speed);
```

### Inspecting & Managing Effects

```rust
// Check if player has an active effect
if player.has_potion_effect("speed") {
    println!("Player currently has speed!");
}

// Remove a specific effect
player.remove_potion_effect("speed");

// Clear all active potion effects
player.clear_potion_effects();
```

### Supported Status Effect Identifiers

Names can be specified with or without the `minecraft:` namespace prefix:

| Identifier | Description |
|---|---|
| `speed` | Increased movement speed |
| `slowness` | Decreased movement speed |
| `haste` | Faster mining and attack speed |
| `mining_fatigue` | Slower mining speed |
| `strength` | Increased melee damage |
| `instant_health` | Instant health restoration |
| `instant_damage` | Instant damage |
| `jump_boost` | Increased jump height |
| `regeneration` | Health regeneration over time |
| `resistance` | Damage reduction |
| `fire_resistance` | Immunity to fire and lava damage |
| `water_breathing` | Unlimited breath underwater |
| `invisibility` | Invisibility to players and mobs |
| `blindness` | Obscures player vision |
| `night_vision` | Full brightness in darkness |
| `hunger` | Accelerates food exhaustion |
| `weakness` | Decreased melee damage output |
| `poison` | Damages health over time (cannot kill) |
| `wither` | Damages health over time (can kill) |
| `glowing` | Outlines entity hitbox through walls |
| `levitation` | Involuntary upward floating |
| `slow_falling` | Gliding fall without fall damage |
| `conduit_power` | Aquatic visibility and buffs |
| `dolphins_grace` | Swift swimming speed |
| `bad_omen` | Triggers raid when entering a village |
| `hero_of_the_village` | Trade discounts from villagers |
| `darkness` | Pulsing darkness effect (Warden) |

---

## 2. Particle Effects

Spawn vanilla particle effects in the world or target specific players directly.

### Spawning World Particles

World particles are broadcast to all players within render distance:

```rust
use potato_api::types::Location;

let loc = Location::new("world", 128.5, 65.0, -320.5);

// spawn_particle(name, location, count, offset_x, offset_y, offset_z, speed)
world.spawn_particle(
    "flame",
    loc,
    25,     // count: 25 particles
    0.3,    // offset_x
    0.5,    // offset_y
    0.3,    // offset_z
    0.05    // speed / spread velocity
);
```

### Spawning Player Particles

Spawn particles visible only to a specific player (ideal for private quest markers or feedback effects):

```rust
player.spawn_particle(
    "heart",
    player.location(),
    10,
    0.5,
    0.5,
    0.5,
    0.1
);
```

### Common Particle Types

- `flame`
- `heart`
- `smoke` / `large_smoke`
- `crit` / `enchanted_hit`
- `portal`
- `soul_fire_flame`
- `explosion`
- `electric_spark`
- `totem_of_undying`
- `witch`
- `happy_villager` / `angry_villager`
