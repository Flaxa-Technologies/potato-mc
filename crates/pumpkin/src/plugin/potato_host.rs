use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

use uuid::Uuid;

use crate::command::argument_builder::{argument, command, literal, ArgumentBuilder};
use crate::command::argument_types::core::bool::BoolArgumentType;
use crate::command::argument_types::core::float::FloatArgumentType;
use crate::command::argument_types::core::integer::IntegerArgumentType;
use crate::command::argument_types::core::string::StringArgumentType;
use crate::command::context::command_context::CommandContext as PumpkinCommandContext;
use crate::command::node::detached::CommandDetachedNode;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::entity::player::Player as PumpkinPlayer;
use crate::net::ClientPlatform;
use crate::plugin::api::events::Payload;
use crate::plugin::api::events::block::block_break::BlockBreakEvent as PumpkinBlockBreakEvent;
use crate::plugin::api::events::block::block_place::BlockPlaceEvent as PumpkinBlockPlaceEvent;
use crate::plugin::api::events::player::inventory_interact::InventoryClickEvent as PumpkinInventoryClickEvent;
use crate::plugin::api::events::player::player_chat::PlayerChatEvent as PumpkinPlayerChatEvent;
use crate::plugin::api::events::player::player_command_preprocess::PlayerCommandPreprocessEvent as PumpkinPlayerCommandPreprocessEvent;
use crate::plugin::api::events::player::player_drop_item::PlayerDropItemEvent as PumpkinPlayerDropItemEvent;
use crate::plugin::api::events::player::player_gamemode_change::PlayerGamemodeChangeEvent as PumpkinPlayerGamemodeChangeEvent;
use crate::plugin::api::events::player::player_item_consume::PlayerItemConsumeEvent as PumpkinPlayerItemConsumeEvent;
use crate::plugin::api::events::player::player_join::PlayerJoinEvent as PumpkinPlayerJoinEvent;
use crate::plugin::api::events::player::player_leave::PlayerLeaveEvent as PumpkinPlayerLeaveEvent;
use crate::plugin::api::events::player::player_move::PlayerMoveEvent as PumpkinPlayerMoveEvent;
use crate::plugin::api::events::player::player_respawn::PlayerRespawnEvent as PumpkinPlayerRespawnEvent;
use crate::plugin::api::events::player::player_teleport::PlayerTeleportEvent as PumpkinPlayerTeleportEvent;
use crate::plugin::api::events::player::player_toggle_flight_event::PlayerToggleFlightEvent as PumpkinPlayerToggleFlightEvent;
use crate::plugin::api::events::player::player_toggle_sneak_event::PlayerToggleSneakEvent as PumpkinPlayerToggleSneakEvent;
use crate::plugin::api::events::player::player_toggle_sprint_event::PlayerToggleSprintEvent as PumpkinPlayerToggleSprintEvent;
use crate::plugin::api::events::server::list_ping::ServerListPingEvent as PumpkinServerListPingEvent;
use crate::plugin::api::{Context as PumpkinContext, EventPriority, Plugin as PumpkinPlugin, PluginFuture};
use crate::plugin::{EventHandler, TypedEventHandler};
use crate::server::Server;
use crate::world::World as PumpkinWorld;
use pumpkin_data::data_component::DataComponent;
use pumpkin_data::data_component_impl::basic::CustomModelDataImpl;
use pumpkin_data::data_component_impl::{CustomNameImpl, LoreImpl};
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::item_stack::ItemStack as PumpkinItemStack;
use pumpkin_data::particle::Particle as PumpkinParticle;
use pumpkin_data::potion::Effect as PumpkinEffect;
use pumpkin_data::screen::WindowType;
use pumpkin_inventory::gui_builder::GUIBuilder;
use pumpkin_inventory::player::player_inventory::PlayerInventory;
use pumpkin_inventory::screen_handler::{ClickType, InventoryPlayer, ScreenHandlerFactory, SharedScreenHandler};
use pumpkin_protocol::codec::item_stack_seralizer::ItemStackSerializer;
use pumpkin_protocol::java::client::play::CSetContainerSlot;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::text::TextComponent;
use pumpkin_world::inventory::{Clearable, Inventory};

use potato_api::command::{
    ArgumentType, CommandContext as PotatoCommandContext, CommandNode,
    CommandSender as PotatoCommandSender, ParsedValue,
};
use potato_api::Event;
use potato_api::event::{
    BlockBreakEvent, BlockPlaceEvent, InventoryClickEvent, PlayerChatEvent,
    PlayerCommandPreprocessEvent, PlayerDropItemEvent, PlayerGameModeChangeEvent,
    PlayerItemConsumeEvent, PlayerJoinEvent, PlayerMoveEvent, PlayerQuitEvent,
    PlayerRespawnEvent, PlayerTeleportEvent, PlayerToggleFlightEvent,
    PlayerToggleSneakEvent, PlayerToggleSprintEvent, ServerListPingEvent,
};
use potato_api::host::{
    HostConsole, HostContext, HostEntity, HostLivingEntity, HostPlayer, HostWorld, LogLevel,
    RawEventListener,
};
use potato_api::types::{
    Block as PotatoBlock, Difficulty as PotatoDifficulty, EquipmentSlot as PotatoEquipmentSlot,
    GameMode as PotatoGameMode, HostInventory, ItemStack as PotatoItemStack,
    Location as PotatoLocation, PotionEffect as PotatoPotionEffect, Vector3 as PotatoVector3,
};

pub struct PotatoHostContext {
    pub plugin_name: String,
    pub server: Arc<Server>,
    pub event_listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
    pub task_handles: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
    pub next_task_id: AtomicU64,
    pub data_dir: PathBuf,
}

impl PotatoHostContext {
    pub fn new(plugin_name: String, server: Arc<Server>) -> Self {
        let data_dir = Path::new("plugins").join(&plugin_name);
        if !data_dir.exists() {
            let _ = fs::create_dir_all(&data_dir);
        }
        Self {
            plugin_name,
            server,
            event_listeners: Arc::new(RwLock::new(HashMap::new())),
            task_handles: Arc::new(Mutex::new(HashMap::new())),
            next_task_id: AtomicU64::new(1),
            data_dir,
        }
    }
}

fn to_potato_gamemode(mode: pumpkin_util::GameMode) -> PotatoGameMode {
    match mode {
        pumpkin_util::GameMode::Survival => PotatoGameMode::Survival,
        pumpkin_util::GameMode::Creative => PotatoGameMode::Creative,
        pumpkin_util::GameMode::Adventure => PotatoGameMode::Adventure,
        pumpkin_util::GameMode::Spectator => PotatoGameMode::Spectator,
    }
}

fn from_potato_gamemode(mode: PotatoGameMode) -> pumpkin_util::GameMode {
    match mode {
        PotatoGameMode::Survival => pumpkin_util::GameMode::Survival,
        PotatoGameMode::Creative => pumpkin_util::GameMode::Creative,
        PotatoGameMode::Adventure => pumpkin_util::GameMode::Adventure,
        PotatoGameMode::Spectator => pumpkin_util::GameMode::Spectator,
    }
}

fn to_potato_difficulty(diff: pumpkin_util::Difficulty) -> PotatoDifficulty {
    match diff {
        pumpkin_util::Difficulty::Peaceful => PotatoDifficulty::Peaceful,
        pumpkin_util::Difficulty::Easy => PotatoDifficulty::Easy,
        pumpkin_util::Difficulty::Normal => PotatoDifficulty::Normal,
        pumpkin_util::Difficulty::Hard => PotatoDifficulty::Hard,
    }
}

fn from_potato_difficulty(diff: PotatoDifficulty) -> pumpkin_util::Difficulty {
    match diff {
        PotatoDifficulty::Peaceful => pumpkin_util::Difficulty::Peaceful,
        PotatoDifficulty::Easy => pumpkin_util::Difficulty::Easy,
        PotatoDifficulty::Normal => pumpkin_util::Difficulty::Normal,
        PotatoDifficulty::Hard => pumpkin_util::Difficulty::Hard,
    }
}

