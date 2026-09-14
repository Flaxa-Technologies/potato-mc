use std::sync::Arc;
use uuid::Uuid;

use crate::types::{Block, Difficulty, GameMode, ItemStack, Location, PotionEffect, Vector3};
pub use crate::types::HostInventory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub trait HostPlayer: Send + Sync {
    fn uuid(&self) -> Uuid;
    fn name(&self) -> String;
    fn location(&self) -> Location;
    fn world_id(&self) -> String;
    fn teleport(&self, location: &Location) -> bool;
    fn send_message(&self, message: &str);
    fn inventory(&self) -> Arc<dyn HostInventory>;
    fn has_permission(&self, permission: &str) -> bool;

    fn gamemode(&self) -> GameMode;
    fn set_gamemode(&self, mode: GameMode);
    fn health(&self) -> f32;
    fn set_health(&self, health: f32);
    fn max_health(&self) -> f32;
    fn food_level(&self) -> u32;
    fn set_food_level(&self, food: u32);
    fn is_sneaking(&self) -> bool;
    fn is_sprinting(&self) -> bool;
    fn is_flying(&self) -> bool;
    fn set_flying(&self, flying: bool);
    fn can_fly(&self) -> bool;
    fn set_can_fly(&self, can_fly: bool);
    fn ping(&self) -> u32;
    fn level(&self) -> i32;
    fn set_level(&self, level: i32);
    fn exp(&self) -> i32;
    fn give_exp(&self, exp: i32);
    fn send_title(&self, title: &str, subtitle: &str, fade_in_ticks: u32, stay_ticks: u32, fade_out_ticks: u32);
    fn send_action_bar(&self, message: &str);
    fn play_sound(&self, sound: &str, volume: f32, pitch: f32);
    fn kick(&self, reason: &str);
    fn set_player_list_header_footer(&self, header: &str, footer: &str);

    fn drop_item(&self, _item: &ItemStack) {}
    fn give_item(&self, _item: &ItemStack) -> bool { false }
    fn open_gui(&self, _title: &str, _size: usize, _items: &[(usize, ItemStack)], _allow_grab: bool, _allow_put: bool) -> u8 { 0 }
    fn close_inventory(&self) {}
    fn add_potion_effect(&self, _effect: &PotionEffect) {}
    fn remove_potion_effect(&self, _effect_type: &str) {}
    fn clear_potion_effects(&self) {}
    fn has_potion_effect(&self, _effect_type: &str) -> bool { false }
    fn spawn_particle(&self, _particle: &str, _location: &Location, _count: u32, _offset_x: f64, _offset_y: f64, _offset_z: f64, _speed: f32) {}

    // Paper / Bukkit / Dialog expansions
    fn saturation(&self) -> f32 { 5.0 }
    fn set_saturation(&self, _saturation: f32) {}
    fn exhaustion(&self) -> f32 { 0.0 }
    fn set_exhaustion(&self, _exhaustion: f32) {}
    fn exp_progress(&self) -> f32 { 0.0 }
    fn set_exp_progress(&self, _progress: f32) {}
    fn is_op(&self) -> bool { false }
    fn set_op(&self, _op: bool) {}
    fn locale(&self) -> String { "en_us".to_string() }
    fn client_brand(&self) -> String { "vanilla".to_string() }
    fn walk_speed(&self) -> f32 { 0.2 }
    fn set_walk_speed(&self, _speed: f32) {}
    fn fly_speed(&self) -> f32 { 0.1 }
    fn set_fly_speed(&self, _speed: f32) {}
    fn clear_title(&self) {}
    fn reset_title(&self) {}
    fn play_sound_category(&self, sound: &str, _category: u8, volume: f32, pitch: f32) {
        self.play_sound(sound, volume, pitch);
    }
    fn stop_sound(&self, _sound: Option<&str>) {}
    fn open_book(&self, _title: &str, _author: &str, _pages: &[String]) {}
    fn open_sign_editor(&self, _location: &Location) {}
    fn show_dialog_raw(&self, _dialog_json: &str) {}
    fn clear_dialog(&self) {}
    fn send_form_raw(&self, _form_id: u32, _form_json: &str) {}
    fn send_resource_pack(&self, _url: &str, _hash: &str, _required: bool, _prompt: Option<&str>) {}
    fn hide_player(&self, _target: &Uuid) {}
    fn show_player(&self, _target: &Uuid) {}
    fn can_see(&self, _target: &Uuid) -> bool { true }
    fn respawn(&self) {}
    fn set_scoreboard_lines(&self, _title: &str, _lines: &[(usize, String)]) {}
    fn clear_scoreboard(&self) {}
    fn send_plugin_message(&self, _channel: &str, _data: &[u8]) {}

    // Toast, Cooldown, Raytracing, Game Events
    fn set_item_cooldown(&self, _item_id: &str, _ticks: u32) {}
    fn get_item_cooldown(&self, _item_id: &str) -> u32 { 0 }
    fn has_item_cooldown(&self, item_id: &str) -> bool { self.get_item_cooldown(item_id) > 0 }
    fn send_toast(&self, _title: &str, _icon: &str, _frame: u8) {}
    fn send_demo_screen(&self) {}
    fn send_game_event(&self, _event_type: u8, _value: f32) {}
    fn get_target_block(&self, _max_distance: f64) -> Option<Block> { None }
    fn get_target_entity(&self, _max_distance: f64) -> Option<Arc<dyn HostEntity>> { None }
    fn set_player_scoreboard(&self, _scoreboard_json: &str) {}
    fn reset_player_scoreboard(&self) {}
    fn set_permission(&self, _node: &str, _value: bool) {}
    fn unset_permission(&self, _node: &str) {}
}

