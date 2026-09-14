# GUI & Inventory API

PotatoMC provides a native, high-performance Virtual Chest GUI and Inventory management API mirroring the simplicity and power of Paper's `InventoryHolder` and custom screen handlers—running at native C/Rust speeds with zero JVM GC pressure.

---

## 1. Creating Virtual Chest GUIs

Create custom chest screens with `Gui::chest(title, rows)`:

```rust
use potato_api::gui::Gui;
use potato_api::types::ItemStack;

// Create a 3-row (27 slots) virtual chest GUI
let mut gui = Gui::chest("§6§lPotato Rewards Shop", 3);

// Prevent players from taking decorative items
gui.set_allow_grab(false);
gui.set_allow_put(false);

// Configure decorative border
let mut border = ItemStack::new("minecraft:gray_stained_glass_pane", 1);
border.set_custom_name(" ");
gui.fill_border(border);

// Place interactive items in specific slots
let mut reward = ItemStack::new("minecraft:netherite_sword", 1);
reward.set_custom_name("§4§lExecutioner Blade");
reward.add_lore("§7Click to purchase for 50 Coins");
reward.add_enchantment("minecraft:sharpness", 5);
gui.set_item(13, reward); // Center slot
```

### GUI Configuration Options

| Method | Description |
|---|---|
| `Gui::chest(title, rows)` | Create a chest GUI with 1–6 rows (9 to 54 slots). |
| `gui.set_item(slot, item)` | Set an `ItemStack` at an explicit slot index (0-indexed). |
| `gui.get_item(slot)` | Retrieve a reference to the item at `slot`. |
| `gui.fill_border(item)` | Fill row 0, row N-1, and column 0 & 8 borders with an item. |
| `gui.fill_all(item)` | Fill all unoccupied slots with an item. |
| `gui.set_allow_grab(bool)` | Flag whether players can take items out of the GUI (default: `false`). |
| `gui.set_allow_put(bool)` | Flag whether players can place items into the GUI (default: `false`). |

---

## 2. Opening & Closing GUIs

Open or close screens directly on `Player`:

```rust
// Open custom GUI for player
player.open_gui(&gui);

// Close currently opened screen
player.close_inventory();
```

When `open_gui` is called, PotatoMC:
1. Dynamically constructs a container screen.
2. Synchronizes all item slots to the client via native protocol packets (`COpenScreen` and `CSetContainerSlot`).
3. Handles cross-play transparently for both Java Edition and Bedrock Edition players.

---

## 3. Handling GUI Clicks (`InventoryClickEvent`)

Listen to `InventoryClickEvent` to implement custom shop logic, menu navigation, or item protection:

```rust
use potato_api::event::{InventoryClickEvent, Cancellable};

context.register_event(|event: &mut InventoryClickEvent| {
    // Check clicked slot
    if event.slot == 13 {
        // Cancel to prevent grabbing the display item
        event.set_cancelled(true);

        event.player.send_message("§a[Shop] You purchased the Executioner Blade!");
        
        let mut reward = ItemStack::new("minecraft:netherite_sword", 1);
        reward.set_custom_name("§4§lExecutioner Blade");
        reward.add_enchantment("minecraft:sharpness", 5);
        
        // Give item to player
        event.player.give_item(&reward);
        event.player.close_inventory();
    }
});
```

### Event Fields

- `event.player`: The `Player` who clicked.
- `event.slot`: Slot number clicked (0 to size-1 for top inventory, 36–44 for hotbar, 9–35 for main inventory).
- `event.click_type`: Click action identifier (0 = Pickup, 1 = QuickMove/ShiftClick, etc.).
- `event.clicked_item`: `Option<ItemStack>` of the item in the clicked slot.
- `event.cursor_item`: `Option<ItemStack>` of the item currently held on the cursor.
- `event.set_cancelled(true)`: Reverts and cancels the click on both server and client.

---

## 4. Player Inventory Manipulation

### Giving Items (`give_item`)

Adds an item directly to the player's inventory. If the inventory is full, the remaining items are spawned naturally at the player's feet:

```rust
let mut apple = ItemStack::new("minecraft:enchanted_golden_apple", 5);
apple.set_custom_name("§6§lSuper Apple");

// Give directly to player
player.give_item(&apple);
```

### Dropping Items (`drop_item`)

Spawns an item drop directly from the player:

```rust
let diamond = ItemStack::new("minecraft:diamond", 3);
player.drop_item(&diamond);
```

### World Dropped Items

Spawn dropped items anywhere in the world:

```rust
use potato_api::types::{Location, ItemStack};

let loc = Location::new("world", 100.5, 64.0, -200.5);
let gem = ItemStack::new("minecraft:emerald", 10);

// Drop at exact coordinates
world.drop_item(loc, &gem);

// Drop naturally with random horizontal velocity
world.drop_item_naturally(loc, &gem);
```
