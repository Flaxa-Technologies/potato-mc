# Dialog Box, Forms & Virtual Book API

PotatoMC introduces first-class, Paper-grade support for interactive client dialogs, Bedrock crossplay forms, and virtual book interfaces—enabling rich, interactive user experiences without client mods.

---

## 1. Native Java 1.21.4+ Dialog Boxes

Minecraft Java 1.21.4 introduced native modal dialog boxes directly inside the client engine. PotatoMC provides a fluent builder to create and display these dialogs effortlessly:

```rust
use potato_api::dialog::{Dialog, DialogButton};
use potato_api::player::Player;

let dialog = Dialog::new("quest_dragon_slayer")
    .title("§6§lThe Ancient Dragon Quest")
    .body_text("A ferocious dragon threatens the realm! Will you embark on the journey to slay the beast?")
    .add_action_button("§a§lAccept Quest", "quest_accept")
    .add_action_button("§cDecline", "quest_decline")
    .add_url_button("§eWiki & Guides", "https://potatomc.flaxa.in/")
    .can_close_with_escape(true);

// Show dialog to player
player.show_dialog(&dialog);

// Clear dialog when done
player.clear_dialog();
```

### Adding Interactive Inputs

Dialogs can collect input data directly from the user:

```rust
let config_dialog = Dialog::new("player_settings")
    .title("Player Preferences")
    .add_bool_input("pvp_enabled", "Enable PvP Combat", true)
    .add_text_input("nickname", "Custom Nickname", "Enter nick...", "Player")
    .add_number_range("particle_density", "Particle Multiplier", 0.5, 3.0, 1.0, 0.5)
    .add_single_option("title_color", "Preferred Color", vec!["Gold".into(), "Aqua".into(), "Red".into()], 0)
    .add_action_button("Save Changes", "save_settings");

player.show_dialog(&config_dialog);
```

---

## 2. Bedrock & Crossplay Forms (`SimpleForm`, `ModalForm`, `CustomForm`)

For cross-play networks running Bedrock Edition players, PotatoMC includes native Bedrock JSON forms:

### Simple Form (Menu with Buttons & Icons)
```rust
use potato_api::dialog::{SimpleForm, FormImage};

let form = SimpleForm::new("§2§lServer Selector")
    .content("Choose a server realm to join:")
    .button_with_image("Survival Realm", FormImage::url("https://example.com/survival.png"))
    .button_with_image("Creative Realm", FormImage::url("https://example.com/creative.png"))
    .button("Minigames");

player.send_simple_form(101, &form);
```

### Modal Form (Two-Button Confirmation)
```rust
use potato_api::dialog::ModalForm;

let modal = ModalForm::new(
    "Purchase Confirmation",
    "Do you want to buy the Diamond Rank for 1000 Coins?",
    "Confirm Purchase",
    "Cancel"
);

player.send_modal_form(102, &modal);
```

### Custom Form (Form Fields, Toggles, Sliders, Dropdowns)
```rust
use potato_api::dialog::CustomForm;

let custom = CustomForm::new("Guild Creation")
    .input("Guild Tag", "3-5 characters", "POT")
    .toggle("Public Recruitment", true)
    .slider("Tax Rate (%)", 0.0, 20.0, 1.0, 5.0)
    .dropdown("Primary Language", vec!["English".into(), "Spanish".into(), "German".into()], 0);

player.send_custom_form(103, &custom);
```

---

## 3. Virtual Book GUI (`Book`)

Open a written book screen directly for a player without requiring them to hold a book in their hands (equivalent to Paper's `Player#openBook`):

```rust
use potato_api::dialog::Book;

let rules = Book::new("Server Rules", "Admin")
    .add_page("§1§lServer Rules\n\n§01. Be respectful to others.\n2. No hacking or exploits.\n3. Keep chat family-friendly.")
    .add_page("§1§lSection 2\n\n§04. Report bugs on Discord.\n5. Have fun exploring PotatoMC!");

player.open_book(&rules);
```

---

## 4. Sign Editor Dialog

Open the interactive sign typing interface for a player at any world block location:

```rust
player.open_sign_editor(&location);
```

---

## 5. Listening to Dialog Events

Handle player responses and clicks via the event bus:

```rust
use potato_api::event::{DialogClickActionEvent, PlayerFormResponseEvent, Cancellable};

// Java Dialog button clicks
context.register_event(|event: &mut DialogClickActionEvent| {
    if event.action_id == "quest_accept" {
        event.player.send_message("§aYou have accepted the quest!");
        event.player.clear_dialog();
    }
});

// Bedrock form responses
context.register_event(|event: &mut PlayerFormResponseEvent| {
    if event.form_id == 101 {
        event.player.send_message(&format!("Selected option: {}", event.response_json));
    }
});
```
