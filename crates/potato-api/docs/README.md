# PotatoMC Native Plugin API Documentation

<p align="center">
  <img src="../assets/LOGO.png" alt="PotatoMC Logo" width="220"/>
</p>

<p align="center">
  <b>Paper-Grade Native Rust Plugin Development Kit for PotatoMC</b><br>
  Zero JVM Overhead • Sub-Millisecond Latency • Expressive Developer Ergonomics
</p>

<p align="center">
  <a href="https://potatomc.flaxa.in/">🌐 Website</a> •
  <a href="https://discord.com/invite/UUaNzfZyc6">💬 Discord (Flaxa Studios)</a> •
  <a href="https://github.com/Flaxa-Technologies/potato-api">📦 GitHub Repository</a>
</p>

---

## Documentation Index

Welcome to the complete developer documentation for PotatoMC native plugins. Explore the guides below:

1. **[Plugin Lifecycle & Architecture](LIFECYCLE.md)**
   - Plugin trait (`on_load`, `on_enable`, `on_disable`)
   - Plugin metadata declaration
   - Exporting the native dynamic library entrypoint (`potato_plugin!`)
   - Windows `.dll`, Linux `.so`, macOS `.dylib` binaries

2. **[Event System & Cancellable](EVENTS.md)**
   - Publisher / Observer pattern
   - Complete 59-event catalog (IDs 1–59)
   - Block break loot suppression via `set_drop_items(false)`
   - `Cancellable` trait & stopping vanilla actions
   - `EventPriority` ordering (`Lowest` to `Monitor`)

3. **[Virtual GUI & Inventory API](GUI_AND_INVENTORY.md)**
   - Fluent chest GUI builder (`Gui::chest(title, rows)`)
   - Hopper (5 slots) and Dispenser/Dropper (9 slots) virtual layouts
   - Slot mapping, custom borders, and interaction flags (`allow_grab`, `allow_put`)
   - `player.open_gui(&gui)` and `player.close_inventory()`
   - Event handling with `InventoryClickEvent`
   - Giving and dropping items (`player.give_item`, `player.drop_item`, `world.drop_item`)

4. **[Dialog Box, Forms & Virtual Book API](DIALOGS_AND_FORMS.md)**
   - Java 1.21.4+ native Dialog Box builder (`Dialog`)
   - Bedrock crossplay form dialogs (`SimpleForm`, `ModalForm`, `CustomForm`)
   - Virtual Book GUI (`Book`) without item in hand
   - Interactive Sign editor dialog
   - Dialog click and response events (`DialogClickActionEvent`, `PlayerFormResponseEvent`)

5. **[Scoreboards & Teams API](SCOREBOARDS_AND_TEAMS.md)**
   - Fast sidebar scoreboards (`set_sidebar_lines`)
   - Scoreboard builder (`Scoreboard::sidebar`, `set_line`)
   - DisplaySlots (`Sidebar`, `BelowName`, `List`)
   - Team prefix, suffix, color, and friendly fire (`Team`)

6. **[Sounds & Environmental World API](SOUNDS_AND_ENVIRONMENT.md)**
   - Sound effect playback with `SoundCategory`
   - Sound stopping (`stop_sound`)
   - Real and cosmetic lightning strikes (`strike_lightning`, `strike_lightning_effect`)
   - World weather, thunder, and highest block queries

7. **[Potions & Particle Effects](POTIONS_AND_PARTICLES.md)**
   - Fluent `PotionEffect` builder
   - Managing player status effects (`add_potion_effect`, `has_potion_effect`, `clear_potion_effects`)
   - Particle effect broadcasts (`world.spawn_particle`, `player.spawn_particle`)

8. **[Brigadier Command API](COMMANDS.md)**
   - Tree-based fluent command builders (`Command::tree`)
   - Subcommands and argument parsing (`word`, `string`, `int`, `float`, `bool`, `player`)
   - `CommandSender` (Player vs Console handling)
   - Dynamic tab suggestions

9. **[Adventure Text & MiniMessage](TEXT_AND_MINIMESSAGE.md)**
   - Adventure `Component` architecture
   - MiniMessage tag syntax (`<gradient>`, `<bold>`, hex colors)
   - Action bars & animated titles
   - Custom Tab List Header & Footer (`Player#set_player_list_header_footer`)

