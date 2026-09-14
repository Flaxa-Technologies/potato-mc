<div align="center">
  <img src="https://github.com/Flaxa-Technologies/potato-api/raw/main/LOGO.png" alt="PotatoMC Logo" width="220"/>
  <h1>PotatoMC</h1>
  <p><strong>Next-Generation, Ultra-High-Performance Minecraft Server Engine</strong></p>

  <p>
    <a href="https://github.com/Flaxa-Technologies/potato-mc/releases"><img src="https://img.shields.io/github/v/release/Flaxa-Technologies/potato-mc?style=for-the-badge&amp;color=e5a823" alt="Release"/></a>
    <a href="https://potatomc.flaxa.in/"><img src="https://img.shields.io/badge/Website-potatomc.flaxa.in-blue?style=for-the-badge" alt="Website"/></a>
    <a href="https://discord.com/invite/UUaNzfZyc6"><img src="https://img.shields.io/badge/Discord-Join%20Flaxa%20Studios-5865F2?style=for-the-badge&amp;logo=discord&amp;logoColor=white" alt="Discord"/></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0-green?style=for-the-badge" alt="License"/></a>
    <a href="https://github.com/Pumpkin-MC/Pumpkin"><img src="https://img.shields.io/badge/Fork%20of-PumpkinMC-orange.svg?style=for-the-badge" alt="PumpkinMC Upstream"/></a>
  </p>
</div>

---

## Overview

**PotatoMC** is an ultra-fast, modern Minecraft server implementation built from the ground up in **Rust**. Engineered specifically to eliminate the garbage collection pauses, single-thread bottlenecks, and memory bloat inherent in legacy JVM servers, PotatoMC delivers unprecedented concurrency, rock-solid 20.0 TPS, and microsecond tick latencies.

Whether hosting massive survival networks, minigames, or custom game modes, PotatoMC provides the raw performance and modern architecture required for scalable multiplayer infrastructure.

---

## Core Highlights

- **Blazing Native Performance**: Written entirely in Rust with zero garbage collection overhead and minimal idle memory usage (~30MB vs 1GB+ on Java).
- **Native Rust Plugin Architecture (`potato-api`)**: Write plugins directly in native Rust (`.dll` / `.so`). Experience zero-cost FFI abstractions, direct memory safety, and synchronous execution without JVM overhead.
- **Cross-Play Networking**: Native dual-stack support for both Minecraft Java Edition and Minecraft Bedrock Edition clients.
- **Concurrent Chunk Pipeline**: Asynchronous world generation, radial distance streaming, batched StageCache chunk system, and lock-free entity processing across all available CPU cores.
- **Memory Safety**: Guaranteed compile-time concurrency and memory safety, eliminating memory leaks and data races at the architectural level.

---

## Direct Downloads & Quick Start

### 1. Download Standalone Executables or Server JAR
Grab the official pre-compiled assets from [GitHub Releases](https://github.com/Flaxa-Technologies/potato-mc/releases/latest):

| Platform / Host | Download Link | Description |
|---|---|---|
| **Pterodactyl & Game Panels** | [**server.jar**](https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.2.0/server.jar) | Universal Bootstrap JAR for Pterodactyl, Multicraft, and existing game hosts |
| **Windows** | [**potato-windows-x86_64.exe**](https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.2.0/potato-windows-x86_64.exe) | 64-bit Standalone Windows Binary |
| **Linux** | [**potato-linux-x86_64**](https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.2.0/potato-linux-x86_64) | 64-bit MUSL Static Linux Binary |

### 2. Run the Server

#### Pterodactyl & Existing Hosts (Recommended):
Place `server.jar` in your server root directory and run with your host's standard startup command:
```bash
java -Xms1G -Xmx4G -jar server.jar
```
The bootstrap JAR will automatically detect your OS/architecture, download the matching native PotatoMC binary, and pipe console I/O seamlessly!

#### On Standalone Windows:
```powershell
# Place potato-windows-x86_64.exe in your server folder and run:
.\potato-windows-x86_64.exe
```

#### On Standalone Linux:
```bash
# Download and grant execution permissions:
curl -LO https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.2.0/potato-linux-x86_64
chmod +x potato-linux-x86_64
./potato-linux-x86_64
```

The server will automatically generate world data and configuration files on first launch.

---

## Building From Source

Prerequisites:
- [Rust toolchain](https://rustup.rs/) (1.85+ recommended)

```bash
# Clone the repository
git clone https://github.com/Flaxa-Technologies/potato-mc.git
cd potato-mc

# Build optimized release binary
cargo build --release -p pumpkin --bin pumpkin
```

The compiled binary will be located in `target/release/pumpkin` (or `pumpkin.exe` on Windows).

---

## Plugin Development with Potato API

PotatoMC features a groundbreaking native plugin engine powered by [`potato-api`](https://github.com/Flaxa-Technologies/potato-api). Plugins compile to shared native libraries (`cdylib`) and communicate with the server at native machine speed.

```rust
use potato_api::prelude::*;

#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn on_load(&mut self, context: &mut PluginContext) {
        context.logger().info("MyPlugin has loaded on PotatoMC!");
    }

    fn on_enable(&mut self, context: &mut PluginContext) {
        context.events().register_listener(EventListener::PlayerJoin(|event| {
            event.player().send_message("Welcome to PotatoMC!");
        }));
    }
}

declare_plugin!(MyPlugin);
```

Plugins placed in the `plugins/` directory are loaded dynamically on server startup.

---

## Community & Working With Us

PotatoMC is built by **Flaxa Studios** and an open community of engineers, server creators, and Rust enthusiasts.

We are actively seeking:
- **Core Developers & Contributors**: Help us implement new Minecraft mechanics, network protocol improvements, and optimizations.
- **Plugin Developers**: Build plugins, games, and tools on top of `potato-api`.
- **Server Testers**: Stress-test high player counts, world generation, and report edge cases.

### Get In Touch:
- **Official Website**: [https://potatomc.flaxa.in/](https://potatomc.flaxa.in/)
- **Flaxa Studios Discord**: [Join Our Discord](https://discord.com/invite/UUaNzfZyc6) - chat directly with the core team, get developer roles, and participate in technical design discussions.
- **Issue Tracker**: Report bugs or suggest features right here on [GitHub Issues](https://github.com/Flaxa-Technologies/potato-mc/issues).

---

## Attribution & License

PotatoMC is an open-source project based on [PumpkinMC](https://pumpkinmc.org/). We are grateful to the PumpkinMC contributors for their foundational work.

PotatoMC is released under the [GNU General Public License v3.0](LICENSE).
