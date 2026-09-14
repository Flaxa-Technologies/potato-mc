use std::time::Duration;
use potato_api::bossbar::{BossBar, BossBarColor, BossBarStyle};
use potato_api::command::{Argument, Command, CommandContext, CommandResult, CommandSender};
use potato_api::event::{BlockBreakEvent, Cancellable, PlayerInteractEvent, PlayerJoinEvent};
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::potato_plugin;
use potato_api::text::{Component, NamedTextColor};

#[derive(Default)]
pub struct TestPlugin;

impl Plugin for TestPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "test-plugin".to_string(),
            version: "0.2.0".to_string(),
            authors: vec!["PotatoMC Team".to_string()],
            description: "Showcase plugin demonstrating Paper-grade PotatoMC Native Rust Plugin API".to_string(),
            dependencies: Vec::new(),
        }
    }

    fn on_load(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("TestPlugin loaded into PotatoMC runtime!");

        // 1. Initialize Default YAML Configuration (Bukkit/Paper FileConfiguration equivalent)
        let default_config = r#"# PotatoMC Test Plugin Configuration
welcome_message: "<green><bold>Welcome to PotatoMC, {player}!</bold></green> Powered by Potato Native Plugin API."
tab_header: "<gold><bold>PotatoMC Server</bold></gold>"
tab_footer: "<gray>Paper-grade Rust Native API</gray>"
features:
  custom_join: true
  bossbar_demo: true
"#;
        let _ = context.save_default_config(default_config);
        Ok(())
    }

    fn on_enable(&self, context: &PluginContext) -> Result<(), String> {
        let logger = context.logger();
        logger.info("TestPlugin enabling Paper-grade API features...");

        // Load configuration
        let config = context.config();
        let welcome_template = config.get_string_or(
            "welcome_message",
            "<green>Welcome to PotatoMC, {player}!</green>",
        );
        let tab_header = config.get_string_or("tab_header", "PotatoMC");
        let tab_footer = config.get_string_or("tab_footer", "Powered by Rust");

        // 2. Event Listeners
        // 2a. PlayerJoinEvent: Adventure rich text, MiniMessage, and Tab list header/footer
        let join_logger = context.logger().clone();
        let join_template = welcome_template.clone();
        let header_text = tab_header.clone();
        let footer_text = tab_footer.clone();

        context.register_event(move |event: &mut PlayerJoinEvent| {
            let player_name = event.player.name();
            join_logger.info(format!("[TestPlugin] Player '{}' joined the server!", player_name));

            // Format welcome message with MiniMessage
            let formatted = join_template.replace("{player}", &player_name);
            let comp = Component::from_mini_message(&formatted);
            event.player.send_component(&comp);

            // Set player tab list header and footer (Paper Player#setPlayerListHeaderFooter)
            let h_comp = Component::from_mini_message(&header_text);
            let f_comp = Component::from_mini_message(&footer_text);
            event.player.set_player_list_header_footer(&h_comp.to_legacy_string(), &f_comp.to_legacy_string());
        });

        // 2b. BlockBreakEvent: Block logging and Cancellable demonstration
        let break_logger = context.logger().clone();
        context.register_event(move |event: &mut BlockBreakEvent| {
            let breaker = event.player.as_ref().map_or_else(|| "Unknown".to_string(), |p| p.name());
            break_logger.info(format!(
                "[TestPlugin] Block '{}' at ({:.1}, {:.1}, {:.1}) broken by {}",
                event.block.block_type, event.location.x, event.location.y, event.location.z, breaker
            ));

            // Example of cancellation: protect bedrock
            if event.block.block_type == "minecraft:bedrock" {
                event.set_cancelled(true);
                if let Some(ref player) = event.player {
                    player.send_message("§cYou cannot break bedrock!");
                }
            }
        });

        // 2c. PlayerInteractEvent
        let interact_logger = context.logger().clone();
        context.register_event(move |event: &mut PlayerInteractEvent| {
            interact_logger.debug(format!(
                "[TestPlugin] Player '{}' triggered interact action {:?}",
                event.player.name(),
                event.action
            ));
        });

        // 3. Brigadier-Style Commands
        let cmd = Command::tree("potato")
            .description("Main PotatoMC test plugin command")
            .alias("potatocmd")
            .subcommand(
                Command::tree("hello")
                    .description("Say hello with Adventure rich text")
                    .argument(Argument::word("name"))
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        let name = ctx.get_string("name").unwrap_or("World");
                        let message = Component::text("[PotatoMC] ")
                            .color(NamedTextColor::Gold)
                            .bold()
                            .append(Component::text(format!("Hello, {name}!"))
                            .color(NamedTextColor::Green));

                        match ctx.sender() {
                            CommandSender::Player(p) => p.send_component(&message),
                            CommandSender::Console(c) => c.send_message(&message.to_plain_text()),
                        }
                        Ok(())
                    }),
            )
            .subcommand(
                Command::tree("status")
                    .description("Check plugin and server status")
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        let msg = Component::from_mini_message(
                            "<gold>[PotatoMC Status]</gold> <green>OK!</green> Native cdylib active, Paper-grade API online.",
                        );
                        match ctx.sender() {
                            CommandSender::Player(p) => p.send_component(&msg),
                            CommandSender::Console(c) => c.send_message(&msg.to_plain_text()),
                        }
                        Ok(())
                    }),
            )
            .subcommand(
                Command::tree("ping")
                    .description("Ping command returning pong")
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        match ctx.sender() {
                            CommandSender::Player(p) => p.send_message("§a[PotatoMC] Pong!"),
                            CommandSender::Console(c) => c.send_message("[PotatoMC] Pong!"),
                        }
                        Ok(())
                    }),
            )
            .subcommand(
                Command::tree("tab")
                    .description("Updates player list header & footer")
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        if let CommandSender::Player(p) = ctx.sender() {
                            p.set_player_list_header_footer("§6§lPotatoMC Network", "§aLatency: Good §7| §bPaper API");
                            p.send_message("§aUpdated your player tab list header & footer!");
                        } else {
                            ctx.sender().send_message("This command can only be executed by players.");
                        }
                        Ok(())
                    }),
            )
            .subcommand(
                Command::tree("bossbar")
                    .description("Creates a demonstration BossBar")
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        let bar = BossBar::new(
                            "§d§lEnder Dragon §7(Boss)",
                            BossBarColor::Purple,
                            BossBarStyle::Notched10,
                        ).with_progress(0.85);

                        let info = format!("§aBossBar instantiated: title='{}', progress={:.0}%, color={:?}",
                            bar.title, bar.progress * 100.0, bar.color);
                        ctx.sender().send_message(&info);
                        Ok(())
                    }),
            )
            .subcommand(
                Command::tree("broadcast")
                    .description("Broadcast a message formatted with MiniMessage")
                    .argument(Argument::greedy_string("message"))
                    .executes(|ctx: &CommandContext| -> CommandResult {
                        let raw_msg = ctx.get_string("message").unwrap_or("Hello PotatoMC!");
                        let comp = Component::from_mini_message(raw_msg);
                        let formatted = comp.to_legacy_string();
                        match ctx.sender() {
                            CommandSender::Player(p) => {
                                p.send_message(&format!("§7[Broadcast] {}", formatted));
                            }
                            CommandSender::Console(c) => {
                                c.send_message(&format!("[Broadcast] {}", comp.to_plain_text()));
                            }
                        }
                        Ok(())
                    }),
            )
            .executes(|ctx: &CommandContext| -> CommandResult {
                let usage = "§eUsage: /potato <hello [name] | status | ping | tab | bossbar | broadcast <msg>>";
                ctx.sender().send_message(usage);
                Ok(())
            });

        context.register_command(cmd);

        // 4. Scheduler Tasks (Task concurrency)
        let delayed_logger = context.logger().clone();
        context.scheduler().run_task_later(Duration::from_millis(1500), move || {
            delayed_logger.info("[TestPlugin Scheduler] 1.5-second delayed initialization task completed!");
        });

        let heartbeat_logger = context.logger().clone();
        let _heartbeat_handle = context.scheduler().run_task_repeating(
            Duration::from_millis(10000),
            Duration::from_millis(10000),
            move || {
                heartbeat_logger.info("[TestPlugin Heartbeat] Native tick heartbeat active (10s interval).");
            },
        );

        logger.info("TestPlugin enabled successfully with Paper-grade APIs!");
        Ok(())
    }

    fn on_disable(&self, context: &PluginContext) -> Result<(), String> {
        context.logger().info("TestPlugin disabled. Cleaned up tasks and event listeners.");
        Ok(())
    }
}

// Export the native entrypoint symbol
potato_plugin!(TestPlugin);
