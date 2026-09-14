# Sounds & Environmental World API

PotatoMC offers rich audio and environmental controls for manipulating sound effects, weather, lightning, and world blocks.

---

## 1. Playing Sound Effects

Play vanilla sounds targeting either individual players or broadcast across an area in a world:

```rust
use potato_api::sound::{Sound, SoundCategory};

// 1. Play sound to a specific player
player.play_sound(Sound::ENTITY_PLAYER_LEVELUP, 1.0, 1.0);

// 2. Play sound with an explicit SoundCategory
player.play_sound_category(Sound::BLOCK_NOTE_BLOCK_PLING, SoundCategory::Blocks, 1.0, 1.2);

// 3. Stop playing sounds
player.stop_sound(Some("minecraft:music.game"));
player.stop_sound(None); // Stop all sounds

// 4. Play sound in the world at a position
world.play_sound(&location, Sound::ENTITY_GENERIC_EXPLODE, 1.0, 0.8);
```

### Sound Identifier Constants (`Sound`)

`potato_api::sound::Sound` includes commonly used constants:
- `Sound::ENTITY_PLAYER_LEVELUP`
- `Sound::ENTITY_EXPERIENCE_ORB_PICKUP`
- `Sound::BLOCK_NOTE_BLOCK_PLING`
- `Sound::BLOCK_CHEST_OPEN` / `BLOCK_CHEST_CLOSE`
- `Sound::ENTITY_GENERIC_EXPLODE`
- `Sound::ENTITY_LIGHTNING_BOLT_THUNDER`
- `Sound::UI_BUTTON_CLICK`
- `Sound::UI_TOAST_CHALLENGE_COMPLETE`

---

## 2. Environmental World Control

### Lightning Strikes
```rust
// Real lightning strike: creates fire, thunder sound, and deals damage
world.strike_lightning(&location);

// Cosmetic lightning effect: visual bolt and sound with NO fire or damage
world.strike_lightning_effect(&location);
```

### Weather & Thunder Control
```rust
// Toggle storm/rain
world.set_storm(true);
println!("Is raining: {}", world.is_raining());

// Toggle thundering
world.set_thundering(true);
println!("Is thundering: {}", world.is_thundering());
```

### Highest Block Query
```rust
// Find highest solid block Y coordinate at given X/Z
let highest_y = world.get_highest_block_y(100, -250);
```
