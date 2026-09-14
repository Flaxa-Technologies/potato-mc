# Vanilla Minecraft Java Edition 26.2: Mace & Wind Charge Sound Registry

All sound events verified from local decompiled 26.2 server source (`server-d/decompiled`) and resource data.

---

## Mace Sounds

| Event / Trigger | Sound ID | Source Entity / Category | Volume | Pitch | Exact Vanilla Source |
|---|---|---|---|---|---|
| **Smash in Air** (Victim not on ground) | `minecraft:item.mace.smash_air` | `attacker.getSoundSource()` (`PLAYERS`) | `1.0F` | `1.0F` | `MaceItem.java:68` |
| **Smash on Ground** (Victim on ground, `fallDistance <= 5.0`) | `minecraft:item.mace.smash_ground` | `attacker.getSoundSource()` (`PLAYERS`) | `1.0F` | `1.0F` | `MaceItem.java:65-66` |
| **Heavy Smash on Ground** (Victim on ground, `fallDistance > 5.0`) | `minecraft:item.mace.smash_ground_heavy` | `attacker.getSoundSource()` (`PLAYERS`) | `1.0F` | `1.0F` | `MaceItem.java:65-66` |
| **Normal Attack Swing** | `minecraft:entity.player.attack.strong` | `PLAYERS` | `1.0F` | `1.0F` | `Player.java` |
| **Normal Attack Weak** | `minecraft:entity.player.attack.weak` | `PLAYERS` | `1.0F` | `1.0F` | `Player.java` |
| **Normal Attack Crit** | `minecraft:entity.player.attack.crit` | `PLAYERS` | `1.0F` | `1.0F` | `Player.java` |
| **Normal Attack Knockback** | `minecraft:entity.player.attack.knockback` | `PLAYERS` | `1.0F` | `1.0F` | `Player.java` |
| **Normal Attack No Damage** | `minecraft:entity.player.attack.nodamage` | `PLAYERS` | `1.0F` | `1.0F` | `Player.java` |
| **Mace Breaks** | `minecraft:entity.item.break` | `PLAYERS` | `0.8F` | `0.8F + random * 0.4F` | `LivingEntity.java` |
| **Wind Burst Enchantment Impact** | `minecraft:entity.wind_charge.wind_burst` | `NEUTRAL` / `PLAYERS` | `1.0F` | `1.0F` | `wind_burst.json:34` |

---

## Wind Charge Sounds

| Event / Trigger | Sound ID | Source Entity / Category | Volume | Pitch | Exact Vanilla Source |
|---|---|---|---|---|---|
| **Player Throws Wind Charge** | `minecraft:entity.wind_charge.throw` | `SoundSource.NEUTRAL` | `0.5F` | `0.4F / (random.nextFloat() * 0.4F + 0.8F)` | `WindChargeItem.java:46-50` |
| **Player Wind Charge Burst** | `minecraft:entity.wind_charge.wind_burst` | `SoundSource.NEUTRAL` (in `explode()`) | `1.0F` | `1.0F` | `WindCharge.java:74` |
| **Breeze Wind Charge Burst** | `minecraft:entity.breeze_wind_charge.burst` (`BREEZE_WIND_CHARGE_BURST`) | `SoundSource.HOSTILE` (in `explode()`) | `1.0F` | `1.0F` | `BreezeWindCharge.java:39` |
| **Dispenser Fires Wind Charge** | Event ID `1051` (`SOUND_DISPENSER_PROJECTILE_LAUNCH`) | `BLOCKS` | `1.0F` | `1.2F` | `WindChargeItem.java:78` |
| **Bell Hit by Wind Charge** | `minecraft:block.bell.use` | `BLOCKS` | `2.0F` | `1.0F` | `BellBlock.java:213` |
| **Door / Gate / Trapdoor Toggled** | Respective open/close sounds | `BLOCKS` | `1.0F` | `1.0F` | Respective block classes |
| **Candle Extinguished** | `minecraft:block.candle.extinguish` | `BLOCKS` | `1.0F` | `1.0F` | `AbstractCandleBlock.java:107` |

---

## Particle Effects

| Event | Particle ID | Count | Velocity | Exact Vanilla Source |
|---|---|---|---|---|
| **Mace Smash Particle Event** | Level event `2013` (data `750`) | Server level event packet | - | `MaceItem.java:116` |
| **Mace Heavy Fall Extra Dust** | `minecraft:dust_plume` / fall particles | Client spawned via `spawnExtraParticlesOnFall` | - | `MaceItem.java:62` |
| **Wind Charge Burst Small** | `minecraft:gust_emitter_small` | 1 | `(0, 0, 0)` | `WindCharge.java:71` |
| **Wind Charge Burst Large** | `minecraft:gust_emitter_large` | 1 | `(0, 0, 0)` | `WindCharge.java:72` |
