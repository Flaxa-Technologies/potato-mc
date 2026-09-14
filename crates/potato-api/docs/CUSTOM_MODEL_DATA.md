# Custom Model Data & Item Variants API

PotatoMC allows plugins to easily interface with client-side resource packs via Custom Model Data. Instead of hand-rolling raw NBT manipulation, plugins can read and write model data directly on `ItemStack`, and declare custom item templates using the `ItemVariant` & `ItemVariantRegistry` system.

---

## 1. Direct Custom Model Data Manipulation

Every `ItemStack` in PotatoMC directly exposes custom model data getters and setters:

```rust
use potato_api::types::ItemStack;

let mut item = ItemStack::new("minecraft:diamond_sword", 1);

// Set Custom Model Data
item.set_custom_model_data(Some(10501));

// Fluent builder pattern
let ruby_pick = ItemStack::new("minecraft:diamond_pickaxe", 1)
    .with_custom_model_data(20042)
    .with_name("§cRuby Pickaxe");

// Read Custom Model Data
if let Some(cmd) = item.custom_model_data() {
    println!("Item has CustomModelData: {}", cmd);
}
```

---

## 2. Declarative Item Variants (`ItemVariant`)

Define custom items driven by resource packs cleanly with base item, custom model data, default display names, lore, enchantments, and persistent NBT tags:

```rust
use potato_api::item_variant::ItemVariant;

let ruby_sword = ItemVariant::builder("minecraft:diamond_sword")
    .custom_model_data(10501)
    .name("§c§lRuby Greatsword")
    .lore([
        "§7Forged from crystallized nether rubies.",
        "§4+14 Attack Damage",
        "§6Ability: Flame Burst",
    ])
    .enchantment("minecraft:fire_aspect", 2)
    .enchantment("minecraft:unbreaking", 3)
    .persistent_tag("weapon_type", "ruby_sword")
    .unbreakable(true)
    .build();

// Instantiate the item anytime
let stack = ruby_sword.create(1);
```

---

## 3. Item Variant Registry (`ItemVariantRegistry`)

Manage a central catalog of custom items for your server or minigame network:

```rust
use potato_api::item_variant::ItemVariantRegistry;

let registry = ItemVariantRegistry::new();

// 1. Register variants
registry.register("mmo:ruby_sword", ruby_sword);

// 2. Create by key
if let Some(item) = registry.create("mmo:ruby_sword", 1) {
    player.give_item(&item);
}

// 3. Match items against registered definitions
if registry.matches(&held_item, "mmo:ruby_sword") {
    player.send_message("§aYou are holding the Ruby Greatsword!");
}

// 4. Identify any matching registered item
if let Some(variant_id) = registry.identify(&held_item) {
    println!("Player held custom item: {}", variant_id);
}
```

---

## 4. Minimal Copy-Paste Plugin Example

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::item_variant::{ItemVariant, ItemVariantRegistry};
use potato_api::event::{BlockBreakEvent, Cancellable};
use potato_api::potato_plugin;

#[derive(Default)]
pub struct CustomItemPlugin;

impl Plugin for CustomItemPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("CustomItemPlugin", "1.0.0", "Flaxa")
    }

    fn on_enable(&mut self, context: &mut PluginContext) -> Result<(), String> {
        let registry = ItemVariantRegistry::new();

        // 1. Define custom Emerald Dagger
        let emerald_dagger = ItemVariant::builder("minecraft:iron_sword")
            .custom_model_data(3001)
            .name("§a§lEmerald Dagger")
            .lore(["§7Fast attack speed, poisoned tip."])
            .enchantment("minecraft:sharpness", 4)
            .persistent_tag("rarity", "legendary")
            .build();

        registry.register("rpg:emerald_dagger", emerald_dagger);

        // 2. Drop the custom dagger when breaking an emerald ore
        context.register_event(move |event: &mut BlockBreakEvent| {
            if event.block.is_type("emerald_ore") {
                event.set_drop_items(false); // Cancel vanilla drop
                if let Some(dagger) = registry.create("rpg:emerald_dagger", 1) {
                    if let Some(player) = &event.player {
                        player.give_item(&dagger);
                        player.send_message("§a★ You found an §2Emerald Dagger§a!");
                    }
                }
            }
        });

        Ok(())
    }
}

potato_plugin!(CustomItemPlugin);
```
