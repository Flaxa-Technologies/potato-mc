use serde::{Deserialize, Serialize};

/// Sound categories mirroring Minecraft and Bukkit SoundCategory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundCategory {
    Master = 0,
    Music = 1,
    Records = 2,
    Weather = 3,
    Blocks = 4,
    Hostile = 5,
    Neutral = 6,
    Players = 7,
    Ambient = 8,
    Voice = 9,
}

impl SoundCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Records => "records",
            Self::Weather => "weather",
            Self::Blocks => "blocks",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Players => "players",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
        }
    }
}

/// Standard Minecraft sound identifier constants for plugin convenience.
pub struct Sound;

impl Sound {
    pub const ENTITY_PLAYER_LEVELUP: &'static str = "minecraft:entity.player.levelup";
    pub const ENTITY_EXPERIENCE_ORB_PICKUP: &'static str = "minecraft:entity.experience_orb.pickup";
    pub const BLOCK_NOTE_BLOCK_PLING: &'static str = "minecraft:block.note_block.pling";
    pub const BLOCK_NOTE_BLOCK_BELL: &'static str = "minecraft:block.note_block.bell";
    pub const BLOCK_NOTE_BLOCK_CHIME: &'static str = "minecraft:block.note_block.chime";
    pub const BLOCK_CHEST_OPEN: &'static str = "minecraft:block.chest.open";
    pub const BLOCK_CHEST_CLOSE: &'static str = "minecraft:block.chest.close";
    pub const ENTITY_GENERIC_EXPLODE: &'static str = "minecraft:entity.generic.explode";
    pub const ENTITY_LIGHTNING_BOLT_THUNDER: &'static str = "minecraft:entity.lightning_bolt.thunder";
    pub const ENTITY_ENDERMAN_TELEPORT: &'static str = "minecraft:entity.enderman.teleport";
    pub const UI_BUTTON_CLICK: &'static str = "minecraft:ui.button.click";
    pub const UI_TOAST_CHALLENGE_COMPLETE: &'static str = "minecraft:ui.toast.challenge_complete";
    pub const ENTITY_VILLAGER_YES: &'static str = "minecraft:entity.villager.yes";
    pub const ENTITY_VILLAGER_NO: &'static str = "minecraft:entity.villager.no";
    pub const ENTITY_ITEM_PICKUP: &'static str = "minecraft:entity.item.pickup";
    pub const ITEM_ARMOR_EQUIP_GENERIC: &'static str = "minecraft:item.armor.equip_generic";
    pub const BLOCK_ANVIL_USE: &'static str = "minecraft:block.anvil.use";
    pub const BLOCK_ANVIL_LAND: &'static str = "minecraft:block.anvil.land";
    pub const ENTITY_ARROW_HIT_PLAYER: &'static str = "minecraft:entity.arrow.hit_player";
}
