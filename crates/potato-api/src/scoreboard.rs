use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

/// Display slots for objectives on player screens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisplaySlot {
    List,
    Sidebar,
    BelowName,
    TeamColor(String),
}

/// Rendering style for scoreboards (e.g. integer count or heart visualizer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RenderType {
    #[default]
    Integer,
    Hearts,
}

/// Criterion for scoreboard objectives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ObjectiveCriteria {
    #[default]
    Dummy,
    Trigger,
    DeathCount,
    PlayerKillCount,
    TotalKillCount,
    Health,
    Custom(String),
}

/// An individual scoreboard objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub name: String,
    pub display_name: String,
    pub criteria: ObjectiveCriteria,
    pub render_type: RenderType,
    pub display_slot: Option<DisplaySlot>,
}

impl Objective {
    pub fn new(name: impl Into<String>, display_name: impl Into<String>, criteria: ObjectiveCriteria) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            criteria,
            render_type: RenderType::Integer,
            display_slot: None,
        }
    }

    pub fn with_render_type(mut self, render_type: RenderType) -> Self {
        self.render_type = render_type;
        self
    }

    pub fn with_display_slot(mut self, slot: DisplaySlot) -> Self {
        self.display_slot = Some(slot);
        self
    }
}

/// An individual score entry tracked by an objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub entry: String,
    pub objective: String,
    pub score: i32,
    pub custom_display_name: Option<String>,
}

/// Paper/Bukkit-compatible Scoreboard representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scoreboard {
    pub name: String,
    pub title: String,
    pub slot: DisplaySlot,
    pub lines: Vec<(usize, String)>,
    pub objectives: Vec<Objective>,
    pub scores: Vec<ScoreEntry>,
    pub teams: Vec<Team>,
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self::new("main", "Scoreboard", DisplaySlot::Sidebar)
    }
}

impl Scoreboard {
    pub fn new(name: impl Into<String>, title: impl Into<String>, slot: DisplaySlot) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            slot,
            lines: Vec::new(),
            objectives: Vec::new(),
            scores: Vec::new(),
            teams: Vec::new(),
        }
    }

    /// Fast helper to create a sidebar scoreboard (like FastBoard).
    pub fn sidebar(title: impl Into<String>) -> Self {
        Self::new("sidebar", title, DisplaySlot::Sidebar)
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    // ==========================================
    // Fast Sidebar Lines (Compatibility API)
    // ==========================================

    /// Set a specific score line (score is typically 1..15 for sidebars).
    pub fn set_line(&mut self, score: usize, text: impl Into<String>) {
        let text = text.into();
        if let Some(pos) = self.lines.iter().position(|(s, _)| *s == score) {
            self.lines[pos].1 = text;
        } else {
            self.lines.push((score, text));
            self.lines.sort_by(|a, b| b.0.cmp(&a.0)); // Descending order
        }
    }

    /// Helper to set all lines at once from top to bottom (index 0 gets the highest score).
    pub fn set_lines(&mut self, lines: &[impl AsRef<str>]) {
        self.lines.clear();
        let total = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let score = total.saturating_sub(i);
            self.lines.push((score, line.as_ref().to_string()));
        }
    }

    pub fn get_line(&self, score: usize) -> Option<&str> {
        self.lines.iter().find(|(s, _)| *s == score).map(|(_, t)| t.as_str())
    }

    pub fn remove_line(&mut self, score: usize) {
        self.lines.retain(|(s, _)| *s != score);
    }

    pub fn clear_lines(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> &[(usize, String)] {
        &self.lines
    }

    // ==========================================
    // Objectives API
    // ==========================================

    pub fn register_objective(
        &mut self,
        name: impl Into<String>,
        display_name: impl Into<String>,
        criteria: ObjectiveCriteria,
        slot: DisplaySlot,
    ) -> &mut Objective {
        let name_str = name.into();
        let obj = Objective::new(name_str.clone(), display_name, criteria).with_display_slot(slot);
        self.objectives.retain(|o| o.name != name_str);
        self.objectives.push(obj);
        self.objectives.last_mut().unwrap()
    }

    pub fn get_objective(&self, name: &str) -> Option<&Objective> {
        self.objectives.iter().find(|o| o.name == name)
    }

    pub fn get_objective_mut(&mut self, name: &str) -> Option<&mut Objective> {
        self.objectives.iter_mut().find(|o| o.name == name)
    }

    pub fn remove_objective(&mut self, name: &str) -> Option<Objective> {
        if let Some(pos) = self.objectives.iter().position(|o| o.name == name) {
            self.scores.retain(|s| s.objective != name);
            Some(self.objectives.remove(pos))
        } else {
            None
        }
    }

    pub fn objectives(&self) -> &[Objective] {
        &self.objectives
    }

    pub fn set_display_slot(&mut self, slot: DisplaySlot, objective_name: impl Into<String>) {
        let name = objective_name.into();
        for obj in &mut self.objectives {
            if obj.name == name {
                obj.display_slot = Some(slot.clone());
            } else if obj.display_slot.as_ref() == Some(&slot) {
                obj.display_slot = None;
            }
        }
    }

    pub fn clear_display_slot(&mut self, slot: &DisplaySlot) {
        for obj in &mut self.objectives {
            if obj.display_slot.as_ref() == Some(slot) {
                obj.display_slot = None;
            }
        }
    }

    // ==========================================
    // Score Entries API
    // ==========================================

    pub fn set_score(&mut self, entry: impl Into<String>, objective: impl Into<String>, score: i32) {
        let entry_str = entry.into();
        let obj_str = objective.into();
        if let Some(se) = self.scores.iter_mut().find(|s| s.entry == entry_str && s.objective == obj_str) {
            se.score = score;
        } else {
            self.scores.push(ScoreEntry {
                entry: entry_str,
                objective: obj_str,
                score,
                custom_display_name: None,
            });
        }
    }

    pub fn get_score(&self, entry: &str, objective: &str) -> Option<i32> {
        self.scores
            .iter()
            .find(|s| s.entry == entry && s.objective == objective)
            .map(|s| s.score)
    }

    pub fn reset_score(&mut self, entry: &str, objective: &str) {
        self.scores.retain(|s| !(s.entry == entry && s.objective == objective));
    }

    pub fn reset_scores(&mut self, entry: &str) {
        self.scores.retain(|s| s.entry != entry);
    }

    pub fn scores(&self) -> &[ScoreEntry] {
        &self.scores
    }

    // ==========================================
    // Teams API
    // ==========================================

    pub fn add_team(&mut self, team: Team) {
        self.teams.retain(|t| t.name != team.name);
        self.teams.push(team);
    }

    pub fn get_team(&self, name: &str) -> Option<&Team> {
        self.teams.iter().find(|t| t.name == name)
    }

    pub fn get_team_mut(&mut self, name: &str) -> Option<&mut Team> {
        self.teams.iter_mut().find(|t| t.name == name)
    }

    pub fn remove_team(&mut self, name: &str) -> Option<Team> {
        if let Some(pos) = self.teams.iter().position(|t| t.name == name) {
            Some(self.teams.remove(pos))
        } else {
            None
        }
    }

    pub fn teams(&self) -> &[Team] {
        &self.teams
    }
}

