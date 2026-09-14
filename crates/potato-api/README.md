<p align="center">
  <img src="LOGO.png" alt="PotatoMC Logo" width="280"/>
</p>

# PotatoMC Native Plugin API (`potato-api`)

<p align="center">
  <b>High-Performance Paper-Grade Native Rust Plugin Development Kit for PotatoMC</b><br>
  Zero JVM Overhead • Sub-Millisecond Native Latency • Full Paper Parity • Zero GC Pauses
</p>

<p align="center">
  <a href="https://potatomc.flaxa.in/"><img src="https://img.shields.io/badge/Official_Website-potatomc.flaxa.in-FF8800?style=for-the-badge&logo=google-chrome&logoColor=white" alt="Website"></a>
  <a href="https://discord.com/invite/UUaNzfZyc6"><img src="https://img.shields.io/badge/Discord-Flaxa_Studios-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Discord"></a>
  <a href="https://github.com/Flaxa-Technologies/potato-api"><img src="https://img.shields.io/badge/GitHub-potato--api-181717?style=for-the-badge&logo=github&logoColor=white" alt="GitHub"></a>
  <img src="https://img.shields.io/badge/Rust-2024_Edition-black?style=for-the-badge&logo=rust" alt="Rust Edition">
  <img src="https://img.shields.io/badge/License-GPL--3.0-blue?style=for-the-badge" alt="License">
</p>

---

## 🥔 Welcome to PotatoMC

**PotatoMC** is a next-generation, native Minecraft server engine written in Rust. The **PotatoMC Native Plugin API (`potato-api`)** empowers developers to build ultra-fast, native plugins compiled directly to dynamic libraries:

- **Windows**: `.dll` (Dynamic-Link Library)
- **Linux**: `.so` (Shared Object)
- **macOS**: `.dylib` (Dynamic Library)

By executing natively in-process, PotatoMC plugins eliminate Java Virtual Machine (JVM) overhead, garbage-collection hitches, and JNI boundary penalties while delivering the rich developer experience of Bukkit, Spigot, and Paper.

---

## ⚡ Quick Start

### 1. One-Line Environment Setup (No Cloning Needed!)

Run the automated setup command directly in your terminal:

#### 🐧 Linux (Ubuntu, Debian, Arch, Fedora)
```bash
curl -sSL https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-linux.sh | bash
```

#### 🍎 macOS (Apple Silicon & Intel)
```bash
curl -sSL https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-macos.sh | bash
```

#### 🪟 Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-windows.ps1 | iex
```
*(Or from Command Prompt: `powershell -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-windows.ps1 | iex"`)*

---

### 2. Scaffold a New Plugin in Seconds

Scaffold a fresh plugin project instantly with a single command:

#### Linux & macOS (Bash):
```bash
curl -sSL https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/create-plugin.sh | bash -s MyAwesomePlugin "Your Name" linux
```

#### Windows (PowerShell):
```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/create-plugin.ps1))) -Name MyAwesomePlugin -Platform windows
```

#### Cross-Platform (Python):
```bash
python scripts/create-plugin.py MyAwesomePlugin --author "Your Name" --platform windows
```

### 3. Add to an Existing Project (`Cargo.toml`)

In your plugin's `Cargo.toml`:

```toml
[package]
name = "my-awesome-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
potato-api = { git = "https://github.com/Flaxa-Technologies/potato-api" }
```

---

## 🚀 Writing Your First Plugin

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::event::{PlayerJoinEvent, BlockBreakEvent, Cancellable};
use potato_api::command::{Command, CommandContext, CommandResult, CommandSender};
use potato_api::text::Component;
use potato_api::potato_plugin;

#[derive(Default)]
pub struct MyFirstPlugin;

impl Plugin for MyFirstPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("MyFirstPlugin", "1.0.0")
            .author("Developer")
            .description("My first native PotatoMC plugin")
    }

    fn on_enable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyFirstPlugin enabled at native speed!");

        // 1. Event: Welcome message with MiniMessage
        context.register_event(|event: &mut PlayerJoinEvent| {
            let welcome = format!(
                "<gradient:#ffaa00:#ff5555><bold>Welcome to PotatoMC, {}!</bold></gradient>",
                event.player.name()
            );
            event.player.send_component(&Component::from_mini_message(&welcome));
        });

        // 2. Event: Bedrock break cancellation
        context.register_event(|event: &mut BlockBreakEvent| {
            if event.block.block_type == "minecraft:bedrock" {
                event.set_cancelled(true);
                if let Some(ref player) = event.player {
                    player.send_message("§cYou cannot break bedrock!");
                }
            }
        });

        // 3. Command: /ping -> Pong!
        let cmd = Command::tree("ping")
            .description("Responds with Pong!")
            .executes(|ctx: &CommandContext| -> CommandResult {
                ctx.sender().send_message("§a[PotatoMC] Pong!");
                Ok(())
            });
        context.register_command(cmd);

        Ok(())
    }
}

