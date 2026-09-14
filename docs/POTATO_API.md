# PotatoMC Native Rust Plugin API Reference

## 1. Overview & Architecture

The **PotatoMC Native Plugin API** (`potato-api`) provides a high-level, stable, and idiomatic Rust interface for extending PotatoMC (Pumpkin). It allows developers to author native shared-library plugins (`cdylib` `.dll` / `.so`) with high performance, memory safety, and zero runtime overhead.

```
┌────────────────────────────────────────────────────────┐
│               Native Rust Plugin (.dll)                │
│   (e.g., test-plugin, moderation, minigames, combat)   │
└───────────────────────────┬────────────────────────────┘
                            │ potato-api (Stable Abstraction)
┌───────────────────────────▼────────────────────────────┐
│                    Host Context                        │
│   (Event Dispatcher, Command Bridge, Task Scheduler)   │
└───────────────────────────┬────────────────────────────┘
                            │ Internal Runtime
┌───────────────────────────▼────────────────────────────┐
│              PotatoMC / Pumpkin Server Core            │
└────────────────────────────────────────────────────────┘
```

### Key Design Goals:
1. **Safety Boundary**: Plugins do not directly manipulate raw server memory or unstable server internals. All access happens through safe handles (`Player`, `World`, `CommandContext`).
2. **Familiarity with PaperMC 26.2**: Concepts from Bukkit/Paper (Brigadier structured commands, event listeners, repeating schedulers, metadata) are mapped naturally into modern Rust.
3. **Structured Commands**: Raw string-slice arguments (`&[String]`) are replaced with typed argument definitions, automatic parser validation, subcommands, and tab-completion suggestions.
4. **Decoupled ABI**: The public API crate (`potato-api`) is standalone and has minimal dependencies (`uuid`), isolating plugin authors from changes to internal networking or entity storage crates.

---

## 2. Comparison with PaperMC 26.2

PotatoMC's API was designed by direct inspection of the official **PaperMC 26.2 (build 123-stable)** decompiled source tree (`io.papermc.paper.*` and `org.bukkit.*`):

| Paper 26.2 (Java) | PotatoMC (Rust) | Notes |
|---|---|---|
| `io.papermc.paper.command.brigadier.BasicCommand` / `Commands` | `potato_api::command::Command` | Fluent builder supporting literals, typed arguments, and executors |
| `CommandSourceStack` | `potato_api::command::CommandContext` | Provides `sender()`, `player()`, `get_string()`, `get_int()`, etc. |
| `CommandSender` (`Player` vs `ConsoleCommandSender`) | `potato_api::command::CommandSender` enum | Enum with `Player(Player)` and `Console(Arc<dyn HostConsole>)` |
| `org.bukkit.event.Event` / `@EventHandler` | `potato_api::event::Event` / `context.register_event` | Strongly typed events with closure-based blanket event handlers |
| `org.bukkit.event.Cancellable` | `event.cancelled` field | Boolean flag allowing events to be aborted |
| `org.bukkit.event.EventPriority` | `potato_api::event::EventPriority` | Lowest, Low, Normal, High, Highest, Monitor |
| `org.bukkit.scheduler.BukkitScheduler` | `potato_api::scheduler::Scheduler` | `run_task`, `run_task_later`, `run_task_repeating` |
| `org.bukkit.scheduler.BukkitTask` | `potato_api::scheduler::TaskHandle` | Atomic cancellation via `handle.cancel()` or `is_cancelled()` |
| `org.bukkit.entity.Player` | `potato_api::player::Player` | Safe wrapper with `name`, `uuid`, `location`, `teleport`, `send_message`, `inventory` |
| `org.bukkit.World` | `potato_api::world::World` | Block lookups (`get_block`, `set_block`), dimension identifiers |
| `org.bukkit.configuration.file.FileConfiguration` | `potato_api::config::PluginConfig` | TOML-backed key-value store in plugin data directory |

---

## 3. Plugin Lifecycle

Plugins implement the `Plugin` trait and export entrypoints using `potato_plugin!(PluginStruct)`:

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::potato_plugin;

#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("my-plugin", "1.0.0")
            .author("Developer")
            .description("An example PotatoMC plugin")
    }

    fn on_load(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin loaded!");
        Ok(())
    }

    fn on_enable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin enabled!");
        Ok(())
    }

    fn on_disable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin disabled!");
        Ok(())
    }
}

potato_plugin!(MyPlugin);
```

### Lifecycle Phases:
1. **Discovery & Validation**: Server verifies `POTATO_API_VERSION` export matches the server version.
2. **Instantiation**: `potato_create_plugin()` allocates the plugin boxed trait object.
3. **`on_load`**: Pre-initialization, config loading, and resource setup.
4. **`on_enable`**: Command registration, event listener attachment, and task scheduling.
5. **`on_disable`**: Graceful cleanup; all tasks and event listeners registered by the plugin are automatically deregistered by the host adapter.

---

## 4. Structured Command System

Commands use a builder pattern inspired by Paper's Brigadier integration:

### 4.1. Creating Commands & Subcommands

```rust
use potato_api::command::{Argument, Command, CommandContext, CommandResult, CommandSender};

let cmd = Command::tree("warp")
    .description("Teleport to designated warp locations")
    .permission("myplugin.command.warp")
    .subcommand(
        Command::tree("set")
            .description("Set a warp location")
            .player_only()
            .argument(Argument::word("name"))
            .executes(|ctx: &CommandContext| -> CommandResult {
                let warp_name = ctx.get_string("name")?;
                let player = ctx.player()?;
                let loc = player.location();
                player.send_message(&format!("Warp '{}' set at ({:.1}, {:.1}, {:.1})!", warp_name, loc.x, loc.y, loc.z));
                Ok(())
            }),
    )
    .subcommand(
        Command::tree("to")
            .description("Teleport to a warp")
            .player_only()
            .argument(Argument::word("name"))
            .executes(|ctx: &CommandContext| -> CommandResult {
                let warp_name = ctx.get_string("name")?;
                let player = ctx.player()?;
                player.send_message(&format!("Teleporting to '{}'...", warp_name));
                Ok(())
            }),
    );

