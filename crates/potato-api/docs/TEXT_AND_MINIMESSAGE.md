# Adventure Text & MiniMessage

PotatoMC includes a native implementation of Kyori's **Adventure Component API** and **MiniMessage formatting engine**, eliminating obsolete legacy section formatting (`§`) and providing expressive, RGB-rich text.

---

## 1. MiniMessage Formatting Syntax

Parse rich strings using `Component::from_mini_message(...)`:

```rust
use potato_api::text::Component;

// 1. Color tags
let c1 = Component::from_mini_message("<red>Danger!</red> <green>All systems clear.</green>");

// 2. Hex color codes
let c2 = Component::from_mini_message("<#ff55aa>Custom Pink Color</#ff55aa>");

// 3. True RGB Gradients
let c3 = Component::from_mini_message("<gradient:#ffaa00:#ff5555><bold>VIP SERVER ALERT</bold></gradient>");

// 4. Text decorations
let c4 = Component::from_mini_message("<bold><italic><underlined>Important Notice</underlined></italic></bold>");
```

### Supported Tags
- **Colors**: `<black>`, `<dark_blue>`, `<dark_green>`, `<dark_aqua>`, `<dark_red>`, `<dark_purple>`, `<gold>`, `<gray>`, `<dark_gray>`, `<blue>`, `<green>`, `<aqua>`, `<red>`, `<light_purple>`, `<yellow>`, `<white>`
- **Custom Hex**: `<#RRGGBB>content</#RRGGBB>`
- **Gradients**: `<gradient:#color1:#color2>text</gradient>`
- **Decorations**: `<bold>`, `<italic>`, `<underlined>`, `<strikethrough>`, `<obfuscated>`

---

## 2. Programmatic Component Builder

Build components dynamically using method chaining:

```rust
use potato_api::text::{Component, NamedTextColor};

let comp = Component::text("Hello ")
    .color(NamedTextColor::Yellow)
    .append(
        Component::text("Admin")
            .color(NamedTextColor::Red)
            .bold()
    )
    .append(Component::text("!"));
```

---

## 3. Player UI Methods

### Chat Messages
```rust
player.send_component(&comp);
```

### Action Bar
Displays text immediately above the player's hotbar:
```rust
player.send_action_bar_component(&Component::text("§aAuto-saved world"));
```

### Titles & Subtitles
Displays animated full-screen titles:
```rust
player.send_title_components(
    &Component::from_mini_message("<gold><bold>VICTORY</bold></gold>"),
    &Component::text("You defeated the boss!"),
    10,  // Fade in (ticks)
    70,  // Stay on screen (ticks)
    20   // Fade out (ticks)
);
```

### Player Tab List Header & Footer (Paper Parity)
Dynamically modifies the top and bottom text shown when pressing `[TAB]`:
```rust
let header = Component::from_mini_message("<gold><bold>PotatoMC Server</bold></gold>");
let footer = Component::from_mini_message("<gray>TPS: 20.0 | Ping: 15ms</gray>");

player.set_player_list_header_footer(&header.to_legacy_string(), &footer.to_legacy_string());
```
