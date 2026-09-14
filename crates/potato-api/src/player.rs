use std::sync::Arc;
use uuid::Uuid;

use crate::host::HostPlayer;
use crate::types::{GameMode, Inventory, ItemStack, Location, PotionEffect};

/// Safe, high-level abstraction representing an online player on the PotatoMC server.
#[derive(Clone)]
pub struct Player {
    pub(crate) handle: Arc<dyn HostPlayer>,
}

impl Player {
    pub fn from_handle(handle: Arc<dyn HostPlayer>) -> Self {
        Self { handle }
    }

    pub fn inner(&self) -> &Arc<dyn HostPlayer> {
        &self.handle
    }

    /// Returns the unique UUID of this player.
    pub fn uuid(&self) -> Uuid {
        self.handle.uuid()
    }

    /// Returns the player's username.
    pub fn name(&self) -> String {
        self.handle.name()
    }

    /// Returns the current location of the player.
    pub fn location(&self) -> Location {
        self.handle.location()
    }

    /// Returns the world identifier (e.g. "minecraft:overworld") the player is currently in.
    pub fn world_id(&self) -> String {
        self.handle.world_id()
    }

    /// Teleports the player to the specified target location.
    pub fn teleport(&self, location: &Location) -> bool {
        self.handle.teleport(location)
    }

    /// Sends a system chat message to the player.
    pub fn send_message(&self, message: &str) {
        self.handle.send_message(message);
    }

    /// Accesses the player's inventory.
    pub fn inventory(&self) -> Inventory {
        Inventory::from_handle(self.handle.inventory())
    }

