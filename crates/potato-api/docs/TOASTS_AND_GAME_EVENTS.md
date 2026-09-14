# Toast Notifications, Cooldowns & Game Events API

PotatoMC allows plugins to trigger client-side visual overlays—including Toast Popups, Item Cooldown Sweeps, and Minecraft Game Events.

---

## 1. Toast Notifications (`Toast`)

Show custom advancement-style toast banners in the top-right corner of the player's screen:

```rust
use potato_api::toast::{Toast, ToastFrame};

// Task frame (square border)
let task = Toast::task("New Quest Unlocked", "minecraft:compass");
player.send_toast(&task);

// Goal frame (shield border)
let goal = Toast::goal("100 Kills Milestone", "minecraft:diamond_sword");
player.send_toast(&goal);

// Challenge frame (fancy ornate star border)
let challenge = Toast::challenge("Dragon Slayer", "minecraft:dragon_head");
player.send_toast(&challenge);
```

---

## 2. Item Cooldown Tracker

Trigger grey wipe-animation cooldown sweeps on specific item types (like Paper's `Player#setCooldown`):

```rust
// Put ender pearls on a 5-second (100 ticks) cooldown
player.set_item_cooldown("minecraft:ender_pearl", 100);

// Check remaining ticks
if player.has_item_cooldown("minecraft:ender_pearl") {
    let remaining = player.get_item_cooldown("minecraft:ender_pearl");
    player.send_message(&format!("Ender Pearl is on cooldown for {} ticks!", remaining));
}
```

---

## 3. Game Events & Demo Popups

Send vanilla game event packets to adjust client rendering:

```rust
use potato_api::game_event::GameEvent;

// 1. Show Minecraft Demo Screen dialog
player.send_demo_screen();

// 2. Trigger Elder Guardian curse flash animation
player.send_game_event(GameEvent::ElderGuardianAppearance);

// 3. Play End credits roll
player.send_game_event(GameEvent::WinGameCredits);
```