// Export native entrypoint symbol
potato_plugin!(MyFirstPlugin);
```

### 4. Build and Deploy

#### On Windows:
```cmd
cargo build --release
copy target\release\my_awesome_plugin.dll C:\path\to\server\plugins\
```

#### On Linux:
```bash
cargo build --release
cp target/release/libmy_awesome_plugin.so /path/to/server/plugins/
```

#### On macOS:
```bash
cargo build --release
cp target/release/libmy_awesome_plugin.dylib /path/to/server/plugins/
```

Drop the compiled dynamic library directly into your PotatoMC server's `plugins/` directory and run your server!

---

## 📦 Production Templates

Pre-configured, production-ready templates are included in the `templates/` folder:

- 🪟 **[Windows Template (`.dll`)](templates/windows/)**: Includes `build.bat`, `build.ps1`, `Cargo.toml`, and comprehensive sample code.
- 🐧 **[Linux Template (`.so`)](templates/linux/)**: Includes `build.sh`, `Makefile`, `Cargo.toml`, and ELF-optimized configuration.
- 🍎 **[macOS Template (`.dylib`)](templates/macos/)**: Includes `build.sh`, `Cargo.toml`, and universal target setup.

Every template and scaffolded plugin automatically comes bundled with **`AGENT.md`** and **`CLAUDE.md`** containing complete architectural context, documentation links, build commands, and coding rules for AI coding assistants (Cursor, Claude, Copilot, Antigravity, Codex).

---

## 📖 Complete Documentation

Explore the complete API reference in the [`docs/`](docs/) directory:

- 🏗️ **[Plugin Lifecycle & Architecture](docs/LIFECYCLE.md)**
- ⚡ **[Event System & 59-Event Catalog](docs/EVENTS.md)** (with Block Break loot suppression, Dialog & Form events, World, Combat & NPC events)
- 🪟 **[Virtual GUI & Inventory API](docs/GUI_AND_INVENTORY.md)** (Chest, Hopper, and Dispenser GUIs, slot control, inventory manipulation)
- 💬 **[Dialog Box, Forms & Virtual Book API](docs/DIALOGS_AND_FORMS.md)** (Java 1.21.4+ Dialogs, Bedrock Forms, Books, Signs)
- 📊 **[Scoreboards & Teams API](docs/SCOREBOARDS_AND_TEAMS.md)** (Fast sidebar scoreboards, custom objectives, score change events, teams)
- 🔊 **[Sounds & Environmental World API](docs/SOUNDS_AND_ENVIRONMENT.md)** (Sound effects, lightning strikes, weather control)
- 🧪 **[Potions & Particle Effects](docs/POTIONS_AND_PARTICLES.md)** (Status effects builder and particle broadcasts)
- 🌲 **[Brigadier Command Tree API](docs/COMMANDS.md)**
- 🎨 **[Adventure Text & MiniMessage](docs/TEXT_AND_MINIMESSAGE.md)**
- 📦 **[Persistent Data Container (PDC) & Items](docs/PDC_AND_ITEMS.md)** (with Item Attack Cooldown tracking)
- ⏱️ **[Scheduler & Concurrency Model](docs/SCHEDULER.md)**
- ⚙️ **[Configuration API (YAML)](docs/CONFIGURATION.md)**
- 📊 **[Adventure BossBar API](docs/BOSSBAR.md)**
- 🎯 **[Line-of-Sight Raytracing & Targeting API](docs/RAYTRACING_AND_TARGETING.md)** (Voxel raymarching, bounding box tests, crosshair targeting)
- 🍞 **[Toasts, Game Events & Demo Screens](docs/TOASTS_AND_GAME_EVENTS.md)** (Advancement toasts, demo screens, client game events)
- 🏷️ **[Advanced Entity Flags, Tags & Manipulation](docs/ADVANCED_ENTITIES.md)** (Glowing, invulnerability, silence, gravity, scoreboard tags)
- 🤖 **[NPC & Fake Player API](docs/NPC_API.md)** (Fake players, skins, poses, interaction clicks, waypoint navigation, custom behaviors)
- 🎨 **[Custom Model Data & Item Variants](docs/CUSTOM_MODEL_DATA.md)** (Resource pack items, CustomModelData getters/setters, ItemVariantRegistry)
- 🔄 **[Paper Java to PotatoMC Rust Migration Guide](docs/PAPER_MIGRATION.md)**

---

## 💬 API Suggestions & Reporting Issues

We actively build and expand PotatoMC's plugin ecosystem around developer feedback!

- **💡 Have an API Suggestion or Feature Request?**
  - Join our Discord: [**Flaxa Studios Discord**](https://discord.com/invite/UUaNzfZyc6)
  - Post in `#api-suggestions` or `#plugin-development` to request new event hooks, packets, or host interfaces.
- **🐛 Found a Bug or Issue?**
  - Report on Discord: [**Flaxa Studios Discord**](https://discord.com/invite/UUaNzfZyc6) in the `#bugs-support` channel.
  - Or open an official issue ticket: [**GitHub Issues**](https://github.com/Flaxa-Technologies/potato-api/issues).
- **🌐 Official Website**: [**https://potatomc.flaxa.in/**](https://potatomc.flaxa.in/)
- **🏢 Organization**: Flaxa Studios / Flaxa Technologies

---

## ⚖️ License

The PotatoMC Native Plugin API is licensed under the [GNU General Public License v3.0](LICENSE).
Plugins built against `potato-api` can be licensed according to their author's choice.