fn to_pumpkin_slot(slot: PotatoEquipmentSlot) -> pumpkin_data::data_component_impl::EquipmentSlot {
    match slot {
        PotatoEquipmentSlot::MainHand => pumpkin_data::data_component_impl::EquipmentSlot::MAIN_HAND,
        PotatoEquipmentSlot::OffHand => pumpkin_data::data_component_impl::EquipmentSlot::OFF_HAND,
        PotatoEquipmentSlot::Helmet => pumpkin_data::data_component_impl::EquipmentSlot::HEAD,
        PotatoEquipmentSlot::Chestplate => pumpkin_data::data_component_impl::EquipmentSlot::CHEST,
        PotatoEquipmentSlot::Leggings => pumpkin_data::data_component_impl::EquipmentSlot::LEGS,
        PotatoEquipmentSlot::Boots => pumpkin_data::data_component_impl::EquipmentSlot::FEET,
    }
}

pub fn to_pumpkin_item_stack(it: &PotatoItemStack) -> Option<PumpkinItemStack> {
    let name = it.item_type.strip_prefix("minecraft:").unwrap_or(&it.item_type);
    let item_def = pumpkin_data::item::Item::from_registry_key(name)?;
    let mut stack = PumpkinItemStack::new(it.amount.min(99) as u8, item_def);
    if let Some(custom_name) = &it.custom_name {
        stack.patch.push((
            DataComponent::CustomName,
            Some(Box::new(CustomNameImpl {
                name: TextComponent::text(custom_name.clone()),
            })),
        ));
    }
    if !it.lore.is_empty() {
        let lines = it.lore.iter().map(|l| TextComponent::text(l.clone())).collect();
        stack.patch.push((
            DataComponent::Lore,
            Some(Box::new(LoreImpl { lines })),
        ));
    }
    if let Some(cmd) = it.custom_model_data {
        stack.patch.push((
            DataComponent::CustomModelData,
            Some(Box::new(CustomModelDataImpl {
                floats: vec![cmd as f32],
                flags: Vec::new(),
                strings: Vec::new(),
                colors: Vec::new(),
            })),
        ));
    }
    Some(stack)
}

pub fn to_potato_item_stack(stack: &PumpkinItemStack) -> Option<PotatoItemStack> {
    if stack.is_empty() {
        return None;
    }
    let mut pot = PotatoItemStack::new(stack.item.registry_key.to_string(), stack.item_count as u32);
    for (comp, opt) in &stack.patch {
        if *comp == DataComponent::CustomName {
            if let Some(data) = opt {
                if let Some(name_impl) = data.as_any().downcast_ref::<CustomNameImpl>() {
                    pot.set_custom_name(name_impl.name.clone().to_pretty_console());
                }
            }
        } else if *comp == DataComponent::Lore {
            if let Some(data) = opt {
                if let Some(lore_impl) = data.as_any().downcast_ref::<LoreImpl>() {
                    for line in &lore_impl.lines {
                        pot.add_lore(line.clone().to_pretty_console());
                    }
                }
            }
        } else if *comp == DataComponent::CustomModelData {
            if let Some(data) = opt {
                if let Some(cmd_impl) = data.as_any().downcast_ref::<CustomModelDataImpl>() {
                    if let Some(first) = cmd_impl.floats.first() {
                        pot.set_custom_model_data(Some(*first as i32));
                    }
                }
            }
        }
    }
    Some(pot)
}

pub struct PotatoSimpleInventory {
    pub stacks: RwLock<Vec<PumpkinItemStack>>,
    size: usize,
}

impl PotatoSimpleInventory {
    pub fn new(size: usize) -> Self {
        Self {
            stacks: RwLock::new(vec![PumpkinItemStack::EMPTY.clone(); size]),
            size,
        }
    }
}

impl Clearable for PotatoSimpleInventory {
    fn clear(&self) {
        let mut stacks = self.stacks.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        stacks.fill_with(|| PumpkinItemStack::EMPTY.clone());
    }
}

impl Inventory for PotatoSimpleInventory {
    fn size(&self) -> usize {
        self.size
    }

    fn is_empty(&self) -> bool {
        let stacks = self.stacks.read().unwrap_or_else(std::sync::PoisonError::into_inner);
        stacks.iter().all(PumpkinItemStack::is_empty)
    }

    fn get_stack(&self, slot: usize) -> PumpkinItemStack {
        let stacks = self.stacks.read().unwrap_or_else(std::sync::PoisonError::into_inner);
        stacks.get(slot).cloned().unwrap_or_else(|| PumpkinItemStack::EMPTY.clone())
    }

    fn remove_stack(&self, slot: usize) -> PumpkinItemStack {
        let mut stacks = self.stacks.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < stacks.len() {
            std::mem::replace(&mut stacks[slot], PumpkinItemStack::EMPTY.clone())
        } else {
            PumpkinItemStack::EMPTY.clone()
        }
    }

    fn remove_stack_specific(&self, slot: usize, amount: u8) -> PumpkinItemStack {
        let mut stacks = self.stacks.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < stacks.len() && !stacks[slot].is_empty() && amount > 0 {
            stacks[slot].split(amount)
        } else {
            PumpkinItemStack::EMPTY.clone()
        }
    }

