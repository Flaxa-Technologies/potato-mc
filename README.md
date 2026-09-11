<div align="center">

# 🥔 PotatoMC

**Next-Generation, Ultra-High-Performance Minecraft Server Engine**

[![Release](https://img.shields.io/github/v/release/Flaxa-Technologies/potato-mc?style=for-the-badge&color=e5a823)](https://github.com/Flaxa-Technologies/potato-mc/releases)
[![Website](https://img.shields.io/badge/Website-potatomc.flaxa.in-blue?style=for-the-badge)](https://potatomc.flaxa.in/)
[![Discord](https://img.shields.io/badge/Discord-Join%20Flaxa%20Studios-5865F2?style=for-the-badge&logo=discord&logoColor=white)](https://discord.com/invite/UUaNzfZyc6)
[![License](https://img.shields.io/badge/License-GPL--3.0-green?style=for-the-badge)](LICENSE)

</div>

---

## 🌟 Overview

**PotatoMC** is an ultra-fast, modern Minecraft server implementation built from the ground up in **Rust**. Engineered specifically to eliminate the garbage collection pauses, single-thread bottlenecks, and memory bloat inherent in legacy JVM servers, PotatoMC delivers unprecedented concurrency, rock-solid 20.0 TPS, and microsecond tick latencies.

Whether hosting massive survival networks, minigames, or custom game modes, PotatoMC provides the raw performance and modern architecture required for scalable multiplayer infrastructure.

---

## ⚡ Core Highlights

- 🚀 **Blazing Native Performance**: Written entirely in Rust with zero garbage collection overhead and minimal idle memory usage (~30MB vs 1GB+ on Java).
- 🧩 **Native Rust Plugin Architecture (potato-api)**: Write plugins directly in native Rust (.dll / .so). Experience zero-cost FFI abstractions, direct memory safety, and synchronous execution without JVM overhead.
- 🌐 **Cross-Play Networking**: Native dual-stack support for both Minecraft Java Edition and Minecraft Bedrock Edition clients.
- 🗺️ **Concurrent Chunk Pipeline**: Asynchronous world generation, radial distance streaming, and lock-free entity processing across all available CPU cores.
- 🛡️ **Memory Safety**: Guaranteed compile-time concurrency and memory safety, eliminating memory leaks and data races at the architectural level.

---

## 📥 Direct Downloads & Quick Start

### 1. Download Standalone Executables
Grab the official pre-compiled binaries from [GitHub Releases](https://github.com/Flaxa-Technologies/potato-mc/releases/latest):

| Operating System | Download Link | Architecture |
|---|---|---|
| **Windows** | [**pumpkin-windows-x86_64.exe**](https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.1.0/pumpkin-windows-x86_64.exe) | 64-bit |
| **Linux** | [**pumpkin-linux-x86_64**](https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.1.0/pumpkin-linux-x86_64) | 64-bit MUSL (Static) |

### 2. Run the Server

#### On Windows:
``powershell
# Place pumpkin-windows-x86_64.exe in your server folder and run:
.\pumpkin-windows-x86_64.exe
``

#### On Linux:
``bash
# Download and grant execution permissions:
curl -LO https://github.com/Flaxa-Technologies/potato-mc/releases/download/beta.1.0/pumpkin-linux-x86_64
chmod +x pumpkin-linux-x86_64
./pumpkin-linux-x86_64
``

The server will automatically generate world data and configuration files on first launch.

---

## 🔌 Plugin Development with Potato API

PotatoMC features a groundbreaking native plugin engine powered by [potato-api](https://github.com/Flaxa-Technologies/potato-api). Plugins compile to shared native libraries (cdylib) and communicate with the server at native machine speed.

`ust
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
`

Plugins placed in the plugins/ directory are loaded dynamically on server startup.

---

## 🤝 Community & Working With Us

PotatoMC is built by **Flaxa Studios** and an open community of engineers, server creators, and Rust enthusiasts.

We are actively seeking:
- **Core Developers & Contributors**: Help us implement new Minecraft mechanics, network protocol improvements, and optimizations.
- **Plugin Developers**: Build plugins, games, and tools on top of potato-api.
- **Server Testers**: Stress-test high player counts, world generation, and report edge cases.

### Get In Touch:
- 🌐 **Official Website**: [https://potatomc.flaxa.in/](https://potatomc.flaxa.in/)
- 💬 **Flaxa Studios Discord**: [Join Our Discord](https://discord.com/invite/UUaNzfZyc6) — chat directly with the core team, get developer roles, and participate in technical design discussions.
- 🐛 **Issue Tracker**: Report bugs or suggest features right here on [GitHub Issues](https://github.com/Flaxa-Technologies/potato-mc/issues).

---

<div align="center">
  <sub>Built with ❤️ by <a href="https://potatomc.flaxa.in/">Flaxa Studios</a>.</sub>
</div>