use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// A 3-dimensional vector of floating-point numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            *self
        }
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn distance_squared(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance(&self, other: &Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn add(&self, other: &Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn multiply(&self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Converts Minecraft yaw and pitch (in degrees) into a normalized unit vector.
    pub fn from_yaw_pitch(yaw_deg: f32, pitch_deg: f32) -> Self {
        let yaw_rad = (yaw_deg as f64).to_radians();
        let pitch_rad = (pitch_deg as f64).to_radians();
        let xz = -pitch_rad.cos();
        Self::new(
            xz * yaw_rad.sin(),
            -pitch_rad.sin(),
            -xz * yaw_rad.cos(),
        )
    }
}

/// Axis-aligned bounding box (AABB) for collision and raytracing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl BoundingBox {
    pub fn new(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            min_z: min_z.min(max_z),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
            max_z: min_z.max(max_z),
        }
    }

    pub fn of_point(pos: Vector3, width: f64, height: f64) -> Self {
        let half_w = width / 2.0;
        Self::new(
            pos.x - half_w,
            pos.y,
            pos.z - half_w,
            pos.x + half_w,
            pos.y + height,
            pos.z + half_w,
        )
    }

    pub fn contains(&self, x: f64, y: f64, z: f64) -> bool {
        x >= self.min_x && x <= self.max_x
            && y >= self.min_y && y <= self.max_y
            && z >= self.min_z && z <= self.max_z
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x && self.max_x >= other.min_x
            && self.min_y <= other.max_y && self.max_y >= other.min_y
            && self.min_z <= other.max_z && self.max_z >= other.min_z
    }

    pub fn expand(&self, dx: f64, dy: f64, dz: f64) -> Self {
        Self::new(
            self.min_x - dx,
            self.min_y - dy,
            self.min_z - dz,
            self.max_x + dx,
            self.max_y + dy,
            self.max_z + dz,
        )
    }

    /// Performs slab-method ray intersection against this AABB.
    /// Returns the distance to the intersection along the ray, if any.
    pub fn raytrace(&self, start: Vector3, dir: Vector3, max_dist: f64) -> Option<f64> {
        let inv_x = if dir.x != 0.0 { 1.0 / dir.x } else { f64::INFINITY };
        let inv_y = if dir.y != 0.0 { 1.0 / dir.y } else { f64::INFINITY };
        let inv_z = if dir.z != 0.0 { 1.0 / dir.z } else { f64::INFINITY };

        let t1 = (self.min_x - start.x) * inv_x;
        let t2 = (self.max_x - start.x) * inv_x;
        let t3 = (self.min_y - start.y) * inv_y;
        let t4 = (self.max_y - start.y) * inv_y;
        let t5 = (self.min_z - start.z) * inv_z;
        let t6 = (self.max_z - start.z) * inv_z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax || tmin > max_dist {
            None
        } else {
            Some(tmin.max(0.0))
        }
    }
}

/// Result of a raytrace query against blocks or entities.
#[derive(Debug, Clone, PartialEq)]
pub struct RayTraceResult {
    pub hit_position: Vector3,
    pub hit_normal: Option<Vector3>,
    pub distance: f64,
}

/// Represents a precise position and rotation within a specific world.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub world_id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

impl Location {
    pub fn new(world_id: impl Into<String>, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> Self {
        Self {
            world_id: world_id.into(),
            x,
            y,
            z,
            yaw,
            pitch,
        }
    }

    pub fn position(&self) -> Vector3 {
        Vector3::new(self.x, self.y, self.z)
    }

    pub fn distance_squared(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance(&self, other: &Self) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

/// Represents a block state in the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The namespaced block identifier, e.g. "minecraft:stone".
    pub block_type: String,
    /// The internal numeric state id.
    pub state_id: u32,
}

impl Block {
    pub fn new(block_type: impl Into<String>, state_id: u32) -> Self {
        let mut bt = block_type.into();
        if !bt.contains(':') {
            bt = format!("minecraft:{}", bt);
        }
        Self {
            block_type: bt,
            state_id,
        }
    }

    pub fn is_air(&self) -> bool {
        self.is_type("air") || self.is_type("cave_air") || self.is_type("void_air")
    }

    /// Checks if this block matches the specified name, regardless of whether
    /// a "minecraft:" namespace prefix is provided in the query.
    pub fn is_type(&self, query: &str) -> bool {
        let clean_self = self.block_type.strip_prefix("minecraft:").unwrap_or(&self.block_type);
        let clean_query = query.strip_prefix("minecraft:").unwrap_or(query);
        clean_self.eq_ignore_ascii_case(clean_query)
    }

    /// Returns the simple unnamespaced block name (e.g. "dirt", "grass_block").
    pub fn name(&self) -> &str {
        self.block_type.strip_prefix("minecraft:").unwrap_or(&self.block_type)
    }

    /// Returns the full namespaced block identifier (e.g. "minecraft:dirt").
    pub fn namespaced_id(&self) -> &str {
        &self.block_type
    }
}

/// Minecraft player game mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn id(&self) -> u8 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Survival),
            1 => Some(Self::Creative),
            2 => Some(Self::Adventure),
            3 => Some(Self::Spectator),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }
}

/// World difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn id(&self) -> u8 {
        match self {
            Self::Peaceful => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Peaceful),
            1 => Some(Self::Easy),
            2 => Some(Self::Normal),
            3 => Some(Self::Hard),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }
}

