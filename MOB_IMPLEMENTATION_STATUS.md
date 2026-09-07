# Mob Implementation Status

> **Key:** [x] Done | [~] Partial | [ ] Missing | N/A Not Applicable

> **Columns:** Attrs | Goals | Targets | Env | Loot | XP | Sounds | Spawn | Breed/Tame | Special

Sources of truth (priority order):
1. Decompiled vanilla 26.2: server-d/decompiled
2. Vanilla data files (loot tables, sounds): server-d/decompiled/data/
3. orignal-details/minecraft_mob_reference.xlsx - orientation only

**Notes:**
- Attributes: extracted into generated entity_type.rs - [x] for all unless code actively wrong
- XP: stored in entity_type.experience_reward (generated) - [x] for all unless wrong
- Loot: JSON files in data/minecraft/loot_table/entities/ - [x] if loot path correct
- Sounds: all mobs marked [~] - wiring audit needed across the board
- Spawn: all mobs marked [?] - spawn condition audit deferred

## Legend
- [x] = Fully done / verified correct
- [~] = Partially implemented
- [ ] = Not implemented at all
- N/A = Not applicable
- [?] = Needs audit

---

## Passive Mobs

| # | Mob | Attrs | Goals | Targets | Env | Loot | XP | Sounds | Spawn | Breed/Tame | Special | Notes |
|---|-----|-------|-------|---------|-----|------|----|--------|-------|------------|---------|-------|
| 1 | Allay | [x] | [~] | N/A | [~] | N/A | N/A | [~] | [?] | N/A | [ ] | Item-tracking/delivery; jukebox duplication missing |
| 2 | Armadillo | [x] | [x] | N/A | [~] | [~] | N/A | [~] | [?] | N/A | [~] | Ball-state present; scute shedding needs audit |
| 3 | Axolotl | [x] | [x] | [x] | [~] | [x] | N/A | [~] | [?] | [x] | [~] | Play-dead present; land-flop & air-breach audit needed |
| 4 | Bat | [x] | N/A | N/A | [x] | N/A | N/A | [x] | [x] | N/A | [x] | Roost/fly AI in mob_tick - looks complete |
| 5 | Camel | [x] | [x] | N/A | [~] | N/A | N/A | [~] | [?] | N/A | [~] | Two-rider, dash mechanic needs audit |
| 6 | Cat | [x] | [x] | N/A | [~] | [x] | N/A | [~] | [x] | [x] | [x] | 11 breeds rolled randomly at spawn, tracker/NBT synced, scare-creeper/phantom [x] |
| 7 | Chicken | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | [x] | [x] | Fall-immunity OK; egg-lay OK; chicken-jockey [x] |
| 8 | Cod | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | School swim, land-flop, flop/swim sounds [x] |
| 9 | Cow | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | [x] | [~] | Milking OK; mooshroom in separate file |
| 10 | Donkey | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | [x] | [~] | Chest inventory needs audit |
| 11 | Fox | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | [x] | [x] | Red/Snow variants rolled at spawn, sleep/item-hold/pounce [x] |
| 12 | Frog | [x] | [x] | [x] | [~] | N/A | N/A | [~] | [x] | [x] | [x] | Temperate/Warm/Cold variants rolled at spawn, tongue-shoot goal [x] |
| 13 | Glow Squid | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Squid swim + dark glow-stop on damage + glow ink spray [x] |
| 14 | Horse | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | [x] | [~] | Taming/armor/saddle needs audit |
| 15 | Mooshroom | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [x] | [x] | [x] | Red/Brown types, lightning conversion Red<->Brown, stew milking, shearing [x] |
| 16 | Mule | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Chest mechanic needs audit |
| 17 | Ocelot | [x] | [x] | N/A | [~] | N/A | N/A | [~] | [?] | [x] | [~] | No mob_tick; trust-gain needs audit |
| 18 | Panda | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | [x] | [x] | 7 personality genotypes (recessive Brown/Aggressive/Weak), Aggressive Panda melee AI & target [x] |
| 19 | Parrot | [x] | [x] | N/A | [x] | N/A | N/A | [x] | [?] | N/A | [x] | Sounds, imitations, shoulder-riding, dance, 5 cosmetic colors [x] |
| 20 | Pig | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [x] | [x] | [x] | Lightning conversion to Zombified Piglin, saddle steering [x] |
| 21 | Pufferfish | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | 3-state puff machine, poison touch, inflate/deflate sounds [x] |
| 22 | Rabbit | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | [x] | [x] | 6 coat variants + Killer Bunny (hostile melee AI, high damage/armor, player/wolf/fox target) [x] |
| 23 | Salmon | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | School swim, land-flop, flop/swim sounds, size variants [x] |
| 24 | Sheep | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [x] | [x] | [x] | 6 natural wool colors with vanilla weights (Pink 0.164%), EatGrassGoal, wool regrowth, dye colors [x] |
| 25 | Skeleton Horse | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Trap-spawn/lightning conversion needs audit |
| 26 | Sniffer | [x] | [x] | N/A | [~] | N/A | [x] | [~] | [?] | [x] | [~] | Sniff/dig/plant-seed - large file, needs audit |
| 27 | Snow Golem | [x] | [x] | [x] | [~] | N/A | N/A | [~] | [?] | N/A | [~] | Snow-trail, melt-in-rain/desert needs audit |
| 28 | Squid | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Swim AI, ink spray, swim/squirt/hurt/death sounds [x] |
| 29 | Strider | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | [x] | [~] | Lava-walk, warped-fungus steering needs audit |
| 30 | Tadpole | [x] | [x] | N/A | [~] | N/A | N/A | [~] | [?] | N/A | [~] | Grow-into-frog mechanic needs audit |
| 31 | Tropical Fish | [x] | [x] | N/A | [x] | [x] | [x] | [x] | [?] | N/A | [x] | School swim, flop, patterns & colors, sounds [x] |
| 32 | Turtle | [x] | [x] | N/A | [~] | [x] | [x] | [~] | [?] | [x] | [~] | Egg-laying/hatching, home-beach needs audit |
| 33 | Villager | [x] | [x] | N/A | [~] | [x] | N/A | [~] | [x] | [x] | [x] | 7 biome variants & random profession at spawn, tracker synced, lightning to Witch conversion [x] |
| 34 | Wandering Trader | [x] | [x] | N/A | [~] | N/A | N/A | [~] | [?] | [?] | N/A | 1284 lines; trade/llama needs audit |

