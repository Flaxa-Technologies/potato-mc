use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::player::Player;
use crate::types::{EquipmentSlot, ItemStack, Location};

/// Skin texture and signature data for Player NPCs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinData {
    pub value: String,
    pub signature: Option<String>,
}

impl SkinData {
    pub fn new(value: impl Into<String>, signature: Option<String>) -> Self {
        Self {
            value: value.into(),
            signature,
        }
    }
}

/// Supported poses for NPCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EntityPose {
    #[default]
    Standing,
    FallFlying,
    Sleeping,
    Swimming,
    SpinAttack,
    Sneaking,
    LongJumping,
    Dying,
    Croaking,
    UsingTongue,
    Sitting,
    Roaring,
    Sniffing,
    Emerging,
    Digging,
}

impl EntityPose {
    pub fn to_protocol_id(&self) -> u32 {
        match self {
            Self::Standing => 0,
            Self::FallFlying => 1,
            Self::Sleeping => 2,
            Self::Swimming => 3,
            Self::SpinAttack => 4,
            Self::Sneaking => 5,
            Self::LongJumping => 6,
            Self::Dying => 7,
            Self::Croaking => 8,
            Self::UsingTongue => 9,
            Self::Sitting => 10,
            Self::Roaring => 11,
            Self::Sniffing => 12,
            Self::Emerging => 13,
            Self::Digging => 14,
        }
    }
}

/// Defines whether the NPC renders as a player or a specific entity type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcType {
    Player { skin: Option<SkinData> },
    Entity { entity_type: String },
}

/// Interaction click type for NPCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpcClickType {
    LeftClick,
    RightClick,
}

/// Trait for attaching arbitrary custom behavior to an NPC without touching core engine code.
pub trait NpcBehavior: Send + Sync {
    fn on_spawn(&mut self, _npc: &Npc) {}
    fn on_tick(&mut self, _npc: &Npc) {}
    fn on_interact(&mut self, _npc: &Npc, _player: &Player, _click: NpcClickType) {}
    fn on_despawn(&mut self, _npc: &Npc) {}
}

/// Internal host interface for driving NPC actions and lifecycle.
pub trait HostNpc: Send + Sync {
    fn id(&self) -> u32;
    fn uuid(&self) -> Uuid;
    fn name(&self) -> String;
    fn set_name(&self, name: &str);
    fn location(&self) -> Location;
    fn teleport(&self, location: &Location) -> bool;
    fn move_to(&self, location: &Location, speed: f64);
    fn pose(&self) -> EntityPose;
    fn set_pose(&self, pose: EntityPose);
    fn skin(&self) -> Option<SkinData>;
    fn set_skin(&self, skin: SkinData);
    fn is_glowing(&self) -> bool;
    fn set_glowing(&self, glowing: bool);
    fn set_equipment(&self, slot: EquipmentSlot, item: Option<ItemStack>);
    fn despawn(&self);
    fn is_valid(&self) -> bool;
}

/// Represents an active NPC in the world.
#[derive(Clone)]
pub struct Npc {
    handle: Arc<dyn HostNpc>,
    behavior: Option<Arc<Mutex<Box<dyn NpcBehavior>>>>,
}

impl Npc {
    pub fn new(handle: Arc<dyn HostNpc>, behavior: Option<Box<dyn NpcBehavior>>) -> Self {
        Self {
            handle,
            behavior: behavior.map(|b| Arc::new(Mutex::new(b))),
        }
    }

    /// Creates a fluent builder to configure and spawn a new NPC.
    pub fn builder(name: impl Into<String>) -> NpcBuilder {
        NpcBuilder::new(name)
    }

    pub fn id(&self) -> u32 {
        self.handle.id()
    }

    pub fn uuid(&self) -> Uuid {
        self.handle.uuid()
    }

    pub fn name(&self) -> String {
        self.handle.name()
    }

    pub fn set_name(&self, name: &str) {
        self.handle.set_name(name);
    }

    pub fn location(&self) -> Location {
        self.handle.location()
    }

