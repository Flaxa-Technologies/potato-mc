# Brigadier Command API

PotatoMC features a native implementation of Mojang's **Brigadier** command tree architecture, providing type-safe arguments, nested subcommands, and tab-completion suggestions.

---

## 1. Creating Commands

Commands are constructed using fluent builder trees with `Command::tree("name")`:

```rust
use potato_api::command::{Command, CommandContext, CommandResult, CommandSender, Argument};
use potato_api::text::Component;

let cmd = Command::tree("spawn")
    .description("Teleport to the server spawn point")
    .alias("hub")
    .executes(|ctx: &CommandContext| -> CommandResult {
        match ctx.sender() {
            CommandSender::Player(player) => {
                player.send_message("§aTeleporting to spawn...");
                // player.teleport(spawn_location);
            }
            CommandSender::Console(console) => {
                console.send_message("Only players can teleport to spawn!");
            }
        }
        Ok(())
    });

context.register_command(cmd);
```

---

## 2. Arguments & Types

Arguments are declared with `Argument::<type>("name")`:

| Method | Description | Example |
|---|---|---|
| `Argument::word("name")` | Single word without spaces | `target` |
| `Argument::string("name")` | Full string / sentence | `message` |
| `Argument::integer("amount")` | 32-bit integer | `count` |
| `Argument::int_range("lvl", min, max)` | Integer bounded between min and max | `level` (1..100) |
| `Argument::float("rate")` | 32-bit floating point number | `speed` |
| `Argument::boolean("flag")` | `true` or `false` | `enable` |
| `Argument::player("target")` | Online player username resolution | `player` |

### Retrieving Parsed Arguments in Handlers:
```rust
let sub = Command::tree("give")
    .argument(Argument::word("item"))
    .argument(Argument::integer("amount"))
    .executes(|ctx: &CommandContext| -> CommandResult {
        let item_name = ctx.get_string("item").unwrap_or("apple");
        let amount = ctx.get_int("amount").unwrap_or(1);

        ctx.sender().send_message(&format!("§aGave {}x {}", amount, item_name));
        Ok(())
    });
```

---

## 3. Subcommands & Nesting

Build complex hierarchies by attaching subcommands with `.subcommand(...)`:

```rust
let admin_cmd = Command::tree("admin")
    .description("Administrative controls")
    .subcommand(
        Command::tree("kick")
            .argument(Argument::word("target"))
            .argument(Argument::string("reason"))
            .executes(|ctx: &CommandContext| -> CommandResult {
                let target = ctx.get_string("target")?;
                let reason = ctx.get_string("reason").unwrap_or("Kicked by admin");
                // Kick logic here
                Ok(())
            })
    )
    .subcommand(
        Command::tree("broadcast")
            .argument(Argument::string("message"))
            .executes(|ctx: &CommandContext| -> CommandResult {
                let msg = ctx.get_string("message")?;
                let comp = Component::from_mini_message(&format!("<red>[ALERT]</red> <white>{}</white>", msg));
                // Broadcast logic
                Ok(())
            })
    );

context.register_command(admin_cmd);
```
