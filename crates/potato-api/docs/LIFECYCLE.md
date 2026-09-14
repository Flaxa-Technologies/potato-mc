# Plugin Lifecycle & Architecture

PotatoMC native plugins are compiled directly to native dynamic libraries (`.dll` on Windows, `.so` on Linux, `.dylib` on macOS). They run inside the same memory space as the server engine, offering zero garbage-collection pauses, instant packet manipulation, and direct memory access.

---

## 1. The `Plugin` Trait

Every PotatoMC plugin is represented by a struct implementing the `Plugin` trait and deriving `Default`.

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::potato_plugin;

#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("MyPlugin", "1.0.0")
            .author("Developer")
            .description("Demonstrates the PotatoMC plugin lifecycle")
    }

    fn on_load(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin loaded!");
        // Perform pre-world initialization, default config generation, etc.
        Ok(())
    }

    fn on_enable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin enabled!");
        // Register events, commands, schedulers
        Ok(())
    }

    fn on_disable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("MyPlugin disabled.");
        // Close sockets, flush caches, clean up external resources
        Ok(())
    }
}

// Required: Exports the C-ABI initialization symbol `potato_create_plugin`
potato_plugin!(MyPlugin);
```

---

## 2. Lifecycle Stages

```
                ┌────────────────────────┐
                │  Dynamic Library (.so) │
                │   Discovered & Loaded  │
                └───────────┬────────────┘
                            │
                            ▼
                ┌────────────────────────┐
                │       on_load()        │  <- Config setup, data migrations
                └───────────┬────────────┘
                            │
                            ▼
                ┌────────────────────────┐
                │      on_enable()       │  <- Register events, commands, timers
                └───────────┬────────────┘
                            │
                      (Server Running)
                            │
                            ▼
                ┌────────────────────────┐
                │      on_disable()      │  <- Flush DB, cancel workers, cleanup
                └────────────────────────┘
```

### `on_load(&self, context: &PluginContext) -> Result<(), String>`
- Invoked immediately when the server loads the dynamic library.
- Used to verify configuration files, establish database pools, and prepare static data before the world is generated or loaded.

### `on_enable(&self, context: &PluginContext) -> Result<(), String>`
- Invoked when the server runtime is fully initialized and ready to accept gameplay logic.
- Register all your event listeners, Brigadier commands, and scheduler tasks here.

### `on_disable(&self, context: &PluginContext) -> Result<(), String>`
- Invoked during server shutdown or when plugins are reloaded.
- Event listeners and scheduler tasks belonging to this plugin are automatically cancelled by PotatoMC, but plugins should close any external network connections, database handles, or open file descriptors here.

---

## 3. The `potato_plugin!` Macro

The `potato_plugin!(StructName)` macro generates the `extern "C"` entrypoint function expected by the PotatoMC native loader:

```rust
// Internally expands to:
#[no_mangle]
pub static POTATO_API_VERSION: u32 = 1;

#[no_mangle]
pub extern "C" fn potato_create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(MyPlugin::default());
    Box::into_raw(plugin)
}
```
This guarantees binary compatibility and version negotiation across host and plugin binaries.
