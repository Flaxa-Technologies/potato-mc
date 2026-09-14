# Vanilla Minecraft Java Edition 26.2: Mace System Specification

## Authority
- Primary Authority: Local decompiled Minecraft Java Edition 26.2 server source code at `server-d/decompiled`.
- Classes & Data Traced:
  - `net.minecraft.world.item.MaceItem`
  - `net.minecraft.world.item.Items` (lines 1484–1495)
  - `net.minecraft.world.damagesource.CombatRules`
  - `data/minecraft/enchantment/density.json`
  - `data/minecraft/enchantment/breach.json`
  - `data/minecraft/enchantment/wind_burst.json`
  - `data/minecraft/tags/enchantment/exclusive_set/damage.json`
  - `data/minecraft/tags/item/enchantable/mace.json`
  - `data/minecraft/tags/item/enchantable/durability.json`
  - `data/minecraft/recipe/mace.json`

---

## 1. Mace Item Base Properties

| Property | Vanilla 26.2 Value |
|---|---|
| Item ID | `minecraft:mace` |
| Rarity | `EPIC` |
| Max Durability | `500` |
| Repair Material | `minecraft:breeze_rod` |
| Enchantability | `15` |
| Attack Damage | `5.0` (3.5 hearts) |
| Attack Speed | `-3.4F` (0.6 attacks/sec) |
| Durability Loss per Hit | `1` (`DataComponents.WEAPON, new Weapon(1)`) |
| Durability Loss on Miss | `0` |
| Crafting Recipe | Heavy Core (`#`) over Breeze Rod (`I`) in 3x3 crafting grid |

---

## 2. Smash Attack Qualification

```java
public static boolean canSmashAttack(final LivingEntity attacker) {
   return attacker.fallDistance > 1.5 && !attacker.isFallFlying();
}
```
- **Threshold**: Player must have accumulated `fallDistance > 1.5` blocks.
- **Elytra Exclusion**: If `attacker.isFallFlying()` is true, smash attacks are disabled.
- **Damage Source**: If smash qualifies, damage source is `damageSources().mace(attacker)` (`minecraft:mace_smash`), which tags `#minecraft:is_player_attack` and `#minecraft:mace_smash`.

---

## 3. Smash Damage Calculation Formula

From `MaceItem.java:89–113`:
```java
double fallDistance = attacker.fallDistance;
double damage;
if (fallDistance <= 3.0) {
   damage = 4.0 * fallDistance;
} else if (fallDistance <= 8.0) {
   damage = 12.0 + 2.0 * (fallDistance - 3.0);
} else {
   damage = 22.0 + fallDistance - 8.0;
}
```
Plus Density Enchantment:
```java
float totalBonusDamage = (float)(damage + (0.5 * densityLevel * fallDistance));
float finalDamage = baseAttackDamage + totalBonusDamage;
```

### 3.1 Fall Distance to Damage Table (Base Weapon, No Density)
| Fall Distance (blocks) | Smash Bonus Damage | Total Attack Damage (Base 5.0) | Total Damage in Hearts |
|---|---|---|---|
| <= 1.5 | 0.0 (Normal hit) | 5.0 | 2.5 |
| 2.0 | 8.0 | 13.0 | 6.5 |
| 3.0 | 12.0 | 17.0 | 8.5 |
| 4.0 | 14.0 | 19.0 | 9.5 |
| 5.0 | 16.0 | 21.0 | 10.5 |
| 6.0 | 18.0 | 23.0 | 11.5 |
| 7.0 | 20.0 | 25.0 | 12.5 |
| 8.0 | 22.0 | 27.0 | 13.5 |
| 10.0 | 24.0 | 29.0 | 14.5 |
| 15.0 | 29.0 | 34.0 | 17.0 |
| 20.0 | 34.0 | 39.0 | 19.5 |
| 50.0 | 64.0 | 69.0 | 34.5 |

---

## 4. Safe Fall & Momentum Reset

From `MaceItem.java:54–58, 83–85`:
When a smash attack successfully hits:
1. **Vertical Velocity Reset**:
   ```java
   attacker.setDeltaMovement(attacker.getDeltaMovement().with(Direction.Axis.Y, 0.01F));
   ```
   Cancels downward fall velocity and sets vertical speed to slight upward `+0.01F`.
2. **Fall Damage Cancellation**:
   ```java
   attacker.setIgnoreFallDamageFromCurrentImpulse(true, this.calculateImpactPosition(attacker));
   ```
   Where:
   ```java
   private Vec3 calculateImpactPosition(final LivingEntity attacker) {
      return attacker.isIgnoringFallDamageFromCurrentImpulse() && attacker.currentImpulseImpactPos.y <= attacker.position().y
         ? attacker.currentImpulseImpactPos
         : attacker.position();
   }
   ```
