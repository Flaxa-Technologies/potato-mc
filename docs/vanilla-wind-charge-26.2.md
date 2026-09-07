# Vanilla Minecraft Java Edition 26.2: Wind Charge Specification

## Authority
- Primary Authority: Local decompiled Minecraft Java Edition 26.2 server source code at `server-d/decompiled`.
- Classes Traced:
  - `net.minecraft.world.item.WindChargeItem`
  - `net.minecraft.world.item.Items` (line 1474)
  - `net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.WindCharge`
  - `net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.BreezeWindCharge`
  - `net.minecraft.world.entity.projectile.hurtingprojectile.windcharge.AbstractWindCharge`
  - `net.minecraft.world.level.ServerExplosion`
  - `net.minecraft.server.level.ServerPlayer` (lines 1304–1309)
  - `net.minecraft.world.entity.LivingEntity` (lines 1787–1845)

---

## 1. Item Pipeline & Player Use

### 1.1 Trigger and Item Use (`WindChargeItem.java:27–54`)
- **Method**: `use(Level level, Player player, InteractionHand hand)`
- **Spawn Position**:
  ```java
  new WindCharge(player, level, player.position().x(), player.getEyePosition().y(), player.position().z())
  ```
  Spawns horizontally centered on player at **eye level** (`player.getEyePosition().y()`).
- **Velocity Vector**:
  ```java
  Projectile.spawnProjectileFromRotation(..., serverLevel, stack, player, 0.0F, 1.5F, 1.0F);
  ```
  - Speed (`power`): `1.5F`
  - Divergence / Uncertainty: `1.0F`
  - Additional Pitch Offset: `0.0F`
- **Sound on Throw**:
  - Sound ID: `minecraft:entity.wind_charge.throw` (`SoundEvents.WIND_CHARGE_THROW`)
  - Source: `SoundSource.NEUTRAL`
  - Position: `(player.getX(), player.getY(), player.getZ())`
  - Volume: `0.5F`
  - Pitch: `0.4F / (random.nextFloat() * 0.4F + 0.8F)` (range ~0.33 to 0.50)
- **Cooldown**:
  - Registered in `Items.java:1474`: `new Item.Properties().useCooldown(0.5F)`
  - Duration: **0.5 seconds** (10 game ticks)
  - Applied to cooldown group `minecraft:wind_charge`.
- **Item Consumption**:
  - `stack.consume(1, player)`: Consumes 1 item unless in Creative mode.
  - Awards stat: `Stats.ITEM_USED.get(this)`.
  - Player hand swing: Hand animation is triggered upon use.

---

## 2. Projectile Physics & Collision

### 2.1 Physics (`AbstractWindCharge.java`)
- **Gravity**: `0.0` (does not fall due to gravity; `accelerationPower = 0.0`).
- **Drag / Inertia**:
  - `getInertia()`: `1.0F` (no drag in air)
  - `getLiquidInertia()`: `1.0F` (no drag in water/lava)
- **Fire**: `shouldBurn()`: `false`
- **Deflection Protection**:
  - `noDeflectTicks = 5`: Projectile cannot be deflected during its first 5 ticks of flight.
  - At tick count < 2: Hidden if distance < 3.5 squared to avoid camera clipping.

### 2.2 Entity Hit (`AbstractWindCharge.java:78–94`)
- Direct hit deals `1.0` damage:
  - Damage Source: `damageSources().windCharge(this, owner)` (`DamageType.WIND_CHARGE`).
  - Damage Amount: `1.0F` (half a heart).
  - Triggers post-attack effects (e.g. thorns, enchantments).
- Immediately calls `explode(this.position())`.
- Discards projectile.

### 2.3 Block Hit (`AbstractWindCharge.java:103–113`)
- Computes offset along collision normal:
  ```java
  Vec3i collisionNormal = hitResult.getDirection().getUnitVec3i();
  Vec3 scaledNormal = Vec3.atLowerCornerOf(collisionNormal).multiply(0.25, 0.25, 0.25);
  Vec3 explosionPos = hitResult.getLocation().add(scaledNormal);
  ```
- Calls `explode(explosionPos)` and discards projectile.

---

## 3. Wind Burst Explosion Mechanics

### 3.1 Constants & Parameters
| Parameter | Player Wind Charge (`WindCharge.java`) | Breeze Wind Charge (`BreezeWindCharge.java`) |
|---|---|---|
| **Radius** | `1.2F` | `3.0F` |
| **Explosion Sound** | `minecraft:entity.wind_charge.wind_burst` | `minecraft:entity.breeze.wind_burst` |
| **Block Interaction** | `Level.ExplosionInteraction.TRIGGER` | `Level.ExplosionInteraction.TRIGGER` |
| **Explosion Calculator** | `SimpleExplosionDamageCalculator(true, false, Optional.of(1.22F), #blocks_wind_charge_explosions)` | `SimpleExplosionDamageCalculator(true, false, Optional.empty(), #blocks_wind_charge_explosions)` |
| **Damages Entities** | `false` | `false` |
| **Explodes Blocks** | `true` (triggers interactables) | `true` (triggers interactables if mob griefing) |
| **Knockback Multiplier** | `1.22F` | `1.0F` |
| **Particles** | `gust_emitter_small`, `gust_emitter_large` | `gust_emitter_small`, `gust_emitter_large` |