    fn set_stack(&self, slot: usize, stack: PumpkinItemStack) {
        let mut stacks = self.stacks.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < stacks.len() {
            stacks[slot] = stack;
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct PotatoGuiFactory {
    title: String,
    window_type: WindowType,
    inventory: Arc<PotatoSimpleInventory>,
    allow_grab: bool,
    allow_put: bool,
}

impl ScreenHandlerFactory for PotatoGuiFactory {
    fn create_screen_handler(
        &self,
        sync_id: u8,
        player_inventory: &Arc<PlayerInventory>,
        _player: &dyn InventoryPlayer,
    ) -> Option<SharedScreenHandler> {
        let builder = GUIBuilder::new(self.window_type, self.inventory.clone())
            .allow_grab_items(self.allow_grab)
            .allow_put_items(self.allow_put);
        Some(Arc::new(std::sync::Mutex::new(builder.build(sync_id, player_inventory))))
    }

    fn get_display_name(&self) -> TextComponent {
        TextComponent::text(self.title.clone())
    }
}

fn create_argument_builder(
    name: String,
    arg_type: &potato_api::command::ArgumentType,
) -> crate::command::argument_builder::RequiredArgumentBuilder {
    match arg_type {
        ArgumentType::Word => argument(name, StringArgumentType::SingleWord),
        ArgumentType::String => argument(name, StringArgumentType::QuotablePhrase),
        ArgumentType::GreedyString => argument(name, StringArgumentType::GreedyPhrase),
        ArgumentType::Integer { min, max } => argument(
            name,
            IntegerArgumentType {
                min: min.unwrap_or(i32::MIN),
                max: max.unwrap_or(i32::MAX),
            },
        ),
        ArgumentType::Float { min, max } => argument(
            name,
            FloatArgumentType {
                min: min.unwrap_or(f32::MIN),
                max: max.unwrap_or(f32::MAX),
            },
        ),
        ArgumentType::Boolean => argument(name, BoolArgumentType),
        ArgumentType::Player => argument(name, StringArgumentType::SingleWord),
    }
}

fn build_literal_subcommand(
    node: &CommandNode,
    server: &Arc<Server>,
    plugin_name: &str,
) -> crate::command::argument_builder::LiteralArgumentBuilder {
    let mut builder = literal(node.name.clone());

    if let Some(ref perm) = node.permission {
        let full_perm = if perm.contains(':') {
            perm.clone()
        } else {
            format!("{plugin_name}:{perm}")
        };
        builder = builder.requires(full_perm);
    }

    if node.executor.is_some() {
        builder = builder.executes(PotatoCommandBridgeExecutor {
            node: node.clone(),
            server: server.clone(),
        });
    }

    if !node.arguments.is_empty() {
        let mut last_arg: Option<crate::command::argument_builder::RequiredArgumentBuilder> = None;
        for arg in node.arguments.iter().rev() {
            let mut arg_b = create_argument_builder(arg.name.clone(), &arg.arg_type);
            if let Some(next) = last_arg {
                arg_b = arg_b.then(next);
            } else if node.executor.is_some() {
                arg_b = arg_b.executes(PotatoCommandBridgeExecutor {
                    node: node.clone(),
                    server: server.clone(),
                });
            }
            last_arg = Some(arg_b);
        }
        if let Some(first_arg) = last_arg {
            builder = builder.then(first_arg);
        }
    }

    for sub in &node.subcommands {
        let sub_builder = build_literal_subcommand(sub, server, plugin_name);
        builder = builder.then(sub_builder);
    }

    builder
}

impl HostContext for PotatoHostContext {
    fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    fn log(&self, level: LogLevel, message: &str) {
        let prefix = format!("[{}] {}", self.plugin_name, message);
        match level {
            LogLevel::Trace => tracing::trace!(target: "plugin", "{}", prefix),
            LogLevel::Debug => tracing::debug!(target: "plugin", "{}", prefix),
            LogLevel::Info => tracing::info!(target: "plugin", "{}", prefix),
            LogLevel::Warn => tracing::warn!(target: "plugin", "{}", prefix),
            LogLevel::Error => tracing::error!(target: "plugin", "{}", prefix),
        }
    }

    fn run_task(&self, task: Box<dyn FnOnce() + Send>) {
        self.server.spawn_task(async move {
            task();
        });
    }

    fn run_task_later(&self, delay_millis: u64, task: Box<dyn FnOnce() + Send>) -> u64 {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let handles = self.task_handles.clone();
        let handle = self.server.spawn_task(async move {
            tokio::time::sleep(Duration::from_millis(delay_millis)).await;
            task();
            handles.lock().unwrap().remove(&id);
        });
        self.task_handles.lock().unwrap().insert(id, handle);
        id
    }

    fn run_task_repeating(
        &self,
        initial_delay_millis: u64,
        period_millis: u64,
        mut task: Box<dyn FnMut() + Send>,
    ) -> u64 {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let handle = self.server.spawn_task(async move {
            if initial_delay_millis > 0 {
                tokio::time::sleep(Duration::from_millis(initial_delay_millis)).await;
            }
            let mut interval = tokio::time::interval(Duration::from_millis(period_millis.max(1)));
            loop {
                interval.tick().await;
                task();
            }
        });
        self.task_handles.lock().unwrap().insert(id, handle);
        id
    }

    fn cancel_task(&self, task_id: u64) {
        if let Some(handle) = self.task_handles.lock().unwrap().remove(&task_id) {
            handle.abort();
        }
    }

    fn register_event_listener(&self, event_id: u32, listener: Arc<dyn RawEventListener>) {
        self.event_listeners.write().unwrap().entry(event_id).or_default().push(listener);
    }

    fn register_command(&self, command_node: CommandNode) {
        let server = self.server.clone();

        let mut builder = command(
            command_node.name.clone(),
            command_node.description.clone(),
        );

        if let Some(ref perm) = command_node.permission {
            let full_perm = if perm.contains(':') {
                perm.clone()
            } else {
                format!("{}:{perm}", self.plugin_name)
            };
            builder = builder.requires(full_perm);
        }

        if command_node.executor.is_some() {
            builder = builder.executes(PotatoCommandBridgeExecutor {
                node: command_node.clone(),
                server: server.clone(),
            });
        }

        if !command_node.arguments.is_empty() {
            let mut last_arg: Option<crate::command::argument_builder::RequiredArgumentBuilder> = None;
            for arg in command_node.arguments.iter().rev() {
                let mut arg_b = create_argument_builder(arg.name.clone(), &arg.arg_type);
                if let Some(next) = last_arg {
                    arg_b = arg_b.then(next);
                } else if command_node.executor.is_some() {
                    arg_b = arg_b.executes(PotatoCommandBridgeExecutor {
                        node: command_node.clone(),
                        server: server.clone(),
                    });
                }
                last_arg = Some(arg_b);
            }
            if let Some(first_arg) = last_arg {
                builder = builder.then(first_arg);
            }
        }

        for sub in &command_node.subcommands {
            let sub_builder = build_literal_subcommand(sub, &server, &self.plugin_name);
            builder = builder.then(sub_builder);
        }

        let mut node: CommandDetachedNode = builder.into();
        node.meta.source = Some(self.plugin_name.clone());

        let aliases = command_node.aliases.clone();
        server.command_dispatcher.rcu(|dispatcher| {
            let mut new_dispatcher = (**dispatcher).clone();
            if aliases.is_empty() {
                new_dispatcher.register(node.clone());
            } else {
                new_dispatcher.register_with_aliases(node.clone(), &aliases);
            }
            Arc::new(new_dispatcher)
        });

        // Resend command list to connected players
        for world in server.worlds.load().iter() {
            for player in world.players.load().iter() {
                let command_dispatcher = server.command_dispatcher.load();
                crate::command::client_suggestions::send_c_commands_packet(
                    player,
                    &server,
                    &command_dispatcher,
                );
            }
        }
    }

    fn get_player_by_uuid(&self, uuid: &Uuid) -> Option<Arc<dyn HostPlayer>> {
        self.server
            .get_player_by_uuid(*uuid)
            .map(|p| Arc::new(HostPlayerImpl { player: p, server: self.server.clone() }) as Arc<dyn HostPlayer>)
    }

    fn get_player_by_name(&self, name: &str) -> Option<Arc<dyn HostPlayer>> {
        self.server
            .get_player_by_name(name)
            .map(|p| Arc::new(HostPlayerImpl { player: p, server: self.server.clone() }) as Arc<dyn HostPlayer>)
    }

    fn get_online_players(&self) -> Vec<Arc<dyn HostPlayer>> {
        let mut result = Vec::new();
        for world in self.server.worlds.load().iter() {
            for player in world.players.load().iter() {
                result.push(Arc::new(HostPlayerImpl {
                    player: player.clone(),
                    server: self.server.clone(),
                }) as Arc<dyn HostPlayer>);
            }
        }
        result
    }

    fn get_world(&self, name: &str) -> Option<Arc<dyn HostWorld>> {
        for world in self.server.worlds.load().iter() {
            if world.dimension.minecraft_name == name {
                return Some(Arc::new(HostWorldImpl {
                    world: world.clone(),
                    server: self.server.clone(),
                }) as Arc<dyn HostWorld>);
            }
        }
        None
    }

    fn get_worlds(&self) -> Vec<Arc<dyn HostWorld>> {
        self.server
            .worlds
            .load()
            .iter()
            .map(|w| Arc::new(HostWorldImpl { world: w.clone(), server: self.server.clone() }) as Arc<dyn HostWorld>)
            .collect()
    }

    fn data_folder(&self) -> PathBuf {
        self.data_dir.clone()
    }

    fn broadcast(&self, message: &str) {
        let comp = TextComponent::text(message.to_string());
        for world in self.server.worlds.load().iter() {
            for player in world.players.load().iter() {
                player.send_system_message(&comp);
            }
        }
    }

    fn server_version(&self) -> &str {
        "1.21.4 (PotatoMC)"
    }

    fn get_tps(&self) -> f64 {
        self.server.get_tps()
    }

    fn max_players(&self) -> u32 {
        self.server.advanced_config.networking.java.max_players as u32
    }

    fn dispatch_command(&self, command: &str) -> bool {
        let line = command.strip_prefix('/').unwrap_or(command);
        self.server.command_dispatcher.load().handle_command(
            &crate::command::CommandSender::Console.into_source(&self.server),
            line,
        );
        true
    }

    fn server_motd(&self) -> String {
        self.server.advanced_config.networking.java.motd.clone()
    }

    fn shutdown(&self) {
        self.broadcast("§cServer is shutting down...");
    }
}

pub struct HostPlayerImpl {
    pub player: Arc<PumpkinPlayer>,
    pub server: Arc<Server>,
}

impl HostPlayer for HostPlayerImpl {
    fn uuid(&self) -> Uuid {
        self.player.gameprofile.id
    }

    fn name(&self) -> String {
        self.player.gameprofile.name.clone()
    }

    fn location(&self) -> PotatoLocation {
        let pos = self.player.living_entity.entity.pos.load();
        let yaw = self.player.living_entity.entity.yaw.load();
        let pitch = self.player.living_entity.entity.pitch.load();
        let world_id = self.player.world().dimension.minecraft_name.to_string();
        PotatoLocation::new(world_id, pos.x, pos.y, pos.z, yaw, pitch)
    }

    fn world_id(&self) -> String {
        self.player.world().dimension.minecraft_name.to_string()
    }

    fn teleport(&self, location: &PotatoLocation) -> bool {
        let pos = pumpkin_util::math::vector3::Vector3::new(location.x, location.y, location.z);
        let world = self.player.world();
        self.player
            .living_entity
            .entity
            .teleport(pos, Some(location.yaw), Some(location.pitch), &world);
        true
    }

    fn send_message(&self, message: &str) {
        self.player
            .send_system_message(&TextComponent::text(message.to_string()));
    }

    fn inventory(&self) -> Arc<dyn HostInventory> {
        Arc::new(HostInventoryImpl { player: self.player.clone() })
    }

    fn has_permission(&self, permission: &str) -> bool {
        self.player.has_permission(&self.server, permission)
    }

    fn gamemode(&self) -> PotatoGameMode {
        to_potato_gamemode(self.player.gamemode.load())
    }

    fn set_gamemode(&self, mode: PotatoGameMode) {
        self.player.set_gamemode(from_potato_gamemode(mode));
    }

    fn health(&self) -> f32 {
        self.player.living_entity.health.load()
    }

    fn set_health(&self, health: f32) {
        self.player.living_entity.set_health(health);
        self.player.send_health();
    }

    fn max_health(&self) -> f32 {
        self.player.living_entity.get_max_health()
    }

    fn food_level(&self) -> u32 {
        self.player.hunger_manager.level.load() as u32
    }

    fn set_food_level(&self, food: u32) {
        self.player.hunger_manager.level.store(food.min(20) as u8);
        self.player.send_health();
    }

    fn is_sneaking(&self) -> bool {
        self.player.living_entity.entity.is_sneaking()
    }

    fn is_sprinting(&self) -> bool {
        self.player.living_entity.entity.is_sprinting()
    }

    fn is_flying(&self) -> bool {
        self.player.is_flying()
    }

    fn set_flying(&self, flying: bool) {
        if let Ok(mut ab) = self.player.abilities.lock() {
            ab.flying = flying;
        }
        self.player.send_abilities_update();
    }

    fn can_fly(&self) -> bool {
        self.player.abilities.lock().map_or(false, |a| a.allow_flying)
    }

    fn set_can_fly(&self, can_fly: bool) {
        if let Ok(mut ab) = self.player.abilities.lock() {
            ab.allow_flying = can_fly;
        }
        self.player.send_abilities_update();
    }

    fn ping(&self) -> u32 {
        self.player.ping.load(Ordering::Relaxed)
    }

    fn level(&self) -> i32 {
        self.player.get_experience_level()
    }

    fn set_level(&self, level: i32) {
        self.player.set_experience_level(level, true);
    }

    fn exp(&self) -> i32 {
        self.player.experience_points.load(Ordering::Relaxed)
    }

    fn give_exp(&self, exp: i32) {
        self.player.add_experience_points(exp);
    }

    fn send_title(&self, title: &str, subtitle: &str, fade_in_ticks: u32, stay_ticks: u32, fade_out_ticks: u32) {
        self.player.send_title_animation(fade_in_ticks as i32, stay_ticks as i32, fade_out_ticks as i32);
        if !title.is_empty() {
            self.player.show_title(&TextComponent::text(title.to_string()), &crate::entity::player::TitleMode::Title);
        }
        if !subtitle.is_empty() {
            self.player.show_title(&TextComponent::text(subtitle.to_string()), &crate::entity::player::TitleMode::SubTitle);
        }
    }

    fn send_action_bar(&self, message: &str) {
        self.player.show_title(&TextComponent::text(message.to_string()), &crate::entity::player::TitleMode::ActionBar);
    }

    fn play_sound(&self, sound: &str, volume: f32, pitch: f32) {
        let sound_event = pumpkin_data::sound::Sound::from_name(sound)
            .map(|s| pumpkin_protocol::IdOr::Id(s as u16))
            .unwrap_or_else(|| pumpkin_protocol::IdOr::Value(pumpkin_protocol::SoundEvent {
                sound_name: sound.to_string(),
                range: None,
            }));
        let pos = self.player.living_entity.entity.pos.load();
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CSoundEffect::new(
            sound_event,
            pumpkin_data::sound::SoundCategory::Master,
            &pos,
            volume,
            pitch,
            0.0,
        ));
    }

    fn kick(&self, reason: &str) {
        self.player.kick(
            crate::net::DisconnectReason::Kicked,
            &TextComponent::text(reason.to_string()),
        );
    }

    fn set_player_list_header_footer(&self, header: &str, footer: &str) {
        let header_comp = TextComponent::text(header.to_string());
        let footer_comp = TextComponent::text(footer.to_string());
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CTabList::new(
            &header_comp,
            &footer_comp,
        ));
    }

    fn drop_item(&self, item: &PotatoItemStack) {
        if let Some(stack) = to_pumpkin_item_stack(item) {
            self.player.drop_item(stack);
        }
    }

    fn give_item(&self, item: &PotatoItemStack) -> bool {
        if let Some(mut stack) = to_pumpkin_item_stack(item) {
            let inserted = self.player.inventory.insert_stack_anywhere(&mut stack);
            if !stack.is_empty() {
                self.player.drop_item(stack);
            }
            if let Ok(inv) = self.player.inventory.main_inventory.read() {
                for (idx, slot_stack) in inv.iter().enumerate() {
                    let network_slot = if idx < 9 { (idx + 36) as i16 } else { idx as i16 };
                    let serializer = ItemStackSerializer::from(slot_stack.clone());
                    let packet = CSetContainerSlot::new(0, 0, network_slot, &serializer);
                    self.player.try_send_client_packet(&packet);
                }
            }
            inserted
        } else {
            false
        }
    }

    fn open_gui(&self, title: &str, size: usize, items: &[(usize, PotatoItemStack)], allow_grab: bool, allow_put: bool) -> u8 {
        let rows = (size / 9).clamp(1, 6) as u8;
        let window_type = match rows {
            1 => WindowType::Generic9x1,
            2 => WindowType::Generic9x2,
            3 => WindowType::Generic9x3,
            4 => WindowType::Generic9x4,
            5 => WindowType::Generic9x5,
            6 => WindowType::Generic9x6,
            _ => WindowType::Generic9x3,
        };
        let inv_size = (rows as usize) * 9;
        let simple_inv = Arc::new(PotatoSimpleInventory::new(inv_size));
        for (slot, item) in items {
            if *slot < inv_size {
                if let Some(stack) = to_pumpkin_item_stack(item) {
                    simple_inv.set_stack(*slot, stack);
                }
            }
        }
        let factory = PotatoGuiFactory {
            title: title.to_string(),
            window_type,
            inventory: simple_inv,
            allow_grab,
            allow_put,
        };
        self.player.open_handled_screen(&factory, None).unwrap_or(0)
    }

    fn close_inventory(&self) {
        self.player.close_handled_screen();
    }

    fn add_potion_effect(&self, effect: &PotatoPotionEffect) {
        let clean = effect.name();
        if let Some(status_effect) = StatusEffect::from_minecraft_name(clean) {
            self.player.add_effect(PumpkinEffect {
                effect_type: status_effect,
                duration: effect.duration_ticks as i32,
                amplifier: effect.amplifier,
                ambient: effect.ambient,
                show_particles: effect.particles,
                show_icon: effect.show_icon,
                blend: false,
            });
        }
    }

    fn remove_potion_effect(&self, effect_type: &str) {
        let clean = effect_type.strip_prefix("minecraft:").unwrap_or(effect_type);
        if let Some(status_effect) = StatusEffect::from_minecraft_name(clean) {
            self.player.remove_effect(status_effect);
        }
    }

    fn clear_potion_effects(&self) {
        let active = self.player.get_active_effects();
        for eff in active {
            self.player.remove_effect(eff.effect_type);
        }
    }

    fn has_potion_effect(&self, effect_type: &str) -> bool {
        let clean = effect_type.strip_prefix("minecraft:").unwrap_or(effect_type);
        if let Some(status_effect) = StatusEffect::from_minecraft_name(clean) {
            self.player.has_effect(status_effect)
        } else {
            false
        }
    }

    fn spawn_particle(&self, particle: &str, location: &PotatoLocation, count: u32, offset_x: f64, offset_y: f64, offset_z: f64, speed: f32) {
        let clean = particle.strip_prefix("minecraft:").unwrap_or(particle);
        if let Some(part) = PumpkinParticle::from_name(clean) {
            let pos = pumpkin_util::math::vector3::Vector3::new(location.x, location.y, location.z);
            let offset = pumpkin_util::math::vector3::Vector3::new(offset_x as f32, offset_y as f32, offset_z as f32);
            self.player.spawn_particle(pos, offset, speed, count as i32, part);
        }
    }

    fn clear_title(&self) {
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CClearTitle::new(false));
    }

