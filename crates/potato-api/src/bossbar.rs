use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Colors for BossBar displays matching Paper's Adventure BossBar.Color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

/// Division overlay styles for BossBars matching Paper's Adventure BossBar.Overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BossBarStyle {
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

/// A BossBar that can be displayed at the top of a player's screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossBar {
    pub uuid: Uuid,
    pub title: String,
    pub progress: f32,
    pub color: BossBarColor,
    pub style: BossBarStyle,
}

impl BossBar {
    /// Creates a new BossBar with default progress 1.0 (full).
    pub fn new(title: impl Into<String>, color: BossBarColor, style: BossBarStyle) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            title: title.into(),
            progress: 1.0,
            color,
            style,
        }
    }

    /// Sets the progress of the boss bar (clamped to 0.0 .. 1.0).
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Updates the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Updates the progress.
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Updates the color.
    pub fn set_color(&mut self, color: BossBarColor) {
        self.color = color;
    }

    /// Updates the division overlay style.
    pub fn set_style(&mut self, style: BossBarStyle) {
        self.style = style;
    }
}
