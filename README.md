<div align="center">

# PotatoMC

![Server Icon](./server-icon.png)

### Minecraft server software for potato PCs.

[![License: GPL](https://img.shields.io/badge/License-GPLv3-yellow.svg)](https://opensource.org/licenses/gpl-3-0)
[![Upstream: PumpkinMC](https://img.shields.io/badge/Fork%20of-PumpkinMC-orange.svg)](https://github.com/Pumpkin-MC/Pumpkin)
[![Discord](https://img.shields.io/discord/1268592337445978193.svg?label=Upstream%20Discord&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/wT8XjrjKkf)

</div>

**PotatoMC** is an open-source fork of [PumpkinMC](https://pumpkinmc.org/), a Minecraft server software written in Rust. PotatoMC builds on the Pumpkin foundation with a focus on performance, low hardware requirements, vanilla Minecraft compatibility, world generation, entity behavior, AI, networking, and server optimization.

> **Goal:**
> Minecraft server software for potato PCs.

<div align="center">

![Chunk Loading](./assets/pumpkin-chunk-loading.webp)

</div>

## Goals

- **Low Resource Usage**: Engineered to run smoothly on lower-end hardware and "potato PCs".
- **Performance**: Multi-threaded architecture leveraging Rust for speed and minimal memory footprint.
- **Vanilla 26.2 Compatibility**: Strict adherence to Vanilla Minecraft Java Edition 26.2 mechanics, networking, worldgen, and mob AI.
- **Security & Stability**: Proactive exploit prevention and robust crash reporting.
- **Extensibility**: Preserves the native and WASM plugin development framework.

> [!NOTE]
> PotatoMC is an actively developed fork building on upstream PumpkinMC.
> Upstream issue tracking: [Pumpkin-MC Issues](https://github.com/Pumpkin-MC/Pumpkin/issues)

## Features

- [x] Configuration (toml)
- [Tracking: Protocol](https://github.com/Pumpkin-MC/Pumpkin/issues/1401)
  - [x] Server Status/Ping & Favicon
  - [x] Encryption & Online Authentication
  - [x] Packet Compression
  - [x] Java Edition 26.2
  - [x] Bedrock Edition (W.I.P)
  - ...
- [Tracking: World](https://github.com/Pumpkin-MC/Pumpkin/issues/1403)
  - [x] Player Tab-list
  - [x] Scoreboard
  - [x] World Loading & Dimensions
  - [x] World Time & Weather
  - [x] World Borders
  - [x] World Saving
  - [x] Lighting Engine
  - [x] Entity Spawning & Nether Portal Lifecycle
  - [x] Bossbar
  - [x] Chunk Loading (Vanilla, Linear, Pump)
  - [x] Chunk Saving (Vanilla, Linear, Pump)
  - [x] Redstone & Block Updates
  - [x] Liquid Physics
  - ...
- [Tracking: Player](https://github.com/Pumpkin-MC/Pumpkin/issues/1405)
  - [x] Skins & Player Profiles
  - [x] Teleportation & Dimension Transitions
  - [x] Movement & Physics
  - [x] Animation & Combat
  - [x] Inventory & Crafting
  - [x] Experience & Hunger
  - [x] Off Hand
  - [x] Advancements (W.I.P)
  - ...
- Entities
  - [x] Non-Living (Minecart, Projectiles, Items, TNT...)
  - [x] Entity Effects & Statuses
  - [x] Players & Spectators
  - [x] Mobs & Animals
  - [x] Entity AI & Pathfinding
  - [x] Villagers & Trading
  - [x] Entity Persistence & Saving
- Server
  - [x] Plugins (WASM & Native)
  - [x] Query & RCON
  - [x] Particle & Sound Effects
  - [x] Chat & Commands
  - [x] Permissions & Translations
- Proxy Support
  - [x] [BungeeCord](https://github.com/SpigotMC/BungeeCord)
  - [x] [BungeeGuard](https://github.com/lucko/BungeeGuard)
  - [x] [Velocity](https://github.com/PaperMC/Velocity)

## How to Run

### Building from Source

```bash
# Debug build (fast compile)
cargo build

# Release build (optimized for potato PCs)
cargo build --release

# Run
./target/release/pumpkin
```

On first start, the server will generate the default `configuration/` files and load `server-icon.png`.

## Development & AI Guidelines

For architectural rules, parity standards, and AI agent instructions, consult [`instruction.md`](./instruction.md) at the repository root.

## Upstream Links & Attribution

PotatoMC is built on top of the incredible work by the PumpkinMC team and contributors:
- **Upstream Repository**: <https://github.com/Pumpkin-MC/Pumpkin>
- **Upstream Documentation**: <https://docs.pumpkinmc.org/>
- **Upstream Community Discord**: <https://discord.gg/wT8XjrjKkf>
- **Support PumpkinMC**: Consider donating at <https://pumpkinmc.org/donate/>

## License

* **PotatoMC Server**: Licensed under the [GNU General Public License v3.0 (GPLv3)](LICENSE), preserving all original copyright notices from PumpkinMC and its contributors.
* **Plugin API (`pumpkin-plugin-api` & `pumpkin-plugin-wit`)**: Dual-licensed under [MIT](crates/pumpkin-plugin-api/LICENSE-MIT) OR [Apache-2.0](crates/pumpkin-plugin-api/LICENSE-APACHE).
* **Third-Party Assets & Data**: Subject to their respective licenses and attribution terms. See [assets/NOTICE.md](assets/NOTICE.md).