    /// Checks whether the player has a specific permission node.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.handle.has_permission(permission)
    }

    /// Returns the player's current game mode.
    pub fn gamemode(&self) -> GameMode {
        self.handle.gamemode()
    }

    /// Sets the player's game mode.
    pub fn set_gamemode(&self, mode: GameMode) {
        self.handle.set_gamemode(mode);
    }

    /// Returns the player's current health points.
    pub fn health(&self) -> f32 {
        self.handle.health()
    }

    /// Sets the player's current health points.
    pub fn set_health(&self, health: f32) {
        self.handle.set_health(health);
    }

    /// Returns the player's maximum health points.
    pub fn max_health(&self) -> f32 {
        self.handle.max_health()
    }

    /// Returns the player's current food level (0-20).
    pub fn food_level(&self) -> u32 {
        self.handle.food_level()
    }

    /// Sets the player's food level (0-20).
    pub fn set_food_level(&self, food: u32) {
        self.handle.set_food_level(food);
    }

    /// Checks whether the player is sneaking.
    pub fn is_sneaking(&self) -> bool {
        self.handle.is_sneaking()
    }

    /// Checks whether the player is sprinting.
    pub fn is_sprinting(&self) -> bool {
        self.handle.is_sprinting()
    }

    /// Checks whether the player is currently flying.
    pub fn is_flying(&self) -> bool {
        self.handle.is_flying()
    }

    /// Sets whether the player is flying.
    pub fn set_flying(&self, flying: bool) {
        self.handle.set_flying(flying);
    }

    /// Checks whether the player has permission/ability to fly.
    pub fn can_fly(&self) -> bool {
        self.handle.can_fly()
    }

    /// Sets whether the player is allowed to fly.
    pub fn set_can_fly(&self, can_fly: bool) {
        self.handle.set_can_fly(can_fly);
    }

    /// Returns the player's network ping latency in milliseconds.
    pub fn ping(&self) -> u32 {
        self.handle.ping()
    }

    /// Returns the player's current experience level.
    pub fn level(&self) -> i32 {
        self.handle.level()
    }

    /// Sets the player's experience level.
    pub fn set_level(&self, level: i32) {
        self.handle.set_level(level);
    }

    /// Returns the player's total experience points.
    pub fn exp(&self) -> i32 {
        self.handle.exp()
    }

    /// Adds experience points to the player.
    pub fn give_exp(&self, exp: i32) {
        self.handle.give_exp(exp);
    }

    /// Displays a title and subtitle on the player's screen with fade timing.
    pub fn send_title(&self, title: &str, subtitle: &str, fade_in_ticks: u32, stay_ticks: u32, fade_out_ticks: u32) {
        self.handle.send_title(title, subtitle, fade_in_ticks, stay_ticks, fade_out_ticks);
    }

    /// Displays an action bar message above the player's hotbar.
    pub fn send_action_bar(&self, message: &str) {
        self.handle.send_action_bar(message);
    }

    /// Plays a sound effect to the player.
    pub fn play_sound(&self, sound: &str, volume: f32, pitch: f32) {
        self.handle.play_sound(sound, volume, pitch);
    }

    /// Disconnects the player with a custom kick message.
    pub fn kick(&self, reason: &str) {
        self.handle.kick(reason);
    }

    /// Sets the player's tab list header and footer text.
    pub fn set_player_list_header_footer(&self, header: &str, footer: &str) {
        self.handle.set_player_list_header_footer(header, footer);
    }

    /// Sends a rich Adventure text component to the player.
    pub fn send_component(&self, component: &crate::text::Component) {
        self.send_message(&component.to_legacy_string());
    }

    /// Sends a rich Adventure text action bar to the player.
    pub fn send_action_bar_component(&self, component: &crate::text::Component) {
        self.send_action_bar(&component.to_legacy_string());
    }

    /// Sends a rich Adventure text title & subtitle with animation timings to the player.
    pub fn send_title_components(
        &self,
        title: &crate::text::Component,
        subtitle: &crate::text::Component,
        fade_in_ticks: u32,
        stay_ticks: u32,
        fade_out_ticks: u32,
    ) {
        self.send_title(
            &title.to_legacy_string(),
            &subtitle.to_legacy_string(),
            fade_in_ticks,
            stay_ticks,
            fade_out_ticks,
        );
    }

    /// Drops an item stack into the world at the player's position.
    pub fn drop_item(&self, item: &ItemStack) {
        self.handle.drop_item(item);
    }

    /// Deposits an item into the player's inventory, syncing with the client.
    /// Returns true if successfully inserted, false if the inventory is completely full.
    pub fn give_item(&self, item: &ItemStack) -> bool {
        self.handle.give_item(item)
    }

    /// Opens a custom virtual GUI container screen for the player.
    /// Returns the opened screen synchronization ID.
    pub fn open_gui(&self, gui: &crate::gui::Gui) -> u8 {
        let items: Vec<(usize, ItemStack)> = gui
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, opt)| opt.as_ref().map(|it| (idx, it.clone())))
            .collect();
        self.handle.open_gui(&gui.title, gui.size, &items, gui.allow_grab_items, gui.allow_put_items)
    }

    /// Closes any currently open container GUI screen for the player.
    pub fn close_inventory(&self) {
        self.handle.close_inventory();
    }

    /// Applies an active potion effect to the player.
    pub fn add_potion_effect(&self, effect: &PotionEffect) {
        self.handle.add_potion_effect(effect);
    }

    /// Removes a specific potion effect by name or namespaced identifier.
    pub fn remove_potion_effect(&self, effect_type: &str) {
        self.handle.remove_potion_effect(effect_type);
    }

    /// Removes all active potion effects from the player.
    pub fn clear_potion_effects(&self) {
        self.handle.clear_potion_effects();
    }

    /// Checks if the player currently has the specified potion effect.
    pub fn has_potion_effect(&self, effect_type: &str) -> bool {
        self.handle.has_potion_effect(effect_type)
    }

    /// Spawns particle effects visible to this player.
    pub fn spawn_particle(
        &self,
        particle: &str,
        location: &Location,
        count: u32,
        offset: (f64, f64, f64),
        speed: f32,
    ) {
        self.handle.spawn_particle(particle, location, count, offset.0, offset.1, offset.2, speed);
    }

    /// Returns the player's current saturation level.
    pub fn saturation(&self) -> f32 {
        self.handle.saturation()
    }

    /// Sets the player's current saturation level.
    pub fn set_saturation(&self, saturation: f32) {
        self.handle.set_saturation(saturation);
    }

    /// Returns the player's exhaustion level.
    pub fn exhaustion(&self) -> f32 {
        self.handle.exhaustion()
    }

    /// Sets the player's exhaustion level.
    pub fn set_exhaustion(&self, exhaustion: f32) {
        self.handle.set_exhaustion(exhaustion);
    }

    /// Returns the player's experience progress toward next level (0.0 - 1.0).
    pub fn exp_progress(&self) -> f32 {
        self.handle.exp_progress()
    }

    /// Sets the player's experience progress toward next level (0.0 - 1.0).
    pub fn set_exp_progress(&self, progress: f32) {
        self.handle.set_exp_progress(progress);
    }

    /// Checks whether the player is a server operator.
    pub fn is_op(&self) -> bool {
        self.handle.is_op()
    }

    /// Sets whether the player is an operator.
    pub fn set_op(&self, op: bool) {
        self.handle.set_op(op);
    }

    /// Returns the client's locale (e.g. "en_us").
    pub fn locale(&self) -> String {
        self.handle.locale()
    }

    /// Returns the client brand name reported upon handshake (e.g. "vanilla", "fabric").
    pub fn client_brand(&self) -> String {
        self.handle.client_brand()
    }

    /// Returns the player's current walking speed.
    pub fn walk_speed(&self) -> f32 {
        self.handle.walk_speed()
    }

    /// Sets the player's walking speed (default ~0.2).
    pub fn set_walk_speed(&self, speed: f32) {
        self.handle.set_walk_speed(speed);
    }

    /// Returns the player's flying speed.
    pub fn fly_speed(&self) -> f32 {
        self.handle.fly_speed()
    }

    /// Sets the player's flying speed (default ~0.1).
    pub fn set_fly_speed(&self, speed: f32) {
        self.handle.set_fly_speed(speed);
    }

    /// Clears any currently active title display from the player's screen.
    pub fn clear_title(&self) {
        self.handle.clear_title();
    }

    /// Resets the title timings back to vanilla defaults.
    pub fn reset_title(&self) {
        self.handle.reset_title();
    }

    /// Plays a sound to the player using a specific sound category.
    pub fn play_sound_category(
        &self,
        sound: &str,
        category: crate::sound::SoundCategory,
        volume: f32,
        pitch: f32,
    ) {
        self.handle.play_sound_category(sound, category as u8, volume, pitch);
    }

    /// Stops playing a specific sound (or all sounds if None).
    pub fn stop_sound(&self, sound: Option<&str>) {
        self.handle.stop_sound(sound);
    }

    /// Opens a written book interface for the player without requiring an item in hand.
    pub fn open_book(&self, book: &crate::dialog::Book) {
        self.handle.open_book(&book.title, &book.author, &book.pages);
    }

    /// Opens the sign editor dialog UI for a placed sign at the given location.
    pub fn open_sign_editor(&self, location: &Location) {
        self.handle.open_sign_editor(location);
    }

    /// Shows a native Minecraft 1.21.4+ Dialog Box to the player.
    pub fn show_dialog(&self, dialog: &crate::dialog::Dialog) {
        if let Ok(json) = serde_json::to_string(dialog) {
            self.handle.show_dialog_raw(&json);
        }
    }

    /// Clears any active dialog box currently displayed on the player's screen.
    pub fn clear_dialog(&self) {
        self.handle.clear_dialog();
    }

    /// Sends a Bedrock / Crossplay Form dialog directly to the player.
    pub fn send_form(&self, form_id: u32, form_json: &str) {
        self.handle.send_form_raw(form_id, form_json);
    }

    /// Convenience method to send a Bedrock SimpleForm dialog.
    pub fn send_simple_form(&self, form_id: u32, form: &crate::dialog::SimpleForm) {
        if let Ok(json) = form.to_json() {
            self.send_form(form_id, &json);
        }
    }

    /// Convenience method to send a Bedrock ModalForm dialog.
    pub fn send_modal_form(&self, form_id: u32, form: &crate::dialog::ModalForm) {
        if let Ok(json) = form.to_json() {
            self.send_form(form_id, &json);
        }
    }

    /// Convenience method to send a Bedrock CustomForm dialog.
    pub fn send_custom_form(&self, form_id: u32, form: &crate::dialog::CustomForm) {
        if let Ok(json) = form.to_json() {
            self.send_form(form_id, &json);
        }
    }

    /// Prompts the player to download and apply a server resource pack.
    pub fn send_resource_pack(&self, url: &str, hash: &str, required: bool, prompt: Option<&str>) {
        self.handle.send_resource_pack(url, hash, required, prompt);
    }

    /// Hides another player from this player's client (Paper Vanish API).
    pub fn hide_player(&self, other: &Player) {
        self.handle.hide_player(&other.uuid());
    }

    /// Restores visibility of a hidden player for this player's client.
    pub fn show_player(&self, other: &Player) {
        self.handle.show_player(&other.uuid());
    }

    /// Checks if this player can currently see the specified other player.
    pub fn can_see(&self, other: &Player) -> bool {
        self.handle.can_see(&other.uuid())
    }

    /// Forces a player respawn.
    pub fn respawn(&self) {
        self.handle.respawn();
    }

    /// Displays a custom Scoreboard to the player.
    pub fn set_scoreboard(&self, scoreboard: &crate::scoreboard::Scoreboard) {
        self.handle.set_scoreboard_lines(&scoreboard.title, &scoreboard.lines);
        if let Ok(json) = serde_json::to_string(scoreboard) {
            self.handle.set_player_scoreboard(&json);
        }
    }

    /// Clears the scoreboard / sidebar for this player.
    pub fn clear_scoreboard(&self) {
        self.handle.clear_scoreboard();
    }

    /// Resets the player's scoreboard back to the main server scoreboard.
    pub fn reset_scoreboard(&self) {
        self.handle.reset_player_scoreboard();
    }

    /// Dynamically grants or denies a permission node for this player.
    pub fn set_permission(&self, node: &str, value: bool) {
        self.handle.set_permission(node, value);
    }

    /// Removes a player-specific override for this permission node.
    pub fn unset_permission(&self, node: &str) {
        self.handle.unset_permission(node);
    }

    /// Fast helper to render a sidebar scoreboard with given title and lines.
    pub fn set_sidebar_lines(&self, title: &str, lines: &[impl AsRef<str>]) {
        let mut sb = crate::scoreboard::Scoreboard::sidebar(title);
        sb.set_lines(lines);
        self.set_scoreboard(&sb);
    }

    /// Sends a plugin messaging packet (BungeeCord / Velocity / custom channel).
    pub fn send_plugin_message(&self, channel: &str, data: &[u8]) {
        self.handle.send_plugin_message(channel, data);
    }

    /// Returns the player's eye location.
    pub fn eye_location(&self) -> Location {
        let mut loc = self.location();
        loc.y += 1.62;
        loc
    }

    /// Returns a unit Vector3 representing the direction the player is looking.
    pub fn facing_direction(&self) -> crate::types::Vector3 {
        let loc = self.location();
        crate::types::Vector3::from_yaw_pitch(loc.yaw, loc.pitch)
    }

    /// Sets an item cooldown in ticks (renders grey wipe overlay on client).
    pub fn set_item_cooldown(&self, item_id: &str, ticks: u32) {
        self.handle.set_item_cooldown(item_id, ticks);
    }

    /// Returns the remaining cooldown ticks for an item, or 0 if inactive.
    pub fn get_item_cooldown(&self, item_id: &str) -> u32 {
        self.handle.get_item_cooldown(item_id)
    }

    /// Checks whether an item is currently on cooldown for this player.
    pub fn has_item_cooldown(&self, item_id: &str) -> bool {
        self.handle.has_item_cooldown(item_id)
    }

    /// Displays an advancement toast notification in the corner of the player's screen.
    pub fn send_toast(&self, toast: &crate::toast::Toast) {
        self.handle.send_toast(&toast.title, &toast.icon, toast.frame as u8);
    }

    /// Sends the Minecraft Demo Welcome screen popup to the player.
    pub fn send_demo_screen(&self) {
        self.handle.send_demo_screen();
    }

    /// Sends a native client game event packet (e.g. rain, elder guardian, credits).
    pub fn send_game_event(&self, event: crate::game_event::GameEvent) {
        let (id, val) = event.id_and_value();
        self.handle.send_game_event(id, val);
    }

    /// Raytraces from the player's eye along their line of sight to find the targeted block.
    pub fn get_target_block(&self, max_distance: f64) -> Option<crate::types::Block> {
        self.handle.get_target_block(max_distance)
    }

    /// Raytraces from the player's eye along their line of sight to find the targeted entity.
    pub fn get_target_entity(&self, max_distance: f64) -> Option<crate::entity::Entity> {
        self.handle.get_target_entity(max_distance).map(crate::entity::Entity::from_handle)
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("uuid", &self.uuid())
            .field("name", &self.name())
            .field("location", &self.location())
            .finish()
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.uuid() == other.uuid()
    }
}

impl Eq for Player {}

impl std::hash::Hash for Player {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid().hash(state);
    }
}