    fn reset_title(&self) {
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CClearTitle::new(true));
    }

    fn play_sound_category(&self, sound: &str, category: u8, volume: f32, pitch: f32) {
        let sound_event = pumpkin_data::sound::Sound::from_name(sound)
            .map(|s| pumpkin_protocol::IdOr::Id(s as u16))
            .unwrap_or_else(|| pumpkin_protocol::IdOr::Value(pumpkin_protocol::SoundEvent {
                sound_name: sound.to_string(),
                range: None,
            }));
        let pos = self.player.living_entity.entity.pos.load();
        let sound_cat = match category {
            1 => pumpkin_data::sound::SoundCategory::Music,
            2 => pumpkin_data::sound::SoundCategory::Records,
            3 => pumpkin_data::sound::SoundCategory::Weather,
            4 => pumpkin_data::sound::SoundCategory::Blocks,
            5 => pumpkin_data::sound::SoundCategory::Hostile,
            6 => pumpkin_data::sound::SoundCategory::Neutral,
            7 => pumpkin_data::sound::SoundCategory::Players,
            8 => pumpkin_data::sound::SoundCategory::Ambient,
            9 => pumpkin_data::sound::SoundCategory::Voice,
            _ => pumpkin_data::sound::SoundCategory::Master,
        };
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CSoundEffect::new(
            sound_event,
            sound_cat,
            &pos,
            volume,
            pitch,
            0.0,
        ));
    }

    fn clear_dialog(&self) {
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CPlayClearDialog::new());
    }

    fn send_form_raw(&self, form_id: u32, form_json: &str) {
        if let ClientPlatform::Bedrock(client) = self.player.client.as_ref() {
            client.try_enqueue_client_packet(&pumpkin_protocol::bedrock::client::modal_form_request::CModalFormRequest {
                form_id: pumpkin_protocol::codec::var_uint::VarUInt(form_id),
                form_ui_json: form_json.to_string(),
            });
        }
    }

    fn is_op(&self) -> bool {
        self.player.permission_lvl.load() >= pumpkin_util::PermissionLvl::Two
    }

    fn set_op(&self, op: bool) {
        self.player.permission_lvl.store(if op {
            pumpkin_util::PermissionLvl::Two
        } else {
            pumpkin_util::PermissionLvl::Zero
        });
    }

    fn send_demo_screen(&self) {
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CGameEvent {
            event: 5,
            value: 0.0,
        });
    }

    fn send_game_event(&self, event_type: u8, value: f32) {
        self.player.try_send_client_packet(&pumpkin_protocol::java::client::play::CGameEvent {
            event: event_type,
            value,
        });
    }

    fn get_target_block(&self, max_distance: f64) -> Option<PotatoBlock> {
        let eye = self.player.living_entity.entity.pos.load();
        let yaw = self.player.living_entity.entity.yaw.load();
        let pitch = self.player.living_entity.entity.pitch.load();
        let dir = potato_api::types::Vector3::from_yaw_pitch(yaw, pitch);
        let world = self.player.world();
        potato_api::raytrace::RayTrace::trace_blocks(
            potato_api::types::Vector3::new(eye.x, eye.y + 1.62, eye.z),
            dir,
            max_distance,
            0.2,
            |x, y, z| {
                let pos = BlockPos::new(x, y, z);
                let (block, state_id) = world.get_block_and_state_id(&pos);
                if block.is_air() {
                    None
                } else {
                    Some(PotatoBlock::new(block.name.to_string(), state_id.as_u16() as u32))
                }
            },
        ).map(|(b, _, _)| b)
    }
}

