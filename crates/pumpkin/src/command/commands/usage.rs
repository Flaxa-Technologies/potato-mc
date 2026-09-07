use std::sync::atomic::Ordering;

use pumpkin_util::{
    PermissionLvl,
    permission::{Permission, PermissionDefault, PermissionRegistry},
    text::{TextComponent, color::NamedColor},
};
use sysinfo::System;

use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

const DESCRIPTION: &str = "Displays server resource usage: CPU, RAM, TPS, MSPT, players, entities, chunks, and uptime.";
const PERMISSION: &str = "pumpkin:command.usage";

struct UsageExecutor;

impl CommandExecutor for UsageExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let server = context.source.server();

        // --- Performance metrics ---
        let max_tps = server.basic_config.tps as f64;
        let tps = server.get_tps().min(max_tps);
        let mspt = server.get_mspt();
        let tick_count = server.tick_count.load(Ordering::Relaxed);

        // Uptime from tick count
        let uptime_secs = if max_tps > 0.0 {
            (tick_count as f64 / max_tps) as u64
        } else {
            0
        };
        let days    = uptime_secs / 86400;
        let hours   = (uptime_secs % 86400) / 3600;
        let minutes = (uptime_secs % 3600) / 60;
        let seconds = uptime_secs % 60;
        let uptime_str = if days > 0 {
            format!("{days}d {hours:02}h {minutes:02}m {seconds:02}s")
        } else if hours > 0 {
            format!("{hours}h {minutes:02}m {seconds:02}s")
        } else {
            format!("{minutes}m {seconds:02}s")
        };

        // --- World stats ---
        let worlds = server.worlds.load();
        let player_count = worlds.iter().map(|w| w.players.load().len()).sum::<usize>();
        let entity_count = worlds.iter().map(|w| w.entities.load().len()).sum::<usize>();
        let chunk_count  = worlds.iter().map(|w| w.level.loaded_chunks.len()).sum::<usize>();

        // --- System stats via sysinfo ---
        let (proc_mb, sys_used_mb, sys_total_mb, cpu_count) = if sysinfo::IS_SUPPORTED_SYSTEM {
            let mut sys = System::new_all();
            sys.refresh_all();
            let proc_mb = sysinfo::get_current_pid()
                .ok()
                .and_then(|pid| sys.process(pid).map(|p| p.memory() / (1024 * 1024)))
                .unwrap_or(0);
            let sys_used_mb  = sys.used_memory() / (1024 * 1024);
            let sys_total_mb = sys.total_memory() / (1024 * 1024);
            let cpu_count = sys.cpus().len();
            (proc_mb, sys_used_mb, sys_total_mb, cpu_count)
        } else {
            (0, 0, 0, 0)
        };

        // --- TPS color ---
        let tps_color = if tps >= max_tps * 0.9 {
            NamedColor::Green
        } else if tps >= max_tps * 0.75 {
            NamedColor::Yellow
        } else {
            NamedColor::Red
        };

        let send = |msg: TextComponent| {
            if let Some(player) = context.source.as_player() {
                player.send_system_message(&msg);
            } else {
                context.source.send_message(msg);
            }
        };

        send(TextComponent::text("=== Server Usage ===").color_named(NamedColor::Gold));
        send(TextComponent::text("TPS: ")
            .add_child(TextComponent::text(format!("{tps:.1}")).color_named(tps_color))
            .add_child(TextComponent::text(format!(" / {max_tps:.0}  MSPT: ")))
            .add_child(TextComponent::text(format!("{mspt:.2}ms")).color_named(tps_color)));
        send(TextComponent::text("RAM: ")
            .add_child(TextComponent::text(format!("{proc_mb} MB (Process)")).color_named(NamedColor::Aqua))
            .add_child(TextComponent::text(format!("  |  Host: {sys_used_mb} MB / {sys_total_mb} MB")).color_named(NamedColor::Gray)));
        send(TextComponent::text("Logical CPUs: ")
            .add_child(TextComponent::text(format!("{cpu_count}")).color_named(NamedColor::White)));
        send(TextComponent::text("Players: ")
            .add_child(TextComponent::text(format!("{player_count}")).color_named(NamedColor::White)));
        send(TextComponent::text("Entities: ")
            .add_child(TextComponent::text(format!("{entity_count}")).color_named(NamedColor::White)));
        send(TextComponent::text("Loaded Chunks: ")
            .add_child(TextComponent::text(format!("{chunk_count}")).color_named(NamedColor::White)));
        send(TextComponent::text("Uptime: ")
            .add_child(TextComponent::text(uptime_str).color_named(NamedColor::White)));

        Ok(0)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("usage", DESCRIPTION)
            .executes(UsageExecutor),
    );
}