10. **[Persistent Data Container (PDC) & Items](PDC_AND_ITEMS.md)**
    - Attaching arbitrary typed metadata to items (`strings`, `ints`, `longs`, `bytes`)
    - `ItemStack` builder, lore, and custom names
    - Custom enchantments (`add_enchantment`, `get_enchantment_level`)
    - Item attack cooldown tracking (`set_item_cooldown`, `get_item_cooldown`)

11. **[Scheduler & Concurrency](SCHEDULER.md)**
    - Main-thread tick synchronization
    - Delayed tasks (`run_task_later`)
    - Repeating interval tasks (`run_task_repeating`)
    - Asynchronous off-thread background worker threads

12. **[Configuration API](CONFIGURATION.md)**
    - YAML configuration file loader (`config.yml`)
    - Dot-notation queries (`server.motd`, `features.pvp`)
    - Type-safe getters with fallbacks (`get_string_or`, `get_int_or`, `get_bool_or`)
    - Bundling default configuration files

13. **[BossBar API](BOSSBAR.md)**
    - Creating Adventure-compatible BossBars
    - Colors and division overlay styles
    - Real-time progress updates

14. **[Line-of-Sight Raytracing & Targeting API](RAYTRACING_AND_TARGETING.md)**
    - Voxel fast-traversal raymarching (`player.get_target_block(max_distance)`)
    - Axis-Aligned Bounding Box (AABB) intersection tests (`BoundingBox`)
    - Entity crosshair targeting (`player.get_target_entity(max_distance)`)
    - Vector math library (`Vector3` dot, cross, normalize, angles)

15. **[Toasts, Game Events & Demo Screens](TOASTS_AND_GAME_EVENTS.md)**
    - Advancements UI toast notifications (`player.send_toast(...)`, `ToastFrame`)
    - Direct client Game Events (`send_game_event`, rain, elder guardian, credits)
    - Interactive Demo reminder screen (`player.send_demo_screen()`)

16. **[Advanced Entity Flags, Tags & Manipulation](ADVANCED_ENTITIES.md)**
    - Entity metadata flags (`set_glowing`, `set_invulnerable`, `set_silent`, `set_gravity`)
    - Scoreboard tags (`add_scoreboard_tag`, `remove_scoreboard_tag`, `scoreboard_tags`)
    - Precise eye height and eye location calculation

17. **[NPC & Fake Player API](NPC_API.md)**
    - Spawning fake player and mob NPCs (`Npc::builder`)
    - Controlling skin textures, poses, and equipment
    - Click interaction handling (`NpcInteractEvent`)
    - Waypoint pathing navigation (`npc.move_to`)
    - Pluggable behavior traits (`NpcBehavior`) without touching core engine code

18. **[Custom Model Data & Item Variants](CUSTOM_MODEL_DATA.md)**
    - Direct CustomModelData getters & setters on `ItemStack`
    - Declarative resource-pack-driven item templates (`ItemVariant`)
    - Central catalog and matching without raw NBT (`ItemVariantRegistry`)

19. **[Paper to PotatoMC Migration Guide](PAPER_MIGRATION.md)**
    - Side-by-side Java vs. Rust code translations
    - Differences in threading and memory models

---

## ⚡ Instant Setup Links

Install the development environment directly from your terminal:

- **Linux**:
  ```bash
  curl -sSL https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-linux.sh | bash
  ```
- **macOS**:
  ```bash
  curl -sSL https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-macos.sh | bash
  ```
- **Windows (PowerShell)**:
  ```powershell
  irm https://raw.githubusercontent.com/Flaxa-Technologies/potato-api/main/scripts/setup-windows.ps1 | iex
  ```

---

## 💬 API Suggestions & Reporting Issues

- **API Suggestions & New Features**:
  Join our Discord: [**Flaxa Studios Discord**](https://discord.com/invite/UUaNzfZyc6) in the `#api-suggestions` or `#plugin-development` channels!
- **Bug Reports & Issues**:
  Report on Discord in `#bugs-support` or open an issue on [**GitHub Issues**](https://github.com/Flaxa-Technologies/potato-api/issues).
- **Official Website**: [**https://potatomc.flaxa.in/**](https://potatomc.flaxa.in/)