3. **Fall Distance Reset**:
   ```java
   attacker.resetFallDistance();
   ```
4. **Client Motion Packet**:
   If attacker is a `ServerPlayer`, `ClientboundSetEntityMotionPacket` is sent immediately.

---

## 5. Nearby Entity Knockback

From `MaceItem.java:115–147`:
- **Radius**: `3.5` blocks centered at victim's bounding box center.
- **Particle Event**: Level event `2013` with data `750` at victim block position.
- **Eligible Entities**:
  - Must be `LivingEntity`.
  - Excludes: `attacker`, `victim`, spectators, allies, tamed pets of attacker, marker armor stands, creative flying players.
- **Knockback Formula**:
  ```java
  Vec3 direction = nearby.position().subtract(victim.position());
  double knockbackPower = (3.5 - direction.length()) * 0.7F * (attacker.fallDistance > 5.0 ? 2 : 1) * (1.0 - nearby.getAttributeValue(Attributes.KNOCKBACK_RESISTANCE));
  Vec3 knockbackVector = direction.normalize().scale(knockbackPower);
  if (knockbackPower > 0.0) {
     nearby.push(knockbackVector.x, 0.7F, knockbackVector.z);
  }
  ```
  - Horizontal knockback falls off with distance.
  - Knockback doubled if `fallDistance > 5.0` (heavy smash).
  - Vertical knockback is **fixed at `+0.7F`**!
  - Syncs motion packet to affected players.

---

## 6. Mace Enchantments

### 6.1 Density (`density.json`)
- Max Level: **V** (5)
- Effect: `minecraft:smash_damage_per_fallen_block`
- Formula: `+ (0.5 * level)` damage per block fallen.
- Exclusive with: `#minecraft:exclusive_set/damage` (Sharpness, Smite, Bane of Arthropods, Impaling, Breach).

### 6.2 Breach (`breach.json`)
- Max Level: **IV** (4)
- Effect: `minecraft:armor_effectiveness`
- Formula: `-0.15 * level` reduction in target armor effectiveness.
  In `CombatRules.java`:
  `modifiedArmorFraction = (armorFraction - 0.15 * level).clamp(0.0, 1.0)`
  - Breach I: -15% armor effectiveness
  - Breach II: -30% armor effectiveness
  - Breach III: -45% armor effectiveness
  - Breach IV: -60% armor effectiveness
- Exclusive with: `#minecraft:exclusive_set/damage` (Density, Smite, Bane of Arthropods, Sharpness).

### 6.3 Wind Burst (`wind_burst.json`)
- Max Level: **III** (3)
- Requirement: Direct attacker `is_flying == false`, `fall_distance >= 1.5`.
- Trigger: Successful hit post-attack.
- Effect:
  - Creates wind burst explosion centered at **attacker** (`position()`).
  - Radius: `3.5`.
  - Block Interaction: `TRIGGER`.
  - Knockback Multipliers:
    - Level I: `1.2`
    - Level II: `1.75`
    - Level III: `2.2`
  - Sound: `minecraft:entity.wind_charge.wind_burst`.
  - Particles: `gust_emitter_small`, `gust_emitter_large`.
  - Upward impulse launches attacker back into the air, enabling chained smash attacks!

### 6.4 Full Compatibility Matrix
| Enchantment | Allowed? | Max Level | Exclusive With | Notes |
|---|---|---|---|---|
| **Density** | Yes | 5 | Breach, Smite, Bane of Arthropods | Mace exclusive |
| **Breach** | Yes | 4 | Density, Smite, Bane of Arthropods | Mace exclusive |
| **Wind Burst** | Yes | 3 | None | Mace exclusive |
| **Unbreaking** | Yes | 3 | None | Supported via `#enchantable/durability` |
| **Mending** | Yes | 1 | None | Supported via `#enchantable/durability` |
| **Fire Aspect** | Yes | 2 | None | Supported via `#enchantable/fire_aspect` |
| **Smite** | Yes | 5 | Density, Breach, Bane of Arthropods | Supported via `#enchantable/weapon` |
| **Bane of Arthropods** | Yes | 5 | Density, Breach, Smite | Supported via `#enchantable/weapon` |
| **Curse of Vanishing** | Yes | 1 | None | Supported via `#enchantable/vanishing` |
| **Sharpness** | **NO** | - | - | Not in `#enchantable/sharp_weapon` |
| **Knockback** | **NO** | - | - | Not supported on Mace |
| **Sweeping Edge** | **NO** | - | - | Swords only |
| **Looting** | **NO** | - | - | Swords only |
