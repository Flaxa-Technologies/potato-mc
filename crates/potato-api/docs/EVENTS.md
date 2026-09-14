# Event System & Cancellation

The PotatoMC event system implements a type-safe publisher/observer pattern mirroring Paper and Bukkit, with zero JVM runtime penalties and sub-millisecond execution.

---

## 1. Registering Event Handlers

Register event listeners in `on_enable()` via `context.register_event(...)`:

```rust
use potato_api::event::{PlayerJoinEvent, BlockBreakEvent, Cancellable};
use potato_api::text::Component;

// 1. Listen to player joins
context.register_event(|event: &mut PlayerJoinEvent| {
    let name = event.player.name();
    let comp = Component::from_mini_message(&format!("<yellow>Welcome, <gold>{}</gold>!</yellow>", name));
    event.player.send_component(&comp);
});

// 2. Prevent breaking bedrock
context.register_event(|event: &mut BlockBreakEvent| {
    if event.block.is_type("minecraft:bedrock") {
        event.set_cancelled(true);
        if let Some(ref player) = event.player {
            player.send_message("§cBedrock is indestructible!");
        }
    }
});
```

---

## 2. Controlling Block Drops (`BlockBreakEvent`)

Like Paper's `BlockBreakEvent.setDropItems(boolean)`, PotatoMC allows plugins to suppress default block loot while still allowing the block to break. This is the foundation for lucky blocks, custom ore mining, and OP-block plugins:

```rust
use potato_api::event::{BlockBreakEvent, Cancellable};
use potato_api::types::ItemStack;

context.register_event(|event: &mut BlockBreakEvent| {
    // Check if the broken block is dirt or grass
    if event.block.is_type("dirt") || event.block.is_type("grass_block") {
        // Prevent vanilla dirt from dropping
        event.set_drop_items(false);

        // Give or drop custom OP loot instead!
        if let Some(ref player) = event.player {
            let mut reward = ItemStack::new("minecraft:diamond", 2);
            reward.set_custom_name("§b§lLucky Diamond");
            player.give_item(&reward);
        }
    }
});
```

- `event.drop_items()`: Returns `true` if vanilla loot will drop when the block breaks.
- `event.set_drop_items(false)`: Tells PotatoMC to skip vanilla block drops (`BlockFlags::SKIP_DROPS`), while allowing the break itself to complete.

---

## 3. Event Priorities

Event listeners execute in strict priority order. The earlier an event handler executes, the sooner it can inspect or cancel an event:

1. `EventPriority::Lowest` — First to observe, useful for low-level modifications.
2. `EventPriority::Low`
3. `EventPriority::Normal` (Default)
4. `EventPriority::High`
5. `EventPriority::Highest` — Runs right before action confirmation.
6. `EventPriority::Monitor` — Read-only observation; actions MUST NOT be altered here.

---

## 4. The `Cancellable` Trait

Events that can be prevented implement the `Cancellable` trait:

```rust
pub trait Cancellable {
    fn is_cancelled(&self) -> bool;
    fn set_cancelled(&mut self, cancelled: bool);
}
```

Calling `event.set_cancelled(true)` instructs PotatoMC to abort the underlying action (e.g. block breaking, block placing, teleportation, player chat, inventory interaction).

---

## 5. Complete 59-Event Catalog

