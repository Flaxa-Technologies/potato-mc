use std::sync::Arc;
use uuid::Uuid;

use crate::host::{HostEntity, HostLivingEntity};
use crate::types::{Location, Vector3};

/// Safe abstraction representing an active entity in a world.
#[derive(Clone)]
pub struct Entity {
    pub(crate) handle: Arc<dyn HostEntity>,
}

impl Entity {
    pub fn from_handle(handle: Arc<dyn HostEntity>) -> Self {
        Self { handle }
    }

    pub fn inner(&self) -> &Arc<dyn HostEntity> {
        &self.handle
    }

    /// Returns the unique UUID of this entity.
    pub fn uuid(&self) -> Uuid {
        self.handle.uuid()
    }

    /// Returns the entity's type identifier, e.g. "minecraft:zombie".
    pub fn entity_type(&self) -> String {
        self.handle.entity_type()
    }

    /// Returns the current location of the entity.
    pub fn location(&self) -> Location {
        self.handle.location()
    }

    /// Teleports the entity to the specified location.
    pub fn teleport(&self, location: &Location) -> bool {
        self.handle.teleport(location)
    }

    /// Returns the current motion vector of the entity.
    pub fn velocity(&self) -> Vector3 {
        self.handle.velocity()
    }

    /// Updates the motion vector of the entity.
    pub fn set_velocity(&self, velocity: &Vector3) {
        self.handle.set_velocity(velocity);
    }

    /// Removes or despawns the entity from the world.
    pub fn remove(&self) {
        self.handle.remove();
    }

    /// Checks whether the entity is currently touching the ground.
    pub fn is_on_ground(&self) -> bool {
        self.handle.is_on_ground()
    }

    /// Returns the custom display name of the entity, if set.
    pub fn custom_name(&self) -> Option<String> {
        self.handle.custom_name()
    }

    /// Sets or removes the custom display name of the entity.
    pub fn set_custom_name(&self, name: Option<&str>) {
        self.handle.set_custom_name(name);
    }

    /// Returns the number of ticks the entity is on fire.
    pub fn fire_ticks(&self) -> i32 {
        self.handle.fire_ticks()
    }

    /// Sets the number of ticks the entity is on fire.
    pub fn set_fire_ticks(&self, ticks: i32) {
        self.handle.set_fire_ticks(ticks);
    }

    /// Inflicts damage on the entity.
    pub fn damage(&self, amount: f32) {
        self.handle.damage(amount);
    }

    /// Checks if the entity has a glowing outline.
    pub fn is_glowing(&self) -> bool {
        self.handle.is_glowing()
    }

    /// Sets whether the entity has a glowing outline.
    pub fn set_glowing(&self, glowing: bool) {
        self.handle.set_glowing(glowing);
    }

    /// Checks if the entity is invulnerable to damage.
    pub fn is_invulnerable(&self) -> bool {
        self.handle.is_invulnerable()
    }

    /// Sets whether the entity is invulnerable to damage.
    pub fn set_invulnerable(&self, invulnerable: bool) {
        self.handle.set_invulnerable(invulnerable);
    }

    /// Checks if the entity makes ambient and movement sounds.
    pub fn is_silent(&self) -> bool {
        self.handle.is_silent()
    }

    /// Sets whether the entity is silent.
    pub fn set_silent(&self, silent: bool) {
        self.handle.set_silent(silent);
    }

    /// Checks if the entity is affected by gravity.
    pub fn has_gravity(&self) -> bool {
        self.handle.has_gravity()
    }

    /// Sets whether the entity is affected by gravity.
    pub fn set_gravity(&self, gravity: bool) {
        self.handle.set_gravity(gravity);
    }

    /// Returns the list of scoreboard tags attached to this entity.
    pub fn scoreboard_tags(&self) -> Vec<String> {
        self.handle.scoreboard_tags()
    }

    /// Adds a scoreboard tag to this entity.
    pub fn add_scoreboard_tag(&self, tag: &str) -> bool {
        self.handle.add_scoreboard_tag(tag)
    }

    /// Removes a scoreboard tag from this entity.
    pub fn remove_scoreboard_tag(&self, tag: &str) -> bool {
        self.handle.remove_scoreboard_tag(tag)
    }

    /// Returns a copy of the persistent data container attached to this entity.
    pub fn persistent_data(&self) -> crate::types::PersistentDataContainer {
        let json = self.handle.persistent_data_json();
        serde_json::from_str(&json).unwrap_or_default()
    }

    /// Updates the persistent data container for this entity.
    pub fn set_persistent_data(&self, pdc: &crate::types::PersistentDataContainer) {
        if let Ok(json) = serde_json::to_string(pdc) {
            self.handle.set_persistent_data_json(&json);
        }
    }

    /// Attempts to view this entity as a living entity (e.g. mob or player).
    pub fn as_living(&self) -> Option<LivingEntity> {
        self.handle.as_living().map(LivingEntity::from_handle)
    }
}

impl std::fmt::Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entity")
            .field("uuid", &self.uuid())
            .field("type", &self.entity_type())
            .field("location", &self.location())
            .finish()
    }
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.uuid() == other.uuid()
    }
}

impl Eq for Entity {}

impl std::hash::Hash for Entity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid().hash(state);
    }
}

/// Specialized abstraction for living entities with health and attributes.
#[derive(Clone)]
pub struct LivingEntity {
    pub(crate) handle: Arc<dyn HostLivingEntity>,
}

impl LivingEntity {
    pub fn from_handle(handle: Arc<dyn HostLivingEntity>) -> Self {
        Self { handle }
    }

    pub fn inner(&self) -> &Arc<dyn HostLivingEntity> {
        &self.handle
    }

    /// Accesses the underlying base entity.
    pub fn as_entity(&self) -> Entity {
        Entity::from_handle(self.handle.base_entity())
    }

    pub fn uuid(&self) -> Uuid {
        self.handle.base_entity().uuid()
    }

    pub fn entity_type(&self) -> String {
        self.handle.base_entity().entity_type()
    }

    pub fn location(&self) -> Location {
        self.handle.base_entity().location()
    }

    pub fn teleport(&self, location: &Location) -> bool {
        self.handle.base_entity().teleport(location)
    }

    pub fn velocity(&self) -> Vector3 {
        self.handle.base_entity().velocity()
    }

    pub fn set_velocity(&self, velocity: &Vector3) {
        self.handle.base_entity().set_velocity(velocity);
    }

    pub fn remove(&self) {
        self.handle.base_entity().remove();
    }

    /// Retrieves the current health points of this entity.
    pub fn health(&self) -> f32 {
        self.handle.health()
    }

    /// Sets the health points of this entity.
    pub fn set_health(&self, health: f32) {
        self.handle.set_health(health);
    }

    /// Retrieves the maximum health points of this entity.
    pub fn max_health(&self) -> f32 {
        self.handle.max_health()
    }

    /// Returns the eye location of this living entity.
    pub fn eye_location(&self) -> Location {
        self.handle.eye_location()
    }

    /// Returns the eye height offset of this living entity (default 1.62).
    pub fn eye_height(&self) -> f64 {
        self.handle.eye_height()
    }
}

impl std::fmt::Debug for LivingEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LivingEntity")
            .field("uuid", &self.uuid())
            .field("type", &self.entity_type())
            .field("health", &self.health())
            .field("max_health", &self.max_health())
            .finish()
    }
}
