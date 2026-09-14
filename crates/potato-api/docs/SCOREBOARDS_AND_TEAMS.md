# Scoreboard & Teams API

PotatoMC provides a native, high-performance Scoreboard and Team system matching the ergonomics of Bukkit/Paper and popular libraries like FastBoard—with zero allocation overhead and sub-millisecond execution.

---

## 1. Quick Sidebar Scoreboards (FastBoard Style)

For lightweight, reactive sidebar menus (such as lobby or minigame stats), update lines from top to bottom with automatic descending score mapping:

```rust
// Fast sidebar creation (ordered top to bottom)
player.set_sidebar_lines("§6§lPotatoMC Network", &[
    "§7-----------------",
    "§fRank: §eMVP+",
    "§fCoins: §612,450",
    "§fOnline: §a48/100",
    "§7-----------------",
    "§epotatomc.flaxa.in",
]);

// Clear or reset scoreboard
player.clear_scoreboard();
player.reset_scoreboard();
```

---

## 2. Advanced Objectives & Display Slots

PotatoMC supports full objective tracking, criterion management, and display slots (`Sidebar`, `BelowName`, `List` / tab-list, and `TeamColor`):

```rust
use potato_api::scoreboard::{Scoreboard, DisplaySlot, ObjectiveCriteria, RenderType};

let mut board = Scoreboard::new("game_board", "§b§lBedWars", DisplaySlot::Sidebar);

// 1. Sidebar Lines
board.set_line(15, "§7Map: §fAirshow");
board.set_line(14, "§7Kills: §a3");
board.set_line(13, "§7Beds Broken: §e1");

// 2. Below-Name Health Objective (showing player health hearts or numbers)
let health_obj = board.register_objective(
    "player_health",
    "§c❤",
    ObjectiveCriteria::Health,
    DisplaySlot::BelowName,
);
health_obj.render_type = RenderType::Hearts;

// 3. Tab-List Kills Objective
board.register_objective(
    "tab_kills",
    "Kills",
    ObjectiveCriteria::PlayerKillCount,
    DisplaySlot::List,
);

// Apply to player
player.set_scoreboard(&board);
```

---

## 3. Score Entries & Score Tracking

Track, query, and modify numeric scores for arbitrary entities or offline usernames:

```rust
// Set scores explicitly
board.set_score("Steve", "player_health", 20);
board.set_score("Alex", "player_health", 18);

// Query scores
if let Some(score) = board.get_score("Steve", "player_health") {
    println!("Steve has {} health", score);
}

// Reset single score or all scores for an entry
board.reset_score("Steve", "player_health");
board.reset_scores("Alex");
```

---

## 4. Teams & Name Tags (`Team`)

Configure player prefix/suffix colors, friendly fire, invisibility visibility, and collision rules:

```rust
use potato_api::scoreboard::Team;

let red_team = Team::new("red_team")
    .prefix("§c[RED] ")
    .suffix(" §7★")
    .color("red")
    .friendly_fire(false)
    .see_friendly_invisibles(true)
    .add_entry("MAGIC_PLAYZZ")
    .add_entry("Steve");

board.add_team(red_team);
```

---

## 5. Listening to Score Changes (`ScoreboardScoreChangeEvent`)

Hook into score updates across all objectives and sidebars:

```rust
use potato_api::event::{ScoreboardScoreChangeEvent, Cancellable};

context.register_event(|event: &mut ScoreboardScoreChangeEvent| {
    println!(
        "[Scoreboard] {} scored {} (was {:?}) in objective {}",
        event.entry,
        event.new_score,
        event.previous_score,
        event.objective_name
    );

    // Cancel unauthorized score modification
    if event.new_score < 0 {
        event.set_cancelled(true);
    }
});
```

---

## 6. Minimal Copy-Paste Plugin Example

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::scoreboard::{DisplaySlot, ObjectiveCriteria, Scoreboard, Team};
use potato_api::event::{PlayerJoinEvent, ScoreboardScoreChangeEvent};
use potato_api::potato_plugin;

#[derive(Default)]
pub struct ScoreboardPlugin;

impl Plugin for ScoreboardPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("ScoreboardPlugin", "1.0.0", "Flaxa")
    }

    fn on_enable(&mut self, context: &mut PluginContext) -> Result<(), String> {
        // Build the lobby scoreboard
        let mut board = Scoreboard::new("lobby", "§6§lPotatoMC Network", DisplaySlot::Sidebar);
        board.set_line(15, "§7-----------------");
        board.set_line(14, "§fWelcome to §6PotatoMC§f!");
        board.set_line(13, "§fTPS: §a20.0");
        board.set_line(12, "§7-----------------");
        board.set_line(11, "§eplay.potatomc.flaxa.in");

        // Add below-name health
        board.register_objective("health", "§c❤", ObjectiveCriteria::Health, DisplaySlot::BelowName);

        // Add staff team
        let staff = Team::new("staff")
            .prefix("§c[Admin] ")
            .color("red")
            .friendly_fire(false);
        board.add_team(staff);

        // Apply on join
        context.register_event(move |event: &mut PlayerJoinEvent| {
            event.player.set_scoreboard(&board);
        });

        // Track score changes
        context.register_event(|event: &mut ScoreboardScoreChangeEvent| {
            println!("Score update: {} -> {}", event.entry, event.new_score);
        });

        Ok(())
    }
}

potato_plugin!(ScoreboardPlugin);
```
