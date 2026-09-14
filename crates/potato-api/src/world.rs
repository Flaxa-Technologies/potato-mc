use std::sync::Arc;

use crate::entity::Entity;
use crate::host::HostWorld;
use crate::player::Player;
use crate::types::{Block, Difficulty, ItemStack, Location};

/// Safe abstraction representing a loaded dimension/world on the PotatoMC server.
#[derive(Clone)]
pub struct World {
    pub(crate) handle: Arc<dyn HostWorld>,
}

impl World {
    pub fn from_handle(handle: Arc<dyn HostWorld>) -> Self {
        Self { handle }
    }

    pub fn inner(&self) -> &Arc<dyn HostWorld> {
        &self.handle
    }

    /// Returns the unique dimension/world identifier, e.g. "minecraft:overworld".
    pub fn identity(&self) -> String {
        self.handle.identity()
    }

    /// Retrieves the block at the specified world block coordinates.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<Block> {
        self.handle.get_block(x, y, z)
    }

    /// Sets the block state at the specified world block coordinates.
    pub fn set_block(&self, x: i32, y: i32, z: i32, block: &Block) -> bool {
        self.handle.set_block(x, y, z, block)
    }

    /// Spawns an entity of the given type at the target location.
    pub fn spawn_entity(&self, entity_type: &str, location: &Location) -> Result<Entity, String> {
        self.handle
            .spawn_entity(entity_type, location)
            .map(Entity::from_handle)
    }

    /// Searches for an online player in this world by their username.
    pub fn player_lookup(&self, name: &str) -> Option<Player> {
        self.handle.player_lookup(name).map(Player::from_handle)
    }

    /// Returns a list of all players currently present in this world.
    pub fn players(&self) -> Vec<Player> {
        self.handle
            .players()
            .into_iter()
            .map(Player::from_handle)
            .collect()
    }

    /// Returns the world time of day in game ticks (0-24000).
    pub fn time(&self) -> u64 {
        self.handle.time()
    }

    /// Sets the world time of day in game ticks.
    pub fn set_time(&self, time: u64) {
        self.handle.set_time(time);
    }

    /// Checks whether it is currently raining/storming in this world.
    pub fn is_raining(&self) -> bool {
        self.handle.is_raining()
    }

    /// Sets whether it is raining/storming in this world.
    pub fn set_storm(&self, storm: bool) {
        self.handle.set_storm(storm);
    }

    /// Returns the current world difficulty.
    pub fn difficulty(&self) -> Difficulty {
        self.handle.difficulty()
    }

    /// Sets the world difficulty.
    pub fn set_difficulty(&self, diff: Difficulty) {
        self.handle.set_difficulty(diff);
    }

    /// Creates an explosion at the specified location.
    pub fn create_explosion(&self, x: f64, y: f64, z: f64, power: f32, fire: bool, break_blocks: bool) {
        self.handle.create_explosion(x, y, z, power, fire, break_blocks);
    }

    /// Plays a sound effect at a specific world location.
    pub fn play_sound(&self, location: &Location, sound: &str, volume: f32, pitch: f32) {
        self.handle.play_sound(location, sound, volume, pitch);
    }

    /// Broadcasts a system chat message to all players in this world.
    pub fn broadcast_message(&self, message: &str) {
        self.handle.broadcast_message(message);
    }

    /// Drops an item stack into this world at the specified location.
    pub fn drop_item(&self, location: &Location, item: &ItemStack) {
        self.handle.drop_item(location, item);
    }

    /// Drops an item stack into this world at the specified location with slight randomized velocity.
    pub fn drop_item_naturally(&self, location: &Location, item: &ItemStack) {
        self.handle.drop_item_naturally(location, item);
    }

    /// Breaks the block at the specified coordinate, optionally dropping its vanilla items.
    pub fn break_block(&self, x: i32, y: i32, z: i32, drop_items: bool) -> bool {
        self.handle.break_block(x, y, z, drop_items)
    }

    /// Spawns particle effects at the specified location in this world.
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

    /// Strikes a real lightning bolt at the given location (creates fire and damage).
    pub fn strike_lightning(&self, location: &Location) {
        self.handle.strike_lightning(location);
    }

    /// Strikes a visual / sound effect lightning bolt without causing fire or damage.
    pub fn strike_lightning_effect(&self, location: &Location) {
        self.handle.strike_lightning_effect(location);
    }

    /// Returns the highest non-air block Y coordinate at the given horizontal X/Z coordinate.
    pub fn get_highest_block_y(&self, x: i32, z: i32) -> i32 {
        self.handle.get_highest_block_y(x, z)
    }

    /// Checks whether it is currently thundering in this world.
    pub fn is_thundering(&self) -> bool {
        self.handle.is_thundering()
    }

    /// Sets whether it is thundering in this world.
    pub fn set_thundering(&self, thundering: bool) {
        self.handle.set_thundering(thundering);
    }

    /// Plays a sound effect at a specific world location with sound category.
    pub fn play_sound_category(
        &self,
        location: &Location,
        sound: &str,
        category: crate::sound::SoundCategory,
        volume: f32,
        pitch: f32,
    ) {
        self.handle.play_sound_category(location, sound, category as u8, volume, pitch);
    }
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("identity", &self.identity())
            .finish()
    }
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for World {}