pub struct HostInventoryImpl {
    pub player: Arc<PumpkinPlayer>,
}

impl HostInventory for HostInventoryImpl {
    fn get_held_item(&self) -> Option<PotatoItemStack> {
        let item = self.player.inventory.held_item();
        to_potato_item_stack(&item)
    }

    fn set_held_item(&self, item: Option<PotatoItemStack>) {
        let slot = self.player.inventory.selected_slot.load(Ordering::Relaxed) as usize;
        self.set_slot(slot, item);
    }

    fn get_slot(&self, slot: usize) -> Option<PotatoItemStack> {
        let inv = self.player.inventory.main_inventory.read().ok()?;
        let item = inv.get(slot)?;
        to_potato_item_stack(item)
    }

    fn set_slot(&self, slot: usize, item: Option<PotatoItemStack>) {
        let stack = if let Some(ref it) = item {
            to_pumpkin_item_stack(it).unwrap_or_else(|| PumpkinItemStack::EMPTY.clone())
        } else {
            PumpkinItemStack::EMPTY.clone()
        };

        if let Ok(mut inv) = self.player.inventory.main_inventory.write() {
            if slot < inv.len() {
                inv[slot] = stack.clone();
            }
        }

        // Immediately sync slot update to the player's client window 0
        if slot < 36 {
            let network_slot = if slot < 9 { (slot + 36) as i16 } else { slot as i16 };
            let stack_serializer = ItemStackSerializer::from(stack);
            let packet = CSetContainerSlot::new(0, 0, network_slot, &stack_serializer);
            self.player.try_send_client_packet(&packet);
        }
    }

    fn get_equipment(&self, slot: PotatoEquipmentSlot) -> Option<PotatoItemStack> {
        let eq = self.player.inventory.entity_equipment.lock().unwrap();
        let pumpkin_slot = to_pumpkin_slot(slot);
        let item = eq.get(&pumpkin_slot);
        to_potato_item_stack(&item)
    }

    fn set_equipment(&self, slot: PotatoEquipmentSlot, item: Option<PotatoItemStack>) {
        let mut eq = self.player.inventory.entity_equipment.lock().unwrap();
        let pumpkin_slot = to_pumpkin_slot(slot);
        let stack = if let Some(ref it) = item {
            to_pumpkin_item_stack(it).unwrap_or_else(|| PumpkinItemStack::EMPTY.clone())
        } else {
            PumpkinItemStack::EMPTY.clone()
        };
        eq.put(&pumpkin_slot, stack);
    }

    fn clear(&self) {
        self.player.inventory.clear();
    }
}

pub struct HostWorldImpl {
    pub world: Arc<PumpkinWorld>,
    pub server: Arc<Server>,
}

