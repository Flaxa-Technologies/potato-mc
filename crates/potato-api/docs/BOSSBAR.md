# Adventure BossBar API

PotatoMC plugins can construct and manage dynamic, styled BossBars matching Kyori's Adventure specification.

---

## 1. Creating a BossBar

```rust
use potato_api::bossbar::{BossBar, BossBarColor, BossBarStyle};

let bar = BossBar::new(
    "§6§lRaid Boss: Ancient Dragon",
    BossBarColor::Red,
    BossBarStyle::Notched12,
).with_progress(1.0); // 100% full
```

### Colors
- `BossBarColor::Pink`
- `BossBarColor::Blue`
- `BossBarColor::Red`
- `BossBarColor::Green`
- `BossBarColor::Yellow`
- `BossBarColor::Purple`
- `BossBarColor::White`

### Styles (Division Overlays)
- `BossBarStyle::Progress` (Solid continuous bar)
- `BossBarStyle::Notched6` (6 segments)
- `BossBarStyle::Notched10` (10 segments)
- `BossBarStyle::Notched12` (12 segments)
- `BossBarStyle::Notched20` (20 segments)

---

## 2. Display & Updates

```rust
// Update progress dynamically
let mut bar = bar;
bar.set_progress(0.45); // 45% remaining

// Update title
bar.set_title("§c§lDragon Enraged!");

// Update color
bar.set_color(BossBarColor::Purple);
```