/// Living entity equipment slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Helmet,
    Chestplate,
    Leggings,
    Boots,
}

/// Persistent Data Container (PDC) allowing plugins to attach typed key-value data to items/entities.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PersistentDataContainer {
    pub(crate) strings: std::collections::HashMap<String, String>,
    pub(crate) ints: std::collections::HashMap<String, i32>,
    pub(crate) longs: std::collections::HashMap<String, i64>,
    pub(crate) bytes: std::collections::HashMap<String, Vec<u8>>,
}

impl PersistentDataContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.strings.insert(key.into(), value.into());
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.strings.get(key).map(|s| s.as_str())
    }

    pub fn set_int(&mut self, key: impl Into<String>, value: i32) {
        self.ints.insert(key.into(), value);
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.ints.get(key).copied()
    }

    pub fn set_long(&mut self, key: impl Into<String>, value: i64) {
        self.longs.insert(key.into(), value);
    }

    pub fn get_long(&self, key: &str) -> Option<i64> {
        self.longs.get(key).copied()
    }

    pub fn set_bytes(&mut self, key: impl Into<String>, bytes: Vec<u8>) {
        self.bytes.insert(key.into(), bytes);
    }

    pub fn get_bytes(&self, key: &str) -> Option<&[u8]> {
        self.bytes.get(key).map(|b| b.as_slice())
    }

    pub fn has(&self, key: &str) -> bool {
        self.strings.contains_key(key)
            || self.ints.contains_key(key)
            || self.longs.contains_key(key)
            || self.bytes.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) {
        self.strings.remove(key);
        self.ints.remove(key);
        self.longs.remove(key);
        self.bytes.remove(key);
    }
}

/// Represents an item stack with type, amount, enchantments, lore, and PersistentDataContainer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    /// The namespaced item identifier, e.g. "minecraft:diamond_sword".
    pub item_type: String,
    /// The count of items in this stack.
    pub amount: u32,
    /// Optional custom display name.
    pub custom_name: Option<String>,
    /// Lore lines attached to the item stack.
    pub lore: Vec<String>,
    /// Enchantment mapping (e.g. "minecraft:sharpness" -> level).
    pub enchantments: std::collections::HashMap<String, u32>,
    /// Persistent data container attached to this item.
    pub persistent_data: PersistentDataContainer,
    /// Optional CustomModelData integer (used by resource packs).
    pub custom_model_data: Option<i32>,
}

impl ItemStack {
    pub fn new(item_type: impl Into<String>, amount: u32) -> Self {
        Self {
            item_type: item_type.into(),
            amount,
            custom_name: None,
            lore: Vec::new(),
            enchantments: std::collections::HashMap::new(),
            persistent_data: PersistentDataContainer::new(),
            custom_model_data: None,
        }
    }