pub trait HostWorld: Send + Sync {
    fn identity(&self) -> String;
    fn get_block(&self, x: i32, y: i32, z: i32) -> Option<Block>;
    fn set_block(&self, x: i32, y: i32, z: i32, block: &Block) -> bool;
    fn spawn_entity(&self, entity_type: &str, location: &Location) -> Result<Arc<dyn HostEntity>, String>;
    fn player_lookup(&self, name: &str) -> Option<Arc<dyn HostPlayer>>;
    fn players(&self) -> Vec<Arc<dyn HostPlayer>>;

    fn time(&self) -> u64;
    fn set_time(&self, time: u64);
    fn is_raining(&self) -> bool;
    fn set_storm(&self, storm: bool);
    fn difficulty(&self) -> Difficulty;
    fn set_difficulty(&self, diff: Difficulty);
    fn create_explosion(&self, x: f64, y: f64, z: f64, power: f32, fire: bool, break_blocks: bool);
    fn play_sound(&self, location: &Location, sound: &str, volume: f32, pitch: f32);
    fn broadcast_message(&self, message: &str);

    fn drop_item(&self, _location: &Location, _item: &ItemStack) {}
    fn drop_item_naturally(&self, location: &Location, item: &ItemStack) {
        self.drop_item(location, item);
    }
    fn break_block(&self, x: i32, y: i32, z: i32, _drop_items: bool) -> bool {
        self.set_block(x, y, z, &Block::new("minecraft:air", 0))
    }
    fn spawn_particle(&self, _particle: &str, _location: &Location, _count: u32, _offset_x: f64, _offset_y: f64, _offset_z: f64, _speed: f32) {}
    fn strike_lightning(&self, _location: &Location) {}
    fn strike_lightning_effect(&self, _location: &Location) {}
    fn get_highest_block_y(&self, _x: i32, _z: i32) -> i32 { 64 }
    fn is_thundering(&self) -> bool { false }
    fn set_thundering(&self, _thundering: bool) {}
    fn play_sound_category(&self, location: &Location, sound: &str, _category: u8, volume: f32, pitch: f32) {
        self.play_sound(location, sound, volume, pitch);
    }
}

pub trait HostEntity: Send + Sync {
    fn uuid(&self) -> Uuid;
    fn entity_type(&self) -> String;
    fn location(&self) -> Location;
    fn teleport(&self, location: &Location) -> bool;
    fn velocity(&self) -> Vector3;
    fn set_velocity(&self, velocity: &Vector3);
    fn remove(&self);
    fn as_living(&self) -> Option<Arc<dyn HostLivingEntity>>;