impl HostWorld for HostWorldImpl {
    fn identity(&self) -> String {
        self.world.dimension.minecraft_name.to_string()
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> Option<PotatoBlock> {
        let pos = BlockPos::new(x, y, z);
        let (block, state_id) = self.world.get_block_and_state_id(&pos);
        Some(PotatoBlock::new(block.name.to_string(), state_id.as_u16() as u32))
    }

    fn set_block(&self, x: i32, y: i32, z: i32, block: &PotatoBlock) -> bool {
        let pos = BlockPos::new(x, y, z);
        let state_id = pumpkin_data::BlockStateId::new_or_air(block.state_id as u16);
        self.world.set_block_state(&pos, state_id, pumpkin_world::world::BlockFlags::NOTIFY_ALL);
        true
    }

    fn spawn_entity(&self, _entity_type: &str, _location: &PotatoLocation) -> Result<Arc<dyn HostEntity>, String> {
        Err("Custom entity spawning through Potato API will be available in future releases".to_string())
    }

    fn player_lookup(&self, name: &str) -> Option<Arc<dyn HostPlayer>> {
        self.world
            .get_player_by_name(name)
            .map(|p| Arc::new(HostPlayerImpl { player: p, server: self.server.clone() }) as Arc<dyn HostPlayer>)
    }

    fn players(&self) -> Vec<Arc<dyn HostPlayer>> {
        self.world
            .players
            .load()
            .iter()
            .map(|p| Arc::new(HostPlayerImpl { player: p.clone(), server: self.server.clone() }) as Arc<dyn HostPlayer>)
            .collect()
    }

    fn time(&self) -> u64 {
        self.world.get_time_of_day() as u64
    }

    fn set_time(&self, time: u64) {
        self.world.set_time_of_day(time as i64);
    }

    fn is_raining(&self) -> bool {
        self.world.is_raining()
    }

    fn set_storm(&self, storm: bool) {
        self.world.set_raining(storm);
    }

    fn difficulty(&self) -> PotatoDifficulty {
        to_potato_difficulty(self.server.get_difficulty())
    }

    fn set_difficulty(&self, diff: PotatoDifficulty) {
        self.server.set_difficulty(from_potato_difficulty(diff), true);
    }

    fn create_explosion(&self, x: f64, y: f64, z: f64, power: f32, _fire: bool, break_blocks: bool) {
        let interaction = if break_blocks {
            crate::world::ExplosionInteraction::Block
        } else {
            crate::world::ExplosionInteraction::None
        };
        self.world.explode(
            pumpkin_util::math::vector3::Vector3::new(x, y, z),
            power,
            interaction,
        );
    }

    fn play_sound(&self, location: &PotatoLocation, sound: &str, volume: f32, pitch: f32) {
        let sound_event = pumpkin_data::sound::Sound::from_name(sound)
            .map(|s| pumpkin_protocol::IdOr::Id(s as u16))
            .unwrap_or_else(|| pumpkin_protocol::IdOr::Value(pumpkin_protocol::SoundEvent {
                sound_name: sound.to_string(),
                range: None,
            }));
        let pos = pumpkin_util::math::vector3::Vector3::new(location.x, location.y, location.z);
        let packet = pumpkin_protocol::java::client::play::CSoundEffect::new(
            sound_event,
            pumpkin_data::sound::SoundCategory::Master,
            &pos,
            volume,
            pitch,
            0.0,
        );
        for player in self.world.players.load().iter() {
            player.try_send_client_packet(&packet);
        }
    }

    fn broadcast_message(&self, message: &str) {
        let comp = TextComponent::text(message.to_string());
        for player in self.world.players.load().iter() {
            player.send_system_message(&comp);
        }
    }

    fn drop_item(&self, location: &PotatoLocation, item: &PotatoItemStack) {
        if let Some(stack) = to_pumpkin_item_stack(item) {
            let pos = BlockPos::new(location.x.floor() as i32, location.y.floor() as i32, location.z.floor() as i32);
            self.world.drop_stack(&pos, stack);
        }
    }

    fn drop_item_naturally(&self, location: &PotatoLocation, item: &PotatoItemStack) {
        self.drop_item(location, item);
    }

    fn break_block(&self, x: i32, y: i32, z: i32, drop_items: bool) -> bool {
        let pos = BlockPos::new(x, y, z);
        let flags = if drop_items {
            pumpkin_world::world::BlockFlags::NOTIFY_ALL
        } else {
            pumpkin_world::world::BlockFlags::NOTIFY_ALL | pumpkin_world::world::BlockFlags::SKIP_DROPS
        };
        self.world.break_block(&pos, None, flags);
        true
    }

    fn spawn_particle(&self, particle: &str, location: &PotatoLocation, count: u32, offset_x: f64, offset_y: f64, offset_z: f64, speed: f32) {
        let clean = particle.strip_prefix("minecraft:").unwrap_or(particle);
        if let Some(part) = PumpkinParticle::from_name(clean) {
            let pos = pumpkin_util::math::vector3::Vector3::new(location.x, location.y, location.z);
            let offset = pumpkin_util::math::vector3::Vector3::new(offset_x as f32, offset_y as f32, offset_z as f32);
            self.world.spawn_particle(pos, offset, speed, count as i32, part);
        }
    }
}

pub struct HostEntityImpl {
    pub entity: Arc<crate::entity::Entity>,
    pub world_id: String,
}

impl HostEntity for HostEntityImpl {
    fn uuid(&self) -> Uuid {
        self.entity.entity_uuid
    }

    fn entity_type(&self) -> String {
        self.entity.entity_type.resource_name.to_string()
    }

    fn location(&self) -> PotatoLocation {
        let pos = self.entity.pos.load();
        let yaw = self.entity.yaw.load();
        let pitch = self.entity.pitch.load();
        PotatoLocation::new(self.world_id.clone(), pos.x, pos.y, pos.z, yaw, pitch)
    }

    fn teleport(&self, _location: &PotatoLocation) -> bool {
        false
    }

    fn velocity(&self) -> PotatoVector3 {
        let vel = self.entity.velocity.load();
        PotatoVector3::new(vel.x, vel.y, vel.z)
    }

    fn set_velocity(&self, velocity: &PotatoVector3) {
        self.entity.velocity.store(pumpkin_util::math::vector3::Vector3::new(
            velocity.x, velocity.y, velocity.z,
        ));
    }

    fn remove(&self) {
        let _ = self.entity.removal_reason.swap(Some(crate::entity::RemovalReason::Discarded));
    }

    fn as_living(&self) -> Option<Arc<dyn HostLivingEntity>> {
        None
    }

    fn is_on_ground(&self) -> bool {
        self.entity.on_ground.load(Ordering::Relaxed)
    }

    fn custom_name(&self) -> Option<String> {
        self.entity.custom_name.load().as_ref().clone().map(|t| t.to_pretty_console())
    }

    fn set_custom_name(&self, name: Option<&str>) {
        if let Some(n) = name {
            self.entity.set_custom_name(TextComponent::text(n.to_string()));
        } else {
            self.entity.custom_name.store(Arc::new(None));
        }
    }

    fn fire_ticks(&self) -> i32 {
        self.entity.fire_ticks.load(Ordering::Relaxed)
    }

    fn set_fire_ticks(&self, ticks: i32) {
        self.entity.fire_ticks.store(ticks, Ordering::Relaxed);
    }

    fn damage(&self, _amount: f32) {}
}

pub struct HostLivingEntityImpl {
    pub base: Arc<HostEntityImpl>,
    pub living: Arc<crate::entity::living::LivingEntity>,
}

impl HostLivingEntity for HostLivingEntityImpl {
    fn base_entity(&self) -> Arc<dyn HostEntity> {
        self.base.clone()
    }

    fn health(&self) -> f32 {
        self.living.health.load()
    }

    fn set_health(&self, health: f32) {
        self.living.set_health(health);
    }

    fn max_health(&self) -> f32 {
        self.living.get_max_health()
    }
}

pub struct HostConsoleImpl;

impl HostConsole for HostConsoleImpl {
    fn send_message(&self, message: &str) {
        tracing::info!(target: "console", "{}", message);
    }
}

struct PotatoCommandBridgeExecutor {
    node: CommandNode,
    server: Arc<Server>,
}

impl CommandExecutor for PotatoCommandBridgeExecutor {
    fn execute(&self, context: &PumpkinCommandContext) -> CommandExecutorResult {
        let sender = match context.source.as_player() {
            Some(p) => PotatoCommandSender::Player(potato_api::Player::from_handle(Arc::new(
                HostPlayerImpl { player: p, server: self.server.clone() },
            ))),
            None => PotatoCommandSender::Console(Arc::new(HostConsoleImpl)),
        };

        if self.node.player_only && !sender.is_player() {
            sender.send_error("This command can only be executed by in-game players.");
            return Ok(0);
        }

        if let Some(ref perm) = self.node.permission {
            if !sender.has_permission(perm) {
                sender.send_error(&format!("I'm sorry, but you do not have permission to perform this command ({perm})."));
                return Ok(0);
            }
        }

        let mut parsed_args = HashMap::new();

        // 1. Fetch typed arguments from Pumpkin's CommandContext
        for arg_def in &self.node.arguments {
            match &arg_def.arg_type {
                ArgumentType::Integer { .. } => {
                    if let Ok(val) = context.get_argument::<i32>(&arg_def.name) {
                        parsed_args.insert(arg_def.name.clone(), ParsedValue::Integer(*val));
                    }
                }
                ArgumentType::Float { .. } => {
                    if let Ok(val) = context.get_argument::<f32>(&arg_def.name) {
                        parsed_args.insert(arg_def.name.clone(), ParsedValue::Float(*val));
                    }
                }
                ArgumentType::Boolean => {
                    if let Ok(val) = context.get_argument::<bool>(&arg_def.name) {
                        parsed_args.insert(arg_def.name.clone(), ParsedValue::Boolean(*val));
                    }
                }
                ArgumentType::GreedyString | ArgumentType::Word | ArgumentType::String | ArgumentType::Player => {
                    if let Ok(val) = context.get_argument::<String>(&arg_def.name) {
                        parsed_args.insert(arg_def.name.clone(), ParsedValue::String(val.clone()));
                    }
                }
            }
        }

        // 2. Token fallback from context.input relative to this node
        let raw_tokens: Vec<String> = context
            .input
            .split_whitespace()
            .map(String::from)
            .collect();

        let token_offset = raw_tokens
            .iter()
            .position(|t| t.eq_ignore_ascii_case(&self.node.name))
            .map(|pos| pos + 1)
            .unwrap_or(1);

        let sub_tokens = if token_offset < raw_tokens.len() {
            raw_tokens[token_offset..].to_vec()
        } else {
            Vec::new()
        };

        for (i, arg_def) in self.node.arguments.iter().enumerate() {
            if !parsed_args.contains_key(&arg_def.name) {
                if let Some(token) = sub_tokens.get(i) {
                    match &arg_def.arg_type {
                        ArgumentType::Integer { min, max } => {
                            if let Ok(val) = token.parse::<i32>() {
                                if min.map_or(true, |m| val >= m) && max.map_or(true, |m| val <= m) {
                                    parsed_args.insert(arg_def.name.clone(), ParsedValue::Integer(val));
                                }
                            }
                        }
                        ArgumentType::Float { min, max } => {
                            if let Ok(val) = token.parse::<f32>() {
                                if min.map_or(true, |m| val >= m) && max.map_or(true, |m| val <= m) {
                                    parsed_args.insert(arg_def.name.clone(), ParsedValue::Float(val));
                                }
                            }
                        }
                        ArgumentType::Boolean => {
                            if let Ok(val) = token.parse::<bool>() {
                                parsed_args.insert(arg_def.name.clone(), ParsedValue::Boolean(val));
                            }
                        }
                        ArgumentType::GreedyString => {
                            let greedy = sub_tokens[i..].join(" ");
                            parsed_args.insert(arg_def.name.clone(), ParsedValue::String(greedy));
                            break;
                        }
                        _ => {
                            parsed_args.insert(arg_def.name.clone(), ParsedValue::String(token.clone()));
                        }
                    }
                }
            }
        }

        let server_clone = self.server.clone();
        let player_resolver = Arc::new(move |name: &str| {
            server_clone.get_player_by_name(name).map(|p| {
                potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                    player: p,
                    server: server_clone.clone(),
                }))
            })
        });

        let potato_context = PotatoCommandContext::new(
            sender.clone(),
            self.node.name.clone(),
            parsed_args,
            sub_tokens,
            Some(player_resolver),
        );

        if let Some(ref executor) = self.node.executor {
            match executor(&potato_context) {
                Ok(()) => Ok(1),
                Err(e) => {
                    sender.send_error(&e.to_string());
                    Ok(0)
                }
            }
        } else {
            Ok(1)
        }
    }
}