---

## Neutral Mobs

| # | Mob | Attrs | Goals | Targets | Env | Loot | XP | Sounds | Spawn | Breed/Tame | Special | Notes |
|---|-----|-------|-------|---------|-----|------|----|--------|-------|------------|---------|-------|
| 35 | Bee | [x] | [x] | [x] | [~] | [x] | N/A | [~] | [?] | [x] | [~] | Pollination, hive-sleep, honey needs audit |
| 36 | Dolphin | [x] | [x] | [~] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Treasure-find, swim-with-player, jump needs audit |
| 37 | Enderman | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Block-carry, teleport-dodge, stare-provoke needs audit |
| 38 | Goat | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | [x] | [x] | Screaming variant (2% spawn roll), screaming audio, ram-charge timer [x] |
| 39 | Iron Golem | [x] | [x] | [x] | [~] | [x] | N/A | [~] | [?] | N/A | [~] | Flower offering, crack texture needs audit |
| 40 | Llama | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Caravan-follow, spit, chest needs audit |
| 41 | Piglin | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Brain/Sensor AI; gold-barter - 706 lines |
| 42 | Piglin Brute | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Always-hostile regardless of gold |
| 43 | Polar Bear | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Defend-cub goal needs audit |
| 44 | Spider | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Light-level-gated hostility needs audit |
| 45 | Trader Llama | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Wanders with trader needs audit |
| 46 | Wolf | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | [x] | [x] | 9 coat variants (biome/random roll), tracker & NBT synced, taming/armor [x] |

---

## Hostile Mobs