### 3.2 Knockback Vector Calculation (`ServerExplosion.java:184–207`)
```java
double dist = Math.sqrt(entity.distanceToSqr(this.center)) / doubleRadius;
if (!(dist > 1.0)) {
   Vec3 entityOrigin = entity instanceof PrimedTnt ? entity.position() : entity.getEyePosition();
   Vec3 direction = entityOrigin.subtract(this.center).normalize();
   float exposure = getSeenPercent(this.center, entity);
   double knockbackResistance = entity instanceof LivingEntity livingEntity
      ? livingEntity.getAttributeValue(Attributes.EXPLOSION_KNOCKBACK_RESISTANCE)
      : 0.0;
   double knockbackPower = (1.0 - dist) * exposure * knockbackMultiplier * (1.0 - knockbackResistance);
   Vec3 knockback = direction.scale(knockbackPower);
   entity.push(knockback);
   if (entity instanceof Player player) {
      hitPlayers.put(player, knockback);
   }
   entity.onExplosionHit(this.source);
}
```

### 3.3 Upward Player Launch Physics
- Because `entityOrigin` uses `entity.getEyePosition()`, when a player throws a wind charge downward at their feet (`center.y <= pos.y`), `direction.y = (eye_y - center.y)` is **positive**.
- Normalized `direction` points strongly upwards (typically `direction.y >= 0.7`).
- With `knockbackMultiplier = 1.22`, the vertical impulse launches the player ~8–9 blocks into the air.

---

## 4. Fall Damage Suppression / Grace Period

### 4.1 State Stored (`LivingEntity.java:1814–1845`)
- When explosion hits a player (`ServerPlayer.java:1308`):
  ```java
  this.setIgnoreFallDamageFromCurrentImpulse(explosionCausedBy != null && explosionCausedBy.is(EntityTypes.WIND_CHARGE), this.position());
  ```
- Stored state:
  - `currentImpulseImpactPos = position` (the position where the player was when struck by the impulse).
  - `currentImpulseContextResetGraceTime = 40` (ticks = 2 seconds).

### 4.2 Grace Ticking (`LivingEntity.java:2867–2868`)
- Every living entity tick:
  ```java
  if (this.currentImpulseContextResetGraceTime > 0) {
     this.currentImpulseContextResetGraceTime--;
  }
  ```
- Anti-cheat integration (`ServerGamePacketListenerImpl.java:1150`):
  `!this.player.isInPostImpulseGraceTime()` suppresses invalid movement flags while airborne from impulse.

### 4.3 Fall Damage Cancellation Formula (`LivingEntity.java:1788–1804`)
```java
double effectiveFallDistance;
if (this.isIgnoringFallDamageFromCurrentImpulse()) {
   effectiveFallDistance = Math.min(fallDistance, this.currentImpulseImpactPos.y - this.getY());
   boolean hasLandedAboveCurrentImpulseImpactPosY = effectiveFallDistance <= 0.0;
   if (hasLandedAboveCurrentImpulseImpactPosY) {
      this.resetCurrentImpulseContext();
   } else {
      this.tryResetCurrentImpulseContext();
   }
} else {
   effectiveFallDistance = fallDistance;
}
```
- If the player lands **at or above** `currentImpulseImpactPos.y`:
  `effectiveFallDistance <= 0.0` -> **ZERO fall damage is taken**.
- If the player falls **below** `currentImpulseImpactPos.y` (e.g. into a ravine):
  Fall distance only counts from `currentImpulseImpactPos.y` downward!

---

## 5. Block Interactions (`Level.ExplosionInteraction.TRIGGER`)
Blocks affected by wind charge trigger interactions:
- `DoorBlock`: Toggles open/closed if non-iron (`type.canOpenByWindCharge()`).
- `TrapDoorBlock`: Toggles open/closed if non-iron.
- `FenceGateBlock`: Toggles open/closed.
- `ButtonBlock`: Activates button (powered = true) if not already powered.
- `LeverBlock`: Toggles lever state.
- `BellBlock`: Rings the bell.
- `AbstractCandleBlock`: Extinguishes lit candle.
- Immune blocks: Tag `#minecraft:blocks_wind_charge_explosions`.

---

## 6. Dispenser Behavior (`WindChargeItem.java:73–80`)
- Dispense config:
  - Override event: `1051` (`LevelEvent.SOUND_DISPENSER_PROJECTILE_LAUNCH`).
  - Power: `1.0F`.
  - Uncertainty: `6.6666665F`.
  - Position: `DispenserBlock.getDispensePosition(source, 1.0, Vec3.ZERO)`.