/// Registers the event bridges into Pumpkin's event dispatcher for Potato plugins.
pub fn hook_event_bridges(server: &Arc<Server>, host_context: &Arc<PotatoHostContext>) {
    let listeners = host_context.event_listeners.clone();
    let plugin_name = host_context.plugin_name.clone();

    // 1. PlayerJoinEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerJoinBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerJoinEvent> for PlayerJoinBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerJoinEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerJoinEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerJoinEvent {
                            player,
                            join_message: Some(event.join_message.clone().to_pretty_console()),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerJoinEvent::EVENT_ID, (&mut potato_event as *mut PlayerJoinEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                        if let Some(ref msg) = potato_event.join_message {
                            event.join_message = TextComponent::text(msg.clone());
                        }
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerJoinBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerJoinEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 2. PlayerQuitEvent (PlayerLeaveEvent)
    {
        let listeners_clone = listeners.clone();
        struct PlayerLeaveBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerLeaveEvent> for PlayerLeaveBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerLeaveEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerQuitEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerQuitEvent {
                            player,
                            quit_message: Some(event.leave_message.clone().to_pretty_console()),
                        };
                        for l in list {
                            l.handle_raw(PlayerQuitEvent::EVENT_ID, (&mut potato_event as *mut PlayerQuitEvent).cast::<()>());
                        }
                        if let Some(ref msg) = potato_event.quit_message {
                            event.leave_message = TextComponent::text(msg.clone());
                        }
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerLeaveBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerLeaveEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 3. PlayerMoveEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerMoveBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerMoveEvent> for PlayerMoveBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerMoveEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerMoveEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let world_id = event.player.world().dimension.minecraft_name.to_string();
                        let mut potato_event = PlayerMoveEvent {
                            player,
                            from: PotatoLocation::new(world_id.clone(), event.from.x, event.from.y, event.from.z, 0.0, 0.0),
                            to: PotatoLocation::new(world_id, event.to.x, event.to.y, event.to.z, 0.0, 0.0),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerMoveEvent::EVENT_ID, (&mut potato_event as *mut PlayerMoveEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerMoveBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerMoveEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 4. BlockBreakEvent
    {
        let listeners_clone = listeners.clone();
        struct BlockBreakBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinBlockBreakEvent> for BlockBreakBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinBlockBreakEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&BlockBreakEvent::EVENT_ID) {
                        let player = event.player.as_ref().map(|p| {
                            potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                                player: p.clone(),
                                server: server.clone(),
                            }))
                        });
                        let world_id = event
                            .player
                            .as_ref()
                            .map_or_else(|| "minecraft:overworld".to_string(), |p| p.world().dimension.minecraft_name.to_string());
                        let mut potato_event = BlockBreakEvent {
                            player,
                            block: PotatoBlock::new(event.block.name.to_string(), event.block.default_state.id.as_u16() as u32),
                            location: PotatoLocation::new(
                                world_id,
                                event.block_position.0.x as f64,
                                event.block_position.0.y as f64,
                                event.block_position.0.z as f64,
                                0.0,
                                0.0,
                            ),
                            drop_items: event.drop,
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(BlockBreakEvent::EVENT_ID, (&mut potato_event as *mut BlockBreakEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                        event.drop = potato_event.drop_items;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(BlockBreakBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinBlockBreakEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 5. BlockPlaceEvent
    {
        let listeners_clone = listeners.clone();
        struct BlockPlaceBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinBlockPlaceEvent> for BlockPlaceBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinBlockPlaceEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&BlockPlaceEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let world_id = event.player.world().dimension.minecraft_name.to_string();
                        let mut potato_event = BlockPlaceEvent {
                            player,
                            block: PotatoBlock::new(
                                event.block_placed.name.to_string(),
                                event.block_placed.default_state.id.as_u16() as u32,
                            ),
                            location: PotatoLocation::new(
                                world_id,
                                event.block_position.0.x as f64,
                                event.block_position.0.y as f64,
                                event.block_position.0.z as f64,
                                0.0,
                                0.0,
                            ),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(BlockPlaceEvent::EVENT_ID, (&mut potato_event as *mut BlockPlaceEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(BlockPlaceBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinBlockPlaceEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 6. PlayerChatEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerChatBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerChatEvent> for PlayerChatBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerChatEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerChatEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerChatEvent {
                            player,
                            message: event.message.clone(),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerChatEvent::EVENT_ID, (&mut potato_event as *mut PlayerChatEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                        event.message = potato_event.message;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerChatBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerChatEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 7. PlayerCommandPreprocessEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerCommandPreprocessBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerCommandPreprocessEvent> for PlayerCommandPreprocessBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerCommandPreprocessEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerCommandPreprocessEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerCommandPreprocessEvent {
                            player,
                            command: event.command.clone(),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerCommandPreprocessEvent::EVENT_ID, (&mut potato_event as *mut PlayerCommandPreprocessEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                        event.command = potato_event.command;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerCommandPreprocessBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerCommandPreprocessEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 8. PlayerDropItemEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerDropItemBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerDropItemEvent> for PlayerDropItemBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerDropItemEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerDropItemEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerDropItemEvent {
                            player,
                            item: PotatoItemStack::new(event.item_name.clone(), event.count as u32),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerDropItemEvent::EVENT_ID, (&mut potato_event as *mut PlayerDropItemEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerDropItemBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerDropItemEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 9. PlayerItemConsumeEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerItemConsumeBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerItemConsumeEvent> for PlayerItemConsumeBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerItemConsumeEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerItemConsumeEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerItemConsumeEvent {
                            player,
                            item: PotatoItemStack::new(event.item_name.clone(), 1),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerItemConsumeEvent::EVENT_ID, (&mut potato_event as *mut PlayerItemConsumeEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerItemConsumeBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerItemConsumeEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 10. PlayerTeleportEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerTeleportBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerTeleportEvent> for PlayerTeleportBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerTeleportEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerTeleportEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let world_id = event.player.world().dimension.minecraft_name.to_string();
                        let mut potato_event = PlayerTeleportEvent {
                            player,
                            from: PotatoLocation::new(world_id.clone(), event.from.x, event.from.y, event.from.z, 0.0, 0.0),
                            to: PotatoLocation::new(world_id, event.to.x, event.to.y, event.to.z, 0.0, 0.0),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerTeleportEvent::EVENT_ID, (&mut potato_event as *mut PlayerTeleportEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerTeleportBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerTeleportEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 11. PlayerGamemodeChangeEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerGamemodeChangeBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerGamemodeChangeEvent> for PlayerGamemodeChangeBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerGamemodeChangeEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerGameModeChangeEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerGameModeChangeEvent {
                            player,
                            new_gamemode: to_potato_gamemode(event.new_gamemode),
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerGameModeChangeEvent::EVENT_ID, (&mut potato_event as *mut PlayerGameModeChangeEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                        event.new_gamemode = from_potato_gamemode(potato_event.new_gamemode);
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerGamemodeChangeBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerGamemodeChangeEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 12. PlayerToggleSneakEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerToggleSneakBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerToggleSneakEvent> for PlayerToggleSneakBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerToggleSneakEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerToggleSneakEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerToggleSneakEvent {
                            player,
                            is_sneaking: event.is_sneaking,
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerToggleSneakEvent::EVENT_ID, (&mut potato_event as *mut PlayerToggleSneakEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerToggleSneakBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerToggleSneakEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 13. PlayerToggleSprintEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerToggleSprintBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerToggleSprintEvent> for PlayerToggleSprintBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerToggleSprintEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerToggleSprintEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerToggleSprintEvent {
                            player,
                            is_sprinting: event.is_sprinting,
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerToggleSprintEvent::EVENT_ID, (&mut potato_event as *mut PlayerToggleSprintEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerToggleSprintBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerToggleSprintEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 14. PlayerToggleFlightEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerToggleFlightBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerToggleFlightEvent> for PlayerToggleFlightBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerToggleFlightEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerToggleFlightEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let mut potato_event = PlayerToggleFlightEvent {
                            player,
                            is_flying: event.is_flying,
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(PlayerToggleFlightEvent::EVENT_ID, (&mut potato_event as *mut PlayerToggleFlightEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerToggleFlightBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerToggleFlightEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 15. PlayerRespawnEvent
    {
        let listeners_clone = listeners.clone();
        struct PlayerRespawnBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinPlayerRespawnEvent> for PlayerRespawnBridge {
            fn handle_blocking<'a>(
                &'a self,
                server: &'a Arc<Server>,
                event: &'a mut PumpkinPlayerRespawnEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&PlayerRespawnEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let world_id = event.respawned_world.dimension.minecraft_name.to_string();
                        let mut potato_event = PlayerRespawnEvent {
                            player,
                            respawn_location: PotatoLocation::new(
                                world_id,
                                event.position.x,
                                event.position.y,
                                event.position.z,
                                event.yaw,
                                event.pitch,
                            ),
                            is_bed_spawn: false,
                        };
                        for l in list {
                            l.handle_raw(PlayerRespawnEvent::EVENT_ID, (&mut potato_event as *mut PlayerRespawnEvent).cast::<()>());
                        }
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(PlayerRespawnBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinPlayerRespawnEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 16. ServerListPingEvent
    {
        let listeners_clone = listeners.clone();
        struct ServerListPingBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinServerListPingEvent> for ServerListPingBridge {
            fn handle_blocking<'a>(
                &'a self,
                _server: &'a Arc<Server>,
                event: &'a mut PumpkinServerListPingEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&ServerListPingEvent::EVENT_ID) {
                        let mut potato_event = ServerListPingEvent {
                            motd: event.motd.clone().to_pretty_console(),
                            max_players: event.max_players,
                            online_players: event.num_players,
                        };
                        for l in list {
                            l.handle_raw(ServerListPingEvent::EVENT_ID, (&mut potato_event as *mut ServerListPingEvent).cast::<()>());
                        }
                        event.motd = TextComponent::text(potato_event.motd);
                        event.max_players = potato_event.max_players;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(ServerListPingBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinServerListPingEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }

    // 17. InventoryClickEvent
    {
        let listeners_clone = listeners.clone();
        struct InventoryClickBridge {
            listeners: Arc<RwLock<HashMap<u32, Vec<Arc<dyn RawEventListener>>>>>,
        }
        impl EventHandler<PumpkinInventoryClickEvent> for InventoryClickBridge {
            fn handle_blocking<'a>(
                &'a self,
                _server: &'a Arc<Server>,
                event: &'a mut PumpkinInventoryClickEvent,
            ) -> crate::plugin::BoxFuture<'a, ()> {
                let server = _server.clone();
                Box::pin(async move {
                    let map = self.listeners.read().unwrap();
                    if let Some(list) = map.get(&InventoryClickEvent::EVENT_ID) {
                        let player = potato_api::Player::from_handle(Arc::new(HostPlayerImpl {
                            player: event.player.clone(),
                            server: server.clone(),
                        }));
                        let click_type = match event.click_type {
                            ClickType::Left => 0,
                            ClickType::Right => 1,
                            ClickType::ShiftLeft => 2,
                            ClickType::ShiftRight => 3,
                            ClickType::NumberKey(n) => 4 + n,
                            ClickType::Middle => 14,
                            ClickType::Drop => 15,
                            ClickType::ControlDrop => 16,
                            ClickType::DoubleClick => 17,
                            _ => 255,
                        };
                        let clicked_item = event.clicked_item.as_ref().and_then(to_potato_item_stack);
                        let cursor_item = event.cursor.as_ref().and_then(to_potato_item_stack);
                        let mut potato_event = InventoryClickEvent {
                            player,
                            slot: i32::from(event.slot),
                            click_type,
                            clicked_item,
                            cursor_item,
                            cancelled: event.cancelled,
                        };
                        for l in list {
                            l.handle_raw(InventoryClickEvent::EVENT_ID, (&mut potato_event as *mut InventoryClickEvent).cast::<()>());
                        }
                        event.cancelled = potato_event.cancelled;
                    }
                })
            }
        }

        let typed_handler = Arc::new(TypedEventHandler {
            handler: Arc::new(InventoryClickBridge { listeners: listeners_clone }),
            priority: EventPriority::Normal,
            blocking: true,
            source: Some(plugin_name.clone()),
            _phantom: std::marker::PhantomData,
        });

        server.plugin_manager.handlers.rcu(|h| {
            let mut new_h = (**h).clone();
            new_h
                .entry(PumpkinInventoryClickEvent::get_name_static())
                .or_default()
                .push(typed_handler.clone());
            Arc::new(new_h)
        });
    }
}

/// Adapter bridging PotatoMC native plugins with Pumpkin's internal plugin container.
pub struct PotatoPluginAdapter {
    pub inner: Box<dyn potato_api::Plugin>,
    pub host_context: Arc<OnceLock<Arc<PotatoHostContext>>>,
    pub potato_context: Arc<OnceLock<potato_api::PluginContext>>,
}

impl PotatoPluginAdapter {
    pub fn new(inner: Box<dyn potato_api::Plugin>) -> Self {
        Self {
            inner,
            host_context: Arc::new(OnceLock::new()),
            potato_context: Arc::new(OnceLock::new()),
        }
    }
}

impl PumpkinPlugin for PotatoPluginAdapter {
    fn on_load(&self, context: Arc<PumpkinContext>) -> PluginFuture<'_, Result<(), String>> {
        Box::pin(async move {
            let meta = self.inner.metadata();
            let host_ctx = Arc::new(PotatoHostContext::new(meta.name.clone(), context.server.clone()));
            hook_event_bridges(&context.server, &host_ctx);

            let potato_ctx = potato_api::PluginContext::new(host_ctx.clone());
            let _ = self.host_context.set(host_ctx);
            let _ = self.potato_context.set(potato_ctx.clone());

            self.inner.on_load(&potato_ctx)?;
            self.inner.on_enable(&potato_ctx)?;

            tracing::info!(target: "plugin", "Enabled Potato plugin: {} v{}", meta.name, meta.version);
            Ok(())
        })
    }

    fn on_unload(&self, _context: Arc<PumpkinContext>) -> PluginFuture<'_, Result<(), String>> {
        Box::pin(async move {
            if let Some(ctx) = self.potato_context.get() {
                self.inner.on_disable(ctx)?;
            }
            if let Some(host) = self.host_context.get() {
                let handles: Vec<_> = host.task_handles.lock().unwrap().drain().map(|(_, h)| h).collect();
                for h in handles {
                    h.abort();
                }
            }
            Ok(())
        })
    }
}