    fn is_on_ground(&self) -> bool;
    fn custom_name(&self) -> Option<String>;
    fn set_custom_name(&self, name: Option<&str>);
    fn fire_ticks(&self) -> i32;
    fn set_fire_ticks(&self, ticks: i32);
    fn damage(&self, amount: f32);

    fn is_glowing(&self) -> bool { false }
    fn set_glowing(&self, _glowing: bool) {}
    fn is_invulnerable(&self) -> bool { false }
    fn set_invulnerable(&self, _invulnerable: bool) {}
    fn is_silent(&self) -> bool { false }
    fn set_silent(&self, _silent: bool) {}
    fn has_gravity(&self) -> bool { true }
    fn set_gravity(&self, _gravity: bool) {}
    fn scoreboard_tags(&self) -> Vec<String> { Vec::new() }
    fn add_scoreboard_tag(&self, _tag: &str) -> bool { false }
    fn remove_scoreboard_tag(&self, _tag: &str) -> bool { false }
    fn persistent_data_json(&self) -> String { "{}".to_string() }
    fn set_persistent_data_json(&self, _json: &str) {}
}

pub trait HostLivingEntity: Send + Sync {
    fn base_entity(&self) -> Arc<dyn HostEntity>;
    fn health(&self) -> f32;
    fn set_health(&self, health: f32);
    fn max_health(&self) -> f32;
    fn eye_location(&self) -> Location { self.base_entity().location() }
    fn eye_height(&self) -> f64 { 1.62 }
}

pub trait HostConsole: Send + Sync {
    fn send_message(&self, message: &str);
}

pub trait RawEventListener: Send + Sync {
    fn handle_raw(&self, event_id: u32, event_ptr: *mut ());
}

pub trait HostContext: Send + Sync {
    fn plugin_name(&self) -> &str;
    fn log(&self, level: LogLevel, message: &str);

    fn run_task(&self, task: Box<dyn FnOnce() + Send>);
    fn run_task_later(&self, delay_millis: u64, task: Box<dyn FnOnce() + Send>) -> u64;
    fn run_task_repeating(&self, initial_delay_millis: u64, period_millis: u64, task: Box<dyn FnMut() + Send>) -> u64;
    fn cancel_task(&self, task_id: u64);

    fn register_event_listener(&self, event_id: u32, listener: Arc<dyn RawEventListener>);
    fn register_command(&self, command: crate::command::CommandNode);

    fn get_player_by_uuid(&self, uuid: &Uuid) -> Option<Arc<dyn HostPlayer>>;
    fn get_player_by_name(&self, name: &str) -> Option<Arc<dyn HostPlayer>>;
    fn get_online_players(&self) -> Vec<Arc<dyn HostPlayer>>;
    fn get_world(&self, name: &str) -> Option<Arc<dyn HostWorld>>;
    fn get_worlds(&self) -> Vec<Arc<dyn HostWorld>>;
    fn data_folder(&self) -> std::path::PathBuf;

    fn broadcast(&self, message: &str);
    fn server_version(&self) -> &str;

    fn get_tps(&self) -> f64 { 20.0 }
    fn max_players(&self) -> u32 { 100 }
    fn dispatch_command(&self, _command: &str) -> bool { false }
    fn server_motd(&self) -> String { "A PotatoMC Server".to_string() }
    fn set_server_motd(&self, _motd: &str) {}
    fn shutdown(&self) {}
    fn reload(&self) {}
    fn send_plugin_message(&self, _player: &Uuid, _channel: &str, _data: &[u8]) {}
    fn spawn_npc(&self, _name: &str, _entity_type: &str, _location: &Location, _pose: u32, _skin: Option<(&str, Option<&str>)>, _glowing: bool) -> Option<Arc<dyn crate::npc::HostNpc>> { None }
    fn register_permission(&self, _node: &str, _description: Option<&str>, _default_op: bool) {}
}