context.register_command(cmd);
```

### 4.2. Argument Types
- `Argument::word(name)`: Single word token (no spaces).
- `Argument::string(name)`: Quoted or single string argument.
- `Argument::greedy_string(name)`: Captures remainder of command line (useful for broadcast/chat messages).
- `Argument::integer(name)` / `Argument::integer_range(name, min, max)`: Signed 32-bit integer with bounds validation.
- `Argument::float(name)` / `Argument::float_range(name, min, max)`: 32-bit floating point coordinate/value.
- `Argument::boolean(name)`: Parses `true` / `false`.
- `Argument::player(name)`: Automatically validates and resolves online player by name.

### 4.3. Tab-Completion Suggestions
```rust
Argument::word("gamemode").suggests(|_ctx| {
    vec!["survival".to_string(), "creative".to_string(), "adventure".to_string(), "spectator".to_string()]
})
```

---

## 5. Event System

### 5.1. Registering Listeners
Event handlers receive mutable references to event objects, allowing cancellation and message mutation:

```rust
use potato_api::event::{BlockBreakEvent, PlayerJoinEvent, PlayerQuitEvent};

// Welcome joining players
context.register_event(move |event: &mut PlayerJoinEvent| {
    let name = event.player.name();
    event.player.send_message(&format!("Welcome back, {}!", name));
});

// Cancel block breaking in protected areas
context.register_event(move |event: &mut BlockBreakEvent| {
    if event.location.y < 10.0 {
        event.cancelled = true;
        if let Some(ref player) = event.player {
            player.send_message("You cannot break bedrock layer blocks!");
        }
    }
});
```

### 5.2. Core Event Catalog
| Event | Event ID | Description |
|---|---|---|
| `PlayerJoinEvent` | 1 | Player completed login and spawned into the world |
| `PlayerQuitEvent` | 2 | Player disconnected from server |
| `PlayerMoveEvent` | 3 | Player position or rotation updated |
| `PlayerInteractEvent` | 4 | Player clicked block or air (left/right click) |
| `BlockBreakEvent` | 5 | Block was destroyed by player or environment |
| `BlockPlaceEvent` | 6 | Block was placed against another block |
| `EntitySpawnEvent` | 7 | Mob or projectile spawned into world |
| `EntityDeathEvent` | 8 | Living entity died |
| `DamageEvent` | 9 | Entity received damage from source |

---

## 6. Task Scheduler

The scheduler handles asynchronous and timed execution without blocking the main server tick loop:

```rust
use std::time::Duration;

// 1. Fire-and-forget immediate task
context.scheduler().run_task(|| {
    // Background work
});

// 2. Delayed one-shot task (e.g., after 3 seconds)
context.scheduler().run_task_later(Duration::from_secs(3), || {
    // Delayed action
});

// 3. Repeating task with initial delay and period (e.g., every 10 seconds)
let handle = context.scheduler().run_task_repeating(
    Duration::from_secs(1),
    Duration::from_secs(10),
    move || {
        // Periodic check, autosave, or broadcast
    },
);

// Tasks can be cancelled at any time:
handle.cancel();
```

---

## 7. Player, World, and Block Abstractions

### 7.1. Player Operations
```rust
let player = context.get_player_by_name("Steve").unwrap();
println!("UUID: {}", player.uuid());
println!("Pos: {:?}", player.location());

// Messaging
player.send_message("Hello from PotatoMC native plugin!");

// Teleportation
let target_loc = Location::new("minecraft:overworld", 100.0, 64.0, 100.0, 0.0, 0.0);
player.teleport(&target_loc);

// Permissions
if player.has_permission("admin.tools") {
    // ...
}

// Inventory
let inv = player.inventory();
if let Some(held) = inv.held_item() {
    println!("Holding: {} x{}", held.item_type, held.amount);
}
```

### 7.2. World & Block Operations
```rust
if let Some(world) = context.get_world("minecraft:overworld") {
    // Block inspection
    if let Some(block) = world.get_block(0, 64, 0) {
        println!("Block at (0,64,0): {} (state: {})", block.block_type, block.state_id);
    }

    // Block modification
    let stone = Block::new("minecraft:stone", 1);
    world.set_block(0, 64, 0, &stone);
}
```

### 7.3. Configuration Management
Each plugin receives a dedicated data folder (`plugins/<PluginName>/`):
```rust
let mut config = context.config();
let welcome_msg = config.get_or_set("messages.welcome", "Welcome to the server!");
config.save().expect("Failed to save config.toml");
```

---

## 8. Building & Deploying Plugins

To compile a plugin into a dynamic library:
```toml
# Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
potato-api = { path = "../potato-api" }
```

Build command:
```bash
cargo build --release -p my-plugin
```

Copy the resulting binary into the server's `plugins/` directory:
- Windows: `target/release/my_plugin.dll` -> `plugins/my_plugin.dll`
- Linux: `target/release/libmy_plugin.so` -> `plugins/libmy_plugin.so`

When PotatoMC starts, it loads the plugin, registers its commands and listeners, and logs:
```
[INFO] Loaded Potato native plugin: my-plugin (version 0.1.0)
[INFO] Enabled Potato plugin: my-plugin v0.1.0
```
