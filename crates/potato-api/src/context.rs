use std::sync::Arc;
use uuid::Uuid;

use crate::command::CommandNode;
use crate::event::{Event, EventHandler};
use crate::host::{HostContext, LogLevel, RawEventListener};
use crate::player::Player;
use crate::scheduler::Scheduler;
use crate::world::World;

/// Lightweight logger utility for plugins.
#[derive(Clone)]
pub struct Logger {
    host: Arc<dyn HostContext>,
}

impl Logger {
    pub fn info(&self, msg: impl std::fmt::Display) {
        self.host.log(LogLevel::Info, &msg.to_string());
    }

    pub fn warn(&self, msg: impl std::fmt::Display) {
        self.host.log(LogLevel::Warn, &msg.to_string());
    }

    pub fn error(&self, msg: impl std::fmt::Display) {
        self.host.log(LogLevel::Error, &msg.to_string());
    }

    pub fn debug(&self, msg: impl std::fmt::Display) {
        self.host.log(LogLevel::Debug, &msg.to_string());
    }

    pub fn trace(&self, msg: impl std::fmt::Display) {
        self.host.log(LogLevel::Trace, &msg.to_string());
    }
}

/// The core interaction context passed to a plugin during its lifecycle.
#[derive(Clone)]
pub struct PluginContext {
    host: Arc<dyn HostContext>,
    scheduler: Scheduler,
    logger: Logger,
}

impl PluginContext {
    pub fn new(host: Arc<dyn HostContext>) -> Self {
        let scheduler = Scheduler::new(host.clone());
        let logger = Logger { host: host.clone() };
        Self {
            host,
            scheduler,
            logger,
        }
    }

    /// Returns the name of the active plugin.
    pub fn plugin_name(&self) -> &str {
        self.host.plugin_name()
    }

    /// Accesses the plugin logger.
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Accesses the task scheduler.
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// Registers a new command with the server.
    pub fn register_command(&self, command: CommandNode) {
        self.host.register_command(command);
    }

    /// Registers an event handler for a specific event type.
    pub fn register_event<E: Event, H: EventHandler<E>>(&self, handler: H) {
        let listener = Arc::new(EventAdapter {
            handler,
            _phantom: std::marker::PhantomData,
        });
        self.host.register_event_listener(E::EVENT_ID, listener);
    }

    /// Finds an online player by their UUID.
    pub fn get_player(&self, uuid: &Uuid) -> Option<Player> {
        self.host.get_player_by_uuid(uuid).map(Player::from_handle)
    }

    /// Finds an online player by their username.
    pub fn get_player_by_name(&self, name: &str) -> Option<Player> {
        self.host.get_player_by_name(name).map(Player::from_handle)
    }

    /// Returns a list of all currently connected players across all worlds.
    pub fn get_online_players(&self) -> Vec<Player> {
        self.host
            .get_online_players()
            .into_iter()
            .map(Player::from_handle)
            .collect()
    }

    /// Looks up a world by its identifier (e.g. "minecraft:overworld").
    pub fn get_world(&self, name: &str) -> Option<World> {
        self.host.get_world(name).map(World::from_handle)
    }

    /// Returns all worlds loaded on the server.
    pub fn get_worlds(&self) -> Vec<World> {
        self.host
            .get_worlds()
            .into_iter()
            .map(World::from_handle)
            .collect()
    }

    /// Returns the dedicated data folder for this plugin (e.g. `plugins/MyPlugin/`).
    pub fn data_folder(&self) -> std::path::PathBuf {
        self.host.data_folder()
    }

    /// Loads the plugin's `config.yml` configuration file, or an empty config if missing.
    pub fn config(&self) -> crate::config::Config {
        let path = self.data_folder().join("config.yml");
        crate::config::Config::from_file(&path).unwrap_or_default()
    }

    /// Saves the plugin configuration to `config.yml`.
    pub fn save_config(&self, config: &crate::config::Config) -> Result<(), String> {
        let path = self.data_folder().join("config.yml");
        config.save(path)
    }

