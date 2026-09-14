# Persistent Data Container (PDC) & ItemStacks

Bukkit and Paper introduced the **Persistent Data Container (PDC)** to allow plugins to store arbitrary, typed metadata on items, entities, and blocks without unsafe raw NBT manipulation. PotatoMC natively implements PDC in Rust.

---

## 1. Creating & Customizing `ItemStack`

```rust
use potato_api::types::ItemStack;

let mut item = ItemStack::new("minecraft:diamond_sword", 1);

// Set custom display name
item.set_custom_name("§6§lBlade of the Sun");

// Add lore lines
item.add_lore("§7Forged in ancient fires");
item.add_lore("§eRequires Level 40");

// Add enchantments
item.add_enchantment("minecraft:sharpness", 5);
item.add_enchantment("minecraft:unbreaking", 3);
```

---

## 2. Using the Persistent Data Container (PDC)

The `PersistentDataContainer` provides type-safe storage for:
- `String`
- `i32` (Integer)
- `i64` (Long)
- `Vec<u8>` (Byte array / Raw binary)

```rust
use potato_api::types::PersistentDataContainer;

let mut pdc = PersistentDataContainer::new();

// Store typed values
pdc.set_string("rpg:rarity", "MYTHIC");
pdc.set_int("rpg:power_level", 9001);
pdc.set_long("rpg:creation_timestamp", 1726000000);
pdc.set_bytes("rpg:signature", vec![0xDE, 0xAD, 0xBE, 0xEF]);

// Attach to item
item.set_pdc(pdc);
```

### Reading Stored Data
```rust
if let Some(rarity) = item.pdc().get_string("rpg:rarity") {
    println!("Found item rarity: {}", rarity);
}

if let Some(power) = item.pdc().get_int("rpg:power_level") {
    println!("Power rating: {}", power);
}

// Checking key existence
if item.pdc().has("rpg:signature") {
    // Verified item signature
}
```

---

## 3. Querying Enchantments

```rust
// Check if enchantment exists
if item.has_enchantment("minecraft:sharpness") {
    let level = item.get_enchantment_level("minecraft:sharpness").unwrap_or(0);
    println!("Sharpness level: {}", level);
}
```