    pub fn teleport(&self, location: &Location) -> bool {
        self.handle.teleport(location)
    }

    /// Instructs the NPC to navigate toward a destination location.
    pub fn move_to(&self, location: &Location, speed: f64) {
        self.handle.move_to(location, speed);
    }

    pub fn pose(&self) -> EntityPose {
        self.handle.pose()
    }

    pub fn set_pose(&self, pose: EntityPose) {
        self.handle.set_pose(pose);
    }

    pub fn skin(&self) -> Option<SkinData> {
        self.handle.skin()
    }

    pub fn set_skin(&self, skin: SkinData) {
        self.handle.set_skin(skin);
    }

    pub fn is_glowing(&self) -> bool {
        self.handle.is_glowing()
    }

    pub fn set_glowing(&self, glowing: bool) {
        self.handle.set_glowing(glowing);
    }

    pub fn set_equipment(&self, slot: EquipmentSlot, item: Option<ItemStack>) {
        self.handle.set_equipment(slot, item);
    }

    pub fn despawn(&self) {
        if let Some(ref b) = self.behavior {
            if let Ok(mut behavior) = b.lock() {
                behavior.on_despawn(self);
            }
        }
        self.handle.despawn();
    }

    pub fn is_valid(&self) -> bool {
        self.handle.is_valid()
    }

    /// Invokes the behavior tick hook.
    pub fn tick(&self) {
        if let Some(ref b) = self.behavior {
            if let Ok(mut behavior) = b.lock() {
                behavior.on_tick(self);
            }
        }
    }

    /// Invokes the behavior interaction hook.
    pub fn interact(&self, player: &Player, click: NpcClickType) {
        if let Some(ref b) = self.behavior {
            if let Ok(mut behavior) = b.lock() {
                behavior.on_interact(self, player, click);
            }
        }
    }
}

/// Fluent builder for constructing an NPC.
pub struct NpcBuilder {
    name: String,
    npc_type: NpcType,
    location: Option<Location>,
    pose: EntityPose,
    glowing: bool,
    invulnerable: bool,
    behavior: Option<Box<dyn NpcBehavior>>,
}

impl NpcBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            npc_type: NpcType::Player { skin: None },
            location: None,
            pose: EntityPose::Standing,
            glowing: false,
            invulnerable: true,
            behavior: None,
        }
    }

    pub fn player(mut self) -> Self {
        self.npc_type = NpcType::Player { skin: None };
        self
    }

    pub fn entity(mut self, entity_type: impl Into<String>) -> Self {
        self.npc_type = NpcType::Entity {
            entity_type: entity_type.into(),
        };
        self
    }

    pub fn skin(mut self, skin: SkinData) -> Self {
        self.npc_type = NpcType::Player { skin: Some(skin) };
        self
    }

    pub fn location(mut self, loc: Location) -> Self {
        self.location = Some(loc);
        self
    }

    pub fn pose(mut self, pose: EntityPose) -> Self {
        self.pose = pose;
        self
    }

    pub fn glowing(mut self, glowing: bool) -> Self {
        self.glowing = glowing;
        self
    }

    pub fn invulnerable(mut self, invulnerable: bool) -> Self {
        self.invulnerable = invulnerable;
        self
    }

    pub fn behavior(mut self, behavior: Box<dyn NpcBehavior>) -> Self {
        self.behavior = Some(behavior);
        self
    }

    /// Builds and spawns the NPC using the host context.
    pub fn spawn(self, context: &crate::plugin::PluginContext) -> Npc {
        context.spawn_npc(self)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> &NpcType {
        &self.npc_type
    }

    pub fn get_location(&self) -> Option<&Location> {
        self.location.as_ref()
    }

    pub fn get_pose(&self) -> EntityPose {
        self.pose
    }

    pub fn is_glowing(&self) -> bool {
        self.glowing
    }

    pub fn is_invulnerable(&self) -> bool {
        self.invulnerable
    }

    pub fn take_behavior(&mut self) -> Option<Box<dyn NpcBehavior>> {
        self.behavior.take()
    }
}