    /// Saves the default configuration string if `config.yml` does not yet exist.
    pub fn save_default_config(&self, default_content: &str) -> Result<crate::config::Config, String> {
        let path = self.data_folder().join("config.yml");
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&path, default_content).map_err(|e| format!("Failed to write default config: {e}"))?;
        }
        crate::config::Config::from_file(&path)
    }

    /// Broadcasts a chat message to all connected players across the entire server.
    pub fn broadcast(&self, message: &str) {
        self.host.broadcast(message);
    }

    /// Broadcasts a rich Adventure text component to all connected players across the entire server.
    pub fn broadcast_component(&self, component: &crate::text::Component) {
        self.broadcast(&component.to_legacy_string());
    }

    /// Returns the active server implementation and version string.
    pub fn server_version(&self) -> &str {
        self.host.server_version()
    }

    /// Accesses the server management and query interface (Paper's `Bukkit.getServer()`).
    pub fn server(&self) -> Server {
        Server::new(self.host.clone())
    }

    /// Spawns a new NPC in the world using the configured builder.
    pub fn spawn_npc(&self, builder: crate::npc::NpcBuilder) -> crate::npc::Npc {
        self.server().spawn_npc(builder)
    }
}

/// Safe abstraction representing the Minecraft Server instance (Paper's `Bukkit.getServer()`).
#[derive(Clone)]
pub struct Server {
    host: Arc<dyn HostContext>,
}

impl Server {
    pub fn new(host: Arc<dyn HostContext>) -> Self {
        Self { host }
    }

    /// Returns the current server tick rate (target 20.0 TPS).
    pub fn tps(&self) -> f64 {
        self.host.get_tps()
    }

    /// Returns the maximum player slots configured on the server.
    pub fn max_players(&self) -> u32 {
        self.host.max_players()
    }

    /// Returns a list of all currently connected players.
    pub fn online_players(&self) -> Vec<Player> {
        self.host.get_online_players().into_iter().map(Player::from_handle).collect()
    }

    /// Dispatches a command line as the server console.
    pub fn dispatch_command(&self, command: &str) -> bool {
        self.host.dispatch_command(command)
    }

    /// Broadcasts a chat message to all connected players across the entire server.
    pub fn broadcast(&self, message: &str) {
        self.host.broadcast(message);
    }

    /// Broadcasts a rich Adventure text component to all connected players across the entire server.
    pub fn broadcast_component(&self, component: &crate::text::Component) {
        self.broadcast(&component.to_legacy_string());
    }

    /// Returns the server version string.
    pub fn version(&self) -> &str {
        self.host.server_version()
    }

    /// Returns the server Message of the Day (MOTD).
    pub fn motd(&self) -> String {
        self.host.server_motd()
    }

    /// Updates the server Message of the Day (MOTD).
    pub fn set_motd(&self, motd: &str) {
        self.host.set_server_motd(motd);
    }

    /// Initiates graceful server shutdown.
    pub fn shutdown(&self) {
        self.host.shutdown();
    }

    /// Gets a player by their exact username.
    pub fn player_by_name(&self, name: &str) -> Option<Player> {
        self.host.get_player_by_name(name).map(Player::from_handle)
    }

    /// Gets a player by their unique UUID.
    pub fn player_by_uuid(&self, uuid: &uuid::Uuid) -> Option<Player> {
        self.host.get_player_by_uuid(uuid).map(Player::from_handle)
    }

    /// Gets a world by its identifier namespace (e.g. `minecraft:overworld`).
    pub fn world(&self, name: &str) -> Option<crate::world::World> {
        self.host.get_world(name).map(crate::world::World::from_handle)
    }

    /// Gets a list of all loaded worlds on the server.
    pub fn worlds(&self) -> Vec<crate::world::World> {
        self.host.get_worlds().into_iter().map(crate::world::World::from_handle).collect()
    }


    /// Reloads server configurations and reloadable subsystems.
    pub fn reload(&self) {
        self.host.reload();
    }

    /// Sends a plugin messaging packet to the given player.
    pub fn send_plugin_message(&self, player: &Player, channel: &str, data: &[u8]) {
        self.host.send_plugin_message(&player.uuid(), channel, data);
    }

