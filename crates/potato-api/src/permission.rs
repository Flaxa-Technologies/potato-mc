use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default grant behavior for a permission node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PermissionDefault {
    True,
    #[default]
    Op,
    False,
}

/// Declared permission node with metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub name: String,
    pub description: Option<String>,
    pub default_value: PermissionDefault,
}

impl Permission {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            default_value: PermissionDefault::Op,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_default(mut self, default_value: PermissionDefault) -> Self {
        self.default_value = default_value;
        self
    }
}

/// Manages dynamic plugin permissions and per-player overrides.
#[derive(Debug, Clone, Default)]
pub struct PermissionManager {
    permissions: Arc<RwLock<HashMap<String, Permission>>>,
    player_overrides: Arc<RwLock<HashMap<Uuid, HashMap<String, bool>>>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a permission node.
    pub fn register(&self, permission: Permission) {
        let mut map = self.permissions.write().unwrap();
        map.insert(permission.name.clone(), permission);
    }

    /// Overrides a permission node for a specific player UUID.
    pub fn set_player_permission(&self, player_uuid: Uuid, node: impl Into<String>, value: bool) {
        let mut map = self.player_overrides.write().unwrap();
        map.entry(player_uuid)
            .or_default()
            .insert(node.into(), value);
    }

    /// Removes a player-specific permission override.
    pub fn unset_player_permission(&self, player_uuid: &Uuid, node: &str) {
        let mut map = self.player_overrides.write().unwrap();
        if let Some(overrides) = map.get_mut(player_uuid) {
            overrides.remove(node);
        }
    }

    /// Checks if a player has a specific permission override.
    pub fn get_player_override(&self, player_uuid: &Uuid, node: &str) -> Option<bool> {
        let map = self.player_overrides.read().unwrap();
        map.get(player_uuid)?.get(node).copied()
    }
}