    pub fn with_amount(mut self, amount: u32) -> Self {
        self.amount = amount;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    pub fn with_lore(mut self, lore: Vec<String>) -> Self {
        self.lore = lore;
        self
    }

    pub fn with_enchantment(mut self, enchantment: impl Into<String>, level: u32) -> Self {
        self.enchantments.insert(enchantment.into(), level);
        self
    }

    pub fn set_custom_name(&mut self, name: impl Into<String>) {
        self.custom_name = Some(name.into());
    }

    pub fn add_lore(&mut self, line: impl Into<String>) {
        self.lore.push(line.into());
    }

    pub fn pdc(&self) -> &PersistentDataContainer {
        &self.persistent_data
    }

    pub fn pdc_mut(&mut self) -> &mut PersistentDataContainer {
        &mut self.persistent_data
    }

    pub fn set_pdc(&mut self, pdc: PersistentDataContainer) {
        self.persistent_data = pdc;
    }

    pub fn add_enchantment(&mut self, enchantment: impl Into<String>, level: u32) {
        self.enchantments.insert(enchantment.into(), level);
    }

    pub fn get_enchantment(&self, enchantment: &str) -> Option<u32> {
        self.enchantments.get(enchantment).copied()
    }

    pub fn get_enchantment_level(&self, enchantment: &str) -> Option<u32> {
        self.get_enchantment(enchantment)
    }

    pub fn has_enchantment(&self, enchantment: &str) -> bool {
        self.enchantments.contains_key(enchantment)
    }

    pub fn custom_model_data(&self) -> Option<i32> {
        self.custom_model_data
    }

    pub fn set_custom_model_data(&mut self, cmd: Option<i32>) {
        self.custom_model_data = cmd;
    }

    pub fn with_custom_model_data(mut self, cmd: i32) -> Self {
        self.custom_model_data = Some(cmd);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.amount == 0 || self.item_type == "minecraft:air"
    }
}

/// Inventory interface allowing access and mutation of item stacks.
pub trait HostInventory: Send + Sync {
    fn get_held_item(&self) -> Option<ItemStack>;
    fn set_held_item(&self, item: Option<ItemStack>);
    fn get_slot(&self, slot: usize) -> Option<ItemStack>;
    fn set_slot(&self, slot: usize, item: Option<ItemStack>);
    fn get_equipment(&self, slot: EquipmentSlot) -> Option<ItemStack>;
    fn set_equipment(&self, slot: EquipmentSlot, item: Option<ItemStack>);
    fn clear(&self);
}

/// Safe wrapper for a player's inventory.
#[derive(Clone)]
pub struct Inventory {
    handle: Arc<dyn HostInventory>,
}

impl Inventory {
    pub fn from_handle(handle: Arc<dyn HostInventory>) -> Self {
        Self { handle }
    }

    pub fn held_item(&self) -> Option<ItemStack> {
        self.handle.get_held_item()
    }

    pub fn set_held_item(&self, item: Option<ItemStack>) {
        self.handle.set_held_item(item);
    }

    pub fn slot(&self, slot: usize) -> Option<ItemStack> {
        self.handle.get_slot(slot)
    }

    pub fn set_slot(&self, slot: usize, item: Option<ItemStack>) {
        self.handle.set_slot(slot, item);
    }

    pub fn equipment(&self, slot: EquipmentSlot) -> Option<ItemStack> {
        self.handle.get_equipment(slot)
    }

    pub fn set_equipment(&self, slot: EquipmentSlot, item: Option<ItemStack>) {
        self.handle.set_equipment(slot, item);
    }

    pub fn helmet(&self) -> Option<ItemStack> {
        self.equipment(EquipmentSlot::Helmet)
    }

    pub fn chestplate(&self) -> Option<ItemStack> {
        self.equipment(EquipmentSlot::Chestplate)
    }

    pub fn leggings(&self) -> Option<ItemStack> {
        self.equipment(EquipmentSlot::Leggings)
    }

    pub fn boots(&self) -> Option<ItemStack> {
        self.equipment(EquipmentSlot::Boots)
    }

    pub fn offhand(&self) -> Option<ItemStack> {
        self.equipment(EquipmentSlot::OffHand)
    }

    pub fn clear(&self) {
        self.handle.clear();
    }
}

/// Represents an active or applicable potion status effect (e.g. speed, regeneration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PotionEffect {
    /// Namespaced or simple identifier of the effect (e.g. "minecraft:speed" or "speed").
    pub effect_type: String,
    /// Duration of the effect in game ticks (20 ticks = 1 second).
    pub duration_ticks: u32,
    /// Effect amplifier level (0 = Level I, 1 = Level II, etc.).
    pub amplifier: u8,
    /// Whether this is an ambient beacon/conduit effect.
    pub ambient: bool,
    /// Whether particles should be displayed around the entity.
    pub particles: bool,
    /// Whether the status icon should be displayed on screen.
    pub show_icon: bool,
}

impl PotionEffect {
    pub fn new(effect_type: impl Into<String>, duration_ticks: u32, amplifier: u8) -> Self {
        let mut et = effect_type.into();
        if !et.contains(':') {
            et = format!("minecraft:{}", et);
        }
        Self {
            effect_type: et,
            duration_ticks,
            amplifier,
            ambient: false,
            particles: true,
            show_icon: true,
        }
    }

    pub fn with_ambient(mut self, ambient: bool) -> Self {
        self.ambient = ambient;
        self
    }

    pub fn with_particles(mut self, particles: bool) -> Self {
        self.particles = particles;
        self
    }

    pub fn with_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }

    pub fn name(&self) -> &str {
        self.effect_type.strip_prefix("minecraft:").unwrap_or(&self.effect_type)
    }
}