| ID | Event Struct | Cancellable | Key Fields & Methods | Description |
|:---:|---|:---:|---|---|
| **1** | `PlayerJoinEvent` | **Yes** | `player`, `join_message` | Player connected and spawned into world. |
| **2** | `PlayerQuitEvent` | No | `player`, `quit_message` | Player disconnected from server. |
| **3** | `PlayerMoveEvent` | **Yes** | `player`, `from`, `to` | Player moved or rotated in the world. |
| **4** | `PlayerInteractEvent` | **Yes** | `player`, `action`, `clicked_block`, `block_pos` | Player right/left clicked block or air. |
| **5** | `BlockBreakEvent` | **Yes** | `player`, `block`, `location`, `drop_items()`, `set_drop_items(bool)` | Block broken by player. Supports suppressing vanilla loot drops. |
| **6** | `BlockPlaceEvent` | **Yes** | `player`, `block`, `location` | Block placed by player. |
| **7** | `EntitySpawnEvent` | **Yes** | `entity`, `location` | Entity spawning in world. |
| **8** | `EntityDeathEvent` | No | `entity`, `killer`, `death_message`, `dropped_exp` | Living entity killed. |
| **9** | `EntityDamageEvent` / `DamageEvent` | **Yes** | `entity`, `damager`, `damage` | Entity damaged by attack or environment. |
| **10** | `PlayerChatEvent` | **Yes** | `player`, `message` | Player dispatched public chat message. |
| **11** | `PlayerCommandPreprocessEvent` | **Yes** | `player`, `command` | Player issued command prior to execution. |
| **12** | `PlayerDropItemEvent` | **Yes** | `player`, `item` | Player dropped item from inventory (`Q` key). |
| **13** | `PlayerItemConsumeEvent` | **Yes** | `player`, `item` | Player consumed food or potion. |
| **14** | `PlayerRespawnEvent` | No | `player`, `respawn_location`, `is_bed_spawn` | Player clicked respawn screen. |
| **15** | `PlayerTeleportEvent` | **Yes** | `player`, `from`, `to` | Player teleported across locations or worlds. |
| **16** | `PlayerGameModeChangeEvent` | **Yes** | `player`, `new_gamemode` | Player gamemode changed (Survival, Creative, etc.). |
| **17** | `PlayerToggleSneakEvent` | **Yes** | `player`, `is_sneaking` | Sneak/crouch state changed. |
| **18** | `PlayerToggleSprintEvent` | **Yes** | `player`, `is_sprinting` | Sprint state changed. |
| **19** | `PlayerToggleFlightEvent` | **Yes** | `player`, `is_flying` | Flight state toggled on or off. |
| **20** | `PlayerItemHeldEvent` | **Yes** | `player`, `previous_slot`, `new_slot` | Selected hotbar slot changed. |
| **21** | `InventoryClickEvent` | **Yes** | `player`, `slot`, `click_type`, `clicked_item`, `cursor_item` | Clicked a slot in any container or custom GUI. |
| **22** | `ServerListPingEvent` | No | `motd`, `online_players`, `max_players` | Client queried multiplayer server status list. |
| **23** | `InventoryOpenEvent` | **Yes** | `player`, `title` | Container screen opened for player. |
| **24** | `InventoryCloseEvent` | No | `player`, `title` | Container screen closed by player. |
| **25** | `ServerTickStartEvent` | No | `tick_number` | Initiating server tick cycle (50ms tick). |
| **26** | `ServerTickEndEvent` | No | `tick_number`, `duration_millis` | Completed server tick cycle with profiling duration. |
| **27** | `PlayerPickupItemEvent` | **Yes** | `player`, `item` | Ground item picked up into inventory. |
| **28** | `PlayerDeathEvent` | No | `player`, `drops`, `dropped_exp`, `death_message`, `keep_inventory`, `keep_level` | Player died. Supports modifying drops, exp, and keep inventory flags. |
| **29** | `PlayerLevelChangeEvent` | No | `player`, `old_level`, `new_level` | Player experience level changed. |
| **30** | `PlayerExpChangeEvent` | No | `player`, `amount` | Player experience points gained or spent. |
| **31** | `PlayerBedEnterEvent` | **Yes** | `player`, `bed_location` | Player attempted to sleep in a bed. |
| **32** | `PlayerBedLeaveEvent` | No | `player`, `bed_location` | Player left a bed. |
| **33** | `SignChangeEvent` | **Yes** | `player`, `location`, `lines: [String; 4]` | Player finished editing text on a sign. |
| **34** | `ServerCommandEvent` | **Yes** | `sender`, `command` | Console or remote sender executed a command. |
| **35** | `WeatherChangeEvent` | **Yes** | `world`, `to_weather_state` | World rain/storm state changing. |
| **36** | `ThunderChangeEvent` | **Yes** | `world`, `to_thunder_state` | World thunder state changing. |
| **37** | `ExplosionEvent` | **Yes** | `location`, `yield_rate` | Explosion occurred in the world. |
| **38** | `ProjectileLaunchEvent` | **Yes** | `entity`, `shooter` | Arrow, snowball, or projectile launched. |
| **39** | `ProjectileHitEvent` | **Yes** | `entity`, `hit_entity`, `hit_block` | Projectile impacted an entity or block. |
| **40** | `EntityTargetEvent` | **Yes** | `entity`, `target` | Mob or entity selected a new target. |
| **41** | `DialogShowEvent` | **Yes** | `player`, `dialog_id` | Native Dialog Box is about to display on player screen. |
| **42** | `DialogClickActionEvent` | **Yes** | `player`, `action_id`, `payload` | Player clicked an action button inside a Dialog Box. |
| **43** | `DialogClearEvent` | No | `player` | Player closed or dismissed a Dialog Box. |
| **44** | `PlayerFormResponseEvent` | **Yes** | `player`, `form_id`, `response_json` | Player submitted response to a Bedrock / Crossplay Form dialog. |
| **45** | `PlayerPortalEvent` | **Yes** | `player`, `from`, `to` | Player entered nether/end portal with destination mapping. |
| **46** | `PlayerItemBreakEvent` | No | `player`, `broken_item` | Tool, weapon, or armor durability depleted and broke. |
| **47** | `PlayerBucketEmptyEvent` | **Yes** | `player`, `block_clicked`, `bucket_item` | Player emptied water, lava, or powder snow bucket. |
| **48** | `PlayerBucketFillEvent` | **Yes** | `player`, `block_clicked`, `bucket_item` | Player collected liquid or mob with an empty bucket. |
| **49** | `PlayerShearEntityEvent` | **Yes** | `player`, `entity`, `item` | Player sheared sheep, mooshroom, or snow golem. |
| **50** | `EntityDamageByBlockEvent` | **Yes** | `entity`, `damager_block`, `damage` | Entity damaged by cactus, magma, campfire, or falling anvil. |
| **51** | `EntityCombustEvent` | **Yes** | `entity`, `duration_secs` | Entity caught fire (sunlight, lava, fire aspect). |
| **52** | `EntityCombustByEntityEvent` | **Yes** | `entity`, `combuster`, `duration_secs` | Entity set on fire directly by another entity or flaming arrow. |
| **53** | `PlayerAdvancementDoneEvent` | No | `player`, `advancement_id` | Player achieved or completed an in-game advancement criteria. |
| **54** | `InventoryMoveItemEvent` | **Yes** | `source_slot`, `destination_slot`, `item` | Hopper or automation moved an item between containers. |
| **55** | `ServerBroadcastEvent` | **Yes** | `message` | Global announcement dispatched to all connected players. |
| **56** | `ScoreboardScoreChangeEvent` | **Yes** | `scoreboard_name`, `objective_name`, `entry`, `previous_score`, `new_score` | Scoreboard score entry updated or modified. |
| **57** | `NpcInteractEvent` | **Yes** | `player`, `npc_id`, `click_type`, `hand` | Player left-clicked (attacked) or right-clicked an NPC. |
| **58** | `PlayerMaceSmashEvent` | **Yes** | `player`, `target`, `fall_distance`, `damage` | Player executed a mace smash attack from height. |
| **59** | `WindChargeDetonateEvent` | **Yes** | `shooter`, `location`, `radius`, `knockback` | Wind charge burst projectile detonated in the world. |