/// Team representation for scoreboard colors, prefixes, suffixes, and visibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub display_name: String,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub color: Option<String>,
    pub friendly_fire: bool,
    pub see_friendly_invisibles: bool,
    pub entries: Vec<String>,
}

impl Team {
    pub fn new(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            name: n.clone(),
            display_name: n,
            prefix: None,
            suffix: None,
            color: None,
            friendly_fire: false,
            see_friendly_invisibles: false,
            entries: Vec::new(),
        }
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn friendly_fire(mut self, allow: bool) -> Self {
        self.friendly_fire = allow;
        self
    }

    pub fn see_friendly_invisibles(mut self, allow: bool) -> Self {
        self.see_friendly_invisibles = allow;
        self
    }

    pub fn add_entry(mut self, entry: impl Into<String>) -> Self {
        self.entries.push(entry.into());
        self
    }

    pub fn remove_entry(&mut self, entry: &str) {
        self.entries.retain(|e| e != entry);
    }
}

/// Thread-safe manager for server-wide and per-player scoreboards.
#[derive(Debug, Clone, Default)]
pub struct ScoreboardManager {
    main_scoreboard: Arc<RwLock<Scoreboard>>,
}

impl ScoreboardManager {
    pub fn new() -> Self {
        Self {
            main_scoreboard: Arc::new(RwLock::new(Scoreboard::sidebar("§6§lPotatoMC"))),
        }
    }

    pub fn main_scoreboard(&self) -> Scoreboard {
        self.main_scoreboard.read().unwrap().clone()
    }

    pub fn set_main_scoreboard(&self, scoreboard: Scoreboard) {
        *self.main_scoreboard.write().unwrap() = scoreboard;
    }

    pub fn new_scoreboard(&self, name: impl Into<String>, title: impl Into<String>) -> Scoreboard {
        Scoreboard::new(name, title, DisplaySlot::Sidebar)
    }
}
