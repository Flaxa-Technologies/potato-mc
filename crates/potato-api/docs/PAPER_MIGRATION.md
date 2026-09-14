# Migrating from Paper (Java) to PotatoMC (Rust)

A quick guide for Java Bukkit/Spigot/Paper developers transitioning to PotatoMC native plugins.

---

## 1. Concept Translation Table

| Concept | Paper (Java) | PotatoMC (Rust) |
|---|---|---|
| **Plugin Base** | `public class MyPlugin extends JavaPlugin` | `pub struct MyPlugin; impl Plugin for MyPlugin` |
| **Descriptor** | `plugin.yml` / `paper-plugin.yml` | `PluginMetadata::new("Name", "1.0")` |
| **Output File** | `MyPlugin.jar` | `my_plugin.dll` (Win), `libmy_plugin.so` (Linux), `libmy_plugin.dylib` (macOS) |
| **Event Listener** | `@EventHandler public void onJoin(PlayerJoinEvent e)` | `context.register_event(\|e: &mut PlayerJoinEvent\| { ... })` |
| **Cancel Event** | `event.setCancelled(true)` | `event.set_cancelled(true)` |
| **Commands** | `getCommand("hello").setExecutor(...)` | `context.register_command(Command::tree("hello").executes(...))` |
| **Rich Chat** | `Component.text("Hi", NamedTextColor.RED)` | `Component::text("Hi").color(NamedTextColor::Red)` |
| **MiniMessage** | `MiniMessage.miniMessage().deserialize("<red>Hi</red>")` | `Component::from_mini_message("<red>Hi</red>")` |
| **Tab List** | `player.setPlayerListHeaderFooter(h, f)` | `player.set_player_list_header_footer(&h, &f)` |
| **Scheduler** | `Bukkit.getScheduler().runTaskLater(...)` | `context.scheduler().run_task_later(...)` |
| **Configuration** | `getConfig().getString("path")` | `context.config().get_string_or("path", "fallback")` |
| **Item Meta & PDC**| `meta.getPersistentDataContainer()` | `item.pdc().get_string("key")` |

---

## 2. Side-by-Side Example

### Paper (Java):
```java
public class MyPlugin extends JavaPlugin implements Listener {
    @Override
    public void onEnable() {
        getServer().getPluginManager().registerEvents(this, this);
        saveDefaultConfig();
    }

    @EventHandler(priority = EventPriority.HIGH)
    public void onBlockBreak(BlockBreakEvent event) {
        if (event.getBlock().getType() == Material.BEDROCK) {
            event.setCancelled(true);
            event.getPlayer().sendMessage(Component.text("Cannot break bedrock!", NamedTextColor.RED));
        }
    }
}
```

### PotatoMC (Rust):
```rust
#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("MyPlugin", "1.0.0")
    }

    fn on_enable(&self, context: &PluginContext) -> Result<(), String> {
        context.register_event(|event: &mut BlockBreakEvent| {
            if event.block.block_type == "minecraft:bedrock" {
                event.set_cancelled(true);
                if let Some(ref player) = event.player {
                    player.send_message("§cCannot break bedrock!");
                }
            }
        });
        Ok(())
    }
}

potato_plugin!(MyPlugin);
```
