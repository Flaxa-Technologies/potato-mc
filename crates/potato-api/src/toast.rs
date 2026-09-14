use serde::{Deserialize, Serialize};

/// Toast frame design types matching Minecraft advancement types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToastFrame {
    /// Standard curved square border.
    Task,
    /// Pointy shield-style border.
    Goal,
    /// Fancy ornate star/spike border.
    Challenge,
}

/// Rich toast notification displayed in the top-right corner of the player's screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Toast {
    pub title: String,
    pub icon: String,
    pub frame: ToastFrame,
}

impl Toast {
    pub fn new(title: impl Into<String>, icon: impl Into<String>, frame: ToastFrame) -> Self {
        Self {
            title: title.into(),
            icon: icon.into(),
            frame,
        }
    }

    pub fn task(title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self::new(title, icon, ToastFrame::Task)
    }

    pub fn goal(title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self::new(title, icon, ToastFrame::Goal)
    }

    pub fn challenge(title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self::new(title, icon, ToastFrame::Challenge)
    }
}