    /// Spawns a new NPC in the world using the configured builder.
    pub fn spawn_npc(&self, mut builder: crate::npc::NpcBuilder) -> crate::npc::Npc {
        let name = builder.get_name().to_string();
        let entity_type = match builder.get_type() {
            crate::npc::NpcType::Player { .. } => "minecraft:player".to_string(),
            crate::npc::NpcType::Entity { entity_type } => entity_type.clone(),
        };
        let loc = builder.get_location().cloned().unwrap_or_else(|| {
            crate::types::Location::new("minecraft:overworld", 0.0, 64.0, 0.0, 0.0, 0.0)
        });
        let pose = builder.get_pose().to_protocol_id();
        let skin_data = match builder.get_type() {
            crate::npc::NpcType::Player { skin: Some(s) } => {
                Some((s.value.clone(), s.signature.clone()))
            }
            _ => None,
        };
        let skin = skin_data.as_ref().map(|(v, sig)| (v.as_str(), sig.as_deref()));
        let glowing = builder.is_glowing();
        let behavior = builder.take_behavior();

        if let Some(host_npc) = self.host.spawn_npc(&name, &entity_type, &loc, pose, skin, glowing) {
            crate::npc::Npc::new(host_npc, behavior)
        } else {
            static NEXT_NPC_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(200_000);
            let id = NEXT_NPC_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            struct FallbackNpc {
                id: u32,
                uuid: uuid::Uuid,
                name: String,
                location: std::sync::RwLock<crate::types::Location>,
                pose: std::sync::RwLock<crate::npc::EntityPose>,
                skin: std::sync::RwLock<Option<crate::npc::SkinData>>,
                glowing: std::sync::atomic::AtomicBool,
                valid: std::sync::atomic::AtomicBool,
            }

            impl crate::npc::HostNpc for FallbackNpc {
                fn id(&self) -> u32 { self.id }
                fn uuid(&self) -> uuid::Uuid { self.uuid }
                fn name(&self) -> String { self.name.clone() }
                fn set_name(&self, _name: &str) {}
                fn location(&self) -> crate::types::Location { self.location.read().unwrap().clone() }
                fn teleport(&self, loc: &crate::types::Location) -> bool {
                    *self.location.write().unwrap() = loc.clone();
                    true
                }
                fn move_to(&self, loc: &crate::types::Location, _speed: f64) {
                    *self.location.write().unwrap() = loc.clone();
                }
                fn pose(&self) -> crate::npc::EntityPose { *self.pose.read().unwrap() }
                fn set_pose(&self, p: crate::npc::EntityPose) { *self.pose.write().unwrap() = p; }
                fn skin(&self) -> Option<crate::npc::SkinData> { self.skin.read().unwrap().clone() }
                fn set_skin(&self, s: crate::npc::SkinData) { *self.skin.write().unwrap() = Some(s); }
                fn is_glowing(&self) -> bool { self.glowing.load(std::sync::atomic::Ordering::Relaxed) }
                fn set_glowing(&self, g: bool) { self.glowing.store(g, std::sync::atomic::Ordering::Relaxed); }
                fn set_equipment(&self, _slot: crate::types::EquipmentSlot, _item: Option<crate::types::ItemStack>) {}
                fn despawn(&self) { self.valid.store(false, std::sync::atomic::Ordering::Relaxed); }
                fn is_valid(&self) -> bool { self.valid.load(std::sync::atomic::Ordering::Relaxed) }
            }

            let fallback = Arc::new(FallbackNpc {
                id,
                uuid: uuid::Uuid::new_v4(),
                name,
                location: std::sync::RwLock::new(loc),
                pose: std::sync::RwLock::new(builder.get_pose()),
                skin: std::sync::RwLock::new(match builder.get_type() {
                    crate::npc::NpcType::Player { skin: Some(s) } => Some(s.clone()),
                    _ => None,
                }),
                glowing: std::sync::atomic::AtomicBool::new(glowing),
                valid: std::sync::atomic::AtomicBool::new(true),
            });
            crate::npc::Npc::new(fallback, behavior)
        }
    }

    /// Registers a permission node with the server.
    pub fn register_permission(&self, permission: crate::permission::Permission) {
        let is_op = permission.default_value == crate::permission::PermissionDefault::Op;
        self.host.register_permission(&permission.name, permission.description.as_deref(), is_op);
    }
}

struct EventAdapter<E: Event, H: EventHandler<E>> {
    handler: H,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, H: EventHandler<E>> RawEventListener for EventAdapter<E, H> {
    fn handle_raw(&self, event_id: u32, event_ptr: *mut ()) {
        if event_id == E::EVENT_ID {
            // SAFETY: Verified event_id matches this adapter's registered event type E.
            let event = unsafe { &mut *(event_ptr.cast::<E>()) };
            self.handler.handle(event);
        }
    }
}
