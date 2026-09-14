# NPC & Fake Player API

PotatoMC provides a native, low-overhead NPC and Fake Player system. Plugins can spawn custom player or mob NPCs, control skin textures, poses, and equipment, listen for player clicks, command pathfinding navigation, and attach rich custom behaviors without touching core engine code.

---

## 1. Spawning a Fake Player NPC

Spawn a fake player NPC with custom skin and pose using the fluent `Npc::builder`:

```rust
use potato_api::npc::{EntityPose, Npc, SkinData};
use potato_api::Location;

let spawn_loc = Location::new("minecraft:overworld", 100.5, 64.0, 200.5, 90.0, 0.0);

// Optional Mojang skin texture value & signature
let skin = SkinData::new("eyJ0ZXh0dXJlcyI6...", None);

let npc = Npc::builder("Quest Master")
    .player()
    .skin(skin)
    .location(spawn_loc)
    .pose(EntityPose::Standing)
    .glowing(true)
    .invulnerable(true)
    .spawn(context);

println!("Spawned NPC with ID: {}", npc.id());
```

---

## 2. Spawning Mob / Entity NPCs

NPCs can also render as standard Minecraft entities (e.g. Villagers, Iron Golems, Armor Stands):

```rust
let shop_npc = Npc::builder("Blacksmith")
    .entity("minecraft:villager")
    .location(spawn_loc)
    .invulnerable(true)
    .spawn(context);
```

---

## 3. Controlling NPC Poses, Skins & Equipment

Dynamically adjust the NPC's state at any time:

```rust
use potato_api::npc::EntityPose;
use potato_api::types::{EquipmentSlot, ItemStack};

// Update pose
npc.set_pose(EntityPose::Sneaking); // or Sitting, Sleeping, Swimming, etc.

// Equip items
let sword = ItemStack::new("minecraft:diamond_sword", 1);
npc.set_equipment(EquipmentSlot::MainHand, Some(sword));

// Teleport or navigate
let new_loc = Location::new("minecraft:overworld", 110.0, 64.0, 200.0, 0.0, 0.0);
npc.teleport(&new_loc);
npc.move_to(&new_loc, 1.2); // Waypoint pathing with speed multiplier
```

---

## 4. Handling Click Interactions (`NpcInteractEvent`)

Listen for player left-click (attack) or right-click (interact) on any NPC:

```rust
use potato_api::event::{NpcInteractEvent, Cancellable};
use potato_api::npc::NpcClickType;

context.register_event(|event: &mut NpcInteractEvent| {
    match event.click_type {
        NpcClickType::RightClick => {
            event.player.send_message(&format!("§aInteracted with NPC ID #{}", event.npc_id));
        }
        NpcClickType::LeftClick => {
            event.set_cancelled(true); // Prevent PvP knockback
            event.player.send_message("§cYou cannot attack this NPC!");
        }
    }
});
```

---

## 5. Attaching Custom Behavior (`NpcBehavior`)

Plugins can attach arbitrary lifecycle and AI behaviors to an NPC without modifying Pumpkin core code:

```rust
use potato_api::npc::{Npc, NpcBehavior, NpcClickType};
use potato_api::player::Player;

struct ShopkeeperBehavior {
    trade_count: u32,
}

impl NpcBehavior for ShopkeeperBehavior {
    fn on_spawn(&mut self, npc: &Npc) {
        println!("Shopkeeper NPC {} spawned!", npc.name());
    }

    fn on_interact(&mut self, npc: &Npc, player: &Player, click: NpcClickType) {
        if click == NpcClickType::RightClick {
            self.trade_count += 1;
            player.send_message(&format!("§e[Shop] Welcome! Total visitors: {}", self.trade_count));
            // e.g. player.open_gui(&shop_gui);
        }
    }

    fn on_tick(&mut self, npc: &Npc) {
        // e.g. Periodically look around or play particle effects
    }

    fn on_despawn(&mut self, npc: &Npc) {
        println!("Shopkeeper NPC {} removed", npc.name());
    }
}
```

---

## 6. Minimal Copy-Paste Plugin Example

```rust
use potato_api::plugin::{Plugin, PluginContext, PluginMetadata};
use potato_api::npc::{EntityPose, Npc, NpcBehavior, NpcClickType};
use potato_api::event::NpcInteractEvent;
use potato_api::Location;
use potato_api::potato_plugin;

struct WelcomeNpcBehavior;

impl NpcBehavior for WelcomeNpcBehavior {
    fn on_interact(&mut self, npc: &Npc, player: &potato_api::Player, click: NpcClickType) {
        if click == NpcClickType::RightClick {
            player.send_title("§6Welcome!", "§eEnjoy your stay on PotatoMC", 10, 40, 10);
            player.send_message(&format!("§e[{}] §fNice to meet you, {}!", npc.name(), player.name()));
        }
    }
}

#[derive(Default)]
pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("NpcPlugin", "1.0.0", "Flaxa")
    }

    fn on_enable(&mut self, context: &mut PluginContext) -> Result<(), String> {
        let spawn = Location::new("minecraft:overworld", 0.5, 65.0, 0.5, 180.0, 0.0);

        // Spawn a welcome guide NPC
        let npc = Npc::builder("Server Guide")
            .player()
            .location(spawn)
            .pose(EntityPose::Standing)
            .glowing(true)
            .behavior(Box::new(WelcomeNpcBehavior))
            .spawn(context);

        println!("Spawned Guide NPC with ID {}", npc.id());
        Ok(())
    }
}

potato_plugin!(NpcPlugin);
```
