use serde::{Deserialize, Serialize};
use crate::types::GameMode;

/// Client Game Event identifiers matching vanilla Minecraft CGameEvent packet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GameEvent {
    NoRespawnBlockAvailable = 0,
    StartRaining = 1,
    StopRaining = 2,
    ChangeGameMode(GameMode),
    WinGameCredits = 4,
    DemoMessage = 5,
    ArrowHitPlayer = 6,
    RainLevelChange = 7,
    ThunderLevelChange = 8,
    ElderGuardianAppearance = 10,
    EnableRespawnScreen = 11,
    LimitedCrafting = 12,
}

impl GameEvent {
    pub fn id_and_value(&self) -> (u8, f32) {
        match self {
            Self::NoRespawnBlockAvailable => (0, 0.0),
            Self::StartRaining => (1, 0.0),
            Self::StopRaining => (2, 0.0),
            Self::ChangeGameMode(mode) => {
                let val = match mode {
                    GameMode::Survival => 0.0,
                    GameMode::Creative => 1.0,
                    GameMode::Adventure => 2.0,
                    GameMode::Spectator => 3.0,
                };
                (3, val)
            }
            Self::WinGameCredits => (4, 1.0),
            Self::DemoMessage => (5, 0.0),
            Self::ArrowHitPlayer => (6, 0.0),
            Self::RainLevelChange => (7, 0.0),
            Self::ThunderLevelChange => (8, 0.0),
            Self::ElderGuardianAppearance => (10, 0.0),
            Self::EnableRespawnScreen => (11, 0.0),
            Self::LimitedCrafting => (12, 0.0),
        }
    }
}
