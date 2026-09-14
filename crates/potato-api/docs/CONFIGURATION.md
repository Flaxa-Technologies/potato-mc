# Configuration API (YAML & Dot-Notation)

PotatoMC plugins use standard YAML configuration files (`config.yml`), offering a high-performance configuration engine with Bukkit/Paper dot-notation support.

---

## 1. Saving Default Configuration

In your plugin's `on_load()` or `on_enable()`, bundle and write your default configuration:

```rust
let default_config = r#"# MyPlugin Configuration
server:
  welcome_message: "<gradient:#ffaa00:#ff5555><bold>Welcome, {player}!</bold></gradient>"
  max_homes: 5
features:
  pvp_enabled: true
  spawn_radius: 16.5
"#;

// Writes config.yml to plugins/MyPlugin/config.yml only if it doesn't already exist
context.save_default_config(default_config)?;
```

---

## 2. Reading Configuration Values

Read values safely using dot-notation:

```rust
let config = context.config();

// Strings with default fallbacks
let welcome = config.get_string_or("server.welcome_message", "Welcome!");

// Integers
let max_homes = config.get_int_or("server.max_homes", 3);

// Booleans
let pvp = config.get_bool_or("features.pvp_enabled", true);

// Floats
let radius = config.get_float_or("features.spawn_radius", 10.0);
```

---

## 3. Modifying & Saving at Runtime

```rust
let mut config = context.config();

// Update a value
config.set("server.max_homes", 10);

// Save back to plugins/MyPlugin/config.yml
context.save_config(&config)?;
```