| # | Mob | Attrs | Goals | Targets | Env | Loot | XP | Sounds | Spawn | Breed/Tame | Special | Notes |
|---|-----|-------|-------|---------|-----|------|----|--------|-------|------------|---------|-------|
| 47 | Blaze | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Fireball-burst OK; fly-idle needs audit |
| 48 | Bogged | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | N/A | [x] | Tipped arrow shooting (Poison 100 ticks), shearing mechanic drops 4 mushrooms, sheared data sync [x] |
| 49 | Breeze | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Wind charge attack, long jump trajectory, sliding, projectile deflection, whirl/jump sounds [x] |
| 50 | Cave Spider | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Poison-on-hit needs audit |
| 51 | Creaking | [x] | [x] | [x] | [x] | N/A | N/A | [x] | [?] | N/A | [x] | Creaking heart bound, freeze on LOS, invulnerability, unfreeze/freeze/sway sounds [x] |
| 52 | Creeper | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [x] | Timed-fuse, cancel-on-retreat well-implemented |
| 53 | Drowned | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [x] | N/A | [x] | Trident ranged attack goal (3D trajectory & sounds), 6.25% trident / 3% nautilus equipment spawn [x] |
| 54 | Elder Guardian | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Mining-Fatigue aura needs audit |
| 55 | Endermite | [x] | [x] | [x] | [~] | N/A | [x] | [~] | [?] | N/A | [~] | Despawn timer needs audit |
| 56 | Evoker | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Vex-summon, fangs, wololo - 620 lines |
| 57 | Ghast | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Fireball deflect, charge/shoot - 464 lines |
| 58 | Giant | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | N/A | Command-only; stub acceptable |
| 59 | Guardian | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Laser-beam windup needs audit |
| 60 | Hoglin | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Warped-fungus fear, zombification needs audit |
| 61 | Husk | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [x] | N/A | [x] | Unarmed melee Hunger effect (140 * diff), sunlight immunity, 600-tick water conversion to Zombie [x] |
| 62 | Magma Cube | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Split-on-death at tick 20, fire immune, lava jump, size scaling [x] |
| 63 | Phantom | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Swoop-dive-reset cycle, circle anchor, attack player, sounds [x] |
| 64 | Pillager | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Crossbow patrol/raid logic needs audit |
| 65 | Ravager | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Roar/stun, crop destruction needs audit |
| 66 | Shulker | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Open/close two-state, teleport - 531 lines |
| 67 | Silverfish | [x] | [x] | [x] | [~] | N/A | [x] | [~] | [?] | N/A | [~] | Hide-in-block, alarm-call needs audit |
| 68 | Skeleton | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Ranged kiter; flee-in-sun needs audit |
| 69 | Slime | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Split-on-death at tick 20, size scaling, sounds, offset spawn [x] |
| 70 | Stray | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [x] | N/A | [x] | Tipped arrow shooting (Slowness 600 ticks), base skeleton exposed, NBT roundtrip [x] |
| 71 | Vex | [x] | [x] | [x] | [x] | N/A | [x] | [x] | [?] | N/A | [x] | Wall-phasing, lifespan timer, owner-target, charge attack [x] |
| 72 | Vindicator | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Johnny flag; no mob_tick |
| 73 | Warden | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [?] | N/A | [x] | Brain/Vibration/Anger, sniffing, sonic boom, emerge/dig [x] |
| 74 | Witch | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Potion-throw, self-heal - 334 lines |
| 75 | Wither Skeleton | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Wither-on-hit needs audit |
| 76 | Zoglin | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Attack-everything; no mob_tick |
| 77 | Zombie | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Reinforcement-call, door-break, baby & chicken-jockey [x] |
| 78 | Zombie Villager | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | N/A | [x] | Random type & profession at spawn, data tracker synced, NBT persistence, cure preserves type/profession [x] |
| 79 | Zombified Piglin | [x] | [x] | [x] | [x] | [x] | [x] | [~] | [?] | N/A | [~] | Mass-aggro group-anger needs audit |
| 80 | Illusioner | [x] | [x] | [x] | [~] | [x] | [x] | [~] | [?] | N/A | [~] | Low priority; blindness + mirror-clone |

---

## Boss Mobs

| # | Mob | Attrs | Goals | Targets | Env | Loot | XP | Sounds | Spawn | Breed/Tame | Special | Notes |
|---|-----|-------|-------|---------|-----|------|----|--------|-------|------------|---------|-------|
| 81 | Wither | [x] | [x] | [x] | [x] | [x] | [x] | [~] | N/A | N/A | [~] | Multi-head targeting, shield needs audit |
| 82 | Ender Dragon | [x] | [x] | [x] | [x] | [x] | [x] | [x] | N/A | N/A | [x] | Full 12-phase state machine, crystal healing beam tracking, breath clouds, dying sequence, wing flap audio [x] |

---

## Work Order

### Completed:
- Tier-1 Passive Stubs: Cod, Salmon, Tropical Fish, Squid, Glow Squid, Pufferfish, Parrot [x]
- Tier-1 Hostile Stubs: Phantom, Vex, Warden [x]
- Partial mob behavior gaps: Slime & Magma Cube split/sounds, Sheep wool regrowth & eat grass, Zombie Villager cure, Chicken Jockey [x]
- High-Priority Hostile & Boss Mobs: Breeze (wind charge, long jump, slide, deflect, sounds), Creaking (heart-bound, freeze on look, invulnerability), Ender Dragon (12 phases, crystal healing beam, breath attacks, dying sequence) [x]
- Hostile Ranged & Melee Audit: Bogged (tipped arrows, mushroom shearing), Stray (slowness tipped arrows, base skeleton exposure, NBT), Husk (hunger effect, water conversion), Drowned (trident ranged attack goal, equipment spawn) [x]
- Mob Variants & Random Spawning (`orignal-details/variants.md`): Cat (11 breeds), Sheep (weighted natural colors, Pink 0.164%), Panda (7 personality genotypes, Aggressive Panda AI), Rabbit (natural colors + Killer Bunny behavior), Wolf (9 variants), Frog (3 variants), Fox (2 variants), Goat (screaming 2%), Mooshroom (lightning strike Red<->Brown), Pig (lightning strike to Zombified Piglin), Villager (7 biomes + professions, lightning strike to Witch), Zombie Villager (biome types, professions, NBT & cure retention) [x]
- Spawn Eggs & Command Summon: Random variant roll on egg spawn and summon command, variant-by-name support [x]

### Next Phase:
- Remaining Mob Audit passes: Witch self-healing/potions, Shulker peek/teleport/bullet tracking, Ghast fireball deflect tuning
