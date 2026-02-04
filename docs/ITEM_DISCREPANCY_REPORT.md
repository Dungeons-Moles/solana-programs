# Item Discrepancy Report: GDD vs Code

Generated: 2026-02-02
Last Updated: 2026-02-03

This report compares all 80 item definitions in `programs/player-inventory/src/items.rs` against the Game Design Document (`specs/gdd.md`).

## Legend

- **FIXED**: Now matches the GDD
- **Match**: Already matched the GDD
- **Values Only**: Name matches but tier values differ
- **Partial**: Some effects match, others don't
- **Mismatch**: Completely different implementation
- **Complex**: GDD requires mechanics not yet in effect system

---

## STONE Tag (T-ST, G-ST)

| ID      | GDD Name            | Code Name           | GDD Effect                                                   | Code Effect                                      | Status                                          |
| ------- | ------------------- | ------------------- | ------------------------------------------------------------ | ------------------------------------------------ | ----------------------------------------------- |
| T-ST-01 | Bulwark Shovel      | Bulwark Shovel      | +1/2/3 ATK, +4/6/8 ARM                                       | +1/2/3 ATK, +4/6/8 ARM                           | **Match**                                       |
| T-ST-02 | Cragbreaker Hammer  | Cragbreaker Hammer  | +2/3/4 ATK, +3/5/7 ARM; first strike removes 1/2/3 enemy ARM | +2/3/4 ATK, +3/5/7 ARM; OnHit RemoveArmor 1/2/3  | **FIXED**                                       |
| G-ST-01 | Miner Helmet        | Miner Helmet        | +3/6/9 ARM                                                   | +3/6/9 ARM                                       | **Match**                                       |
| G-ST-02 | Work Vest           | Work Vest           | +4/8/12 HP, +1 ARM                                           | +4/8/12 MaxHp, +1/1/1 ARM                        | **FIXED**                                       |
| G-ST-03 | Spiked Bracers      | Spiked Bracers      | BattleStart: 2/4/6 Shrapnel                                  | BattleStart: 2/4/6 Shrapnel                      | **FIXED**                                       |
| G-ST-04 | Reinforcement Plate | Reinforcement Plate | EveryOtherTurn: +1/2/3 ARM                                   | EveryOtherTurn: +1/2/3 ARM                       | **FIXED**                                       |
| G-ST-05 | Rebar Carapace      | Rebar Carapace      | Exposed: +3/5/7 ARM                                          | Exposed: +3/5/7 ARM                              | **FIXED**                                       |
| G-ST-06 | Shrapnel Talisman   | Shrapnel Collar     | OnGainShrapnel (1/turn): +1/2/3 ARM                          | TurnStart: +1/2/3 Shrapnel                       | **Complex** - requires "OnGainShrapnel" trigger |
| G-ST-07 | Crystal Crown       | Bastion Plate       | BattleStart: MaxHP = starting ARM (cap 12/18/24)             | BattleStart: +8/12/16 ARM                        | **Complex** - requires ARM-to-HP conversion     |
| G-ST-08 | Stone Sigil         | Adamant Core        | EndOfTurn: if has ARM, +1/2/3 ARM                            | BattleStart: +6/10/14 ARM; TurnStart: +2/3/4 ARM | **Complex** - requires conditional "if has ARM" |

---

## SCOUT Tag (T-SC, G-SC)

| ID      | GDD Name            | Code Name          | GDD Effect                                              | Code Effect                                          | Status                                                |
| ------- | ------------------- | ------------------ | ------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------- |
| T-SC-01 | Twin Picks          | Twin Picks         | +1/2/3 ATK; strike 2x/turn                              | +1/2/3 ATK, +1/1/1 GainStrikes                       | **FIXED**                                             |
| T-SC-02 | Pneumatic Drill     | Pneumatic Drill    | +1/2/3 ATK; strike 3x/turn                              | +1/2/3 ATK, +2/2/2 GainStrikes                       | **FIXED**                                             |
| G-SC-01 | Miner Boots         | Miner Boots        | +2/3/4 DIG                                              | +2/3/4 DIG                                           | **Match**                                             |
| G-SC-02 | Leather Gloves      | Leather Gloves     | +1/2/3 ATK, +1 DIG                                      | +1/2/3 ATK, +1/1/1 DIG                               | **FIXED**                                             |
| G-SC-03 | Tunnel Instinct     | Tunnel Instinct    | BattleStart: if DIG > enemy DIG, +1/2/3 SPD             | BattleStart: +1/2/3 SPD (if DIG > enemy DIG)         | **FIXED**                                             |
| G-SC-04 | Tunneler Spurs      | Sprint Greaves     | +1/2/3 SPD; if first on T1, +1/2/3 DIG                  | BattleStart: +2/3/4 SPD                              | **Complex** - requires FirstTurnIfFaster conditional  |
| G-SC-05 | Wall-Sense Visor    | Wall-Sense Visor   | +1/2/3 DIG; BattleStart: if DIG > enemy DIG, +2/3/4 ARM | +1/2/3 DIG; BattleStart: +2/3/4 ARM (if DIG > enemy) | **FIXED**                                             |
| G-SC-06 | Drill Servo         | Tunnel Runner Belt | Wounded: +1/2/3 strikes                                 | BattleStart: +4/6/8 DIG                              | **Complex** - requires GainStrikes on Wounded trigger |
| G-SC-07 | Weak-Point Manual   | Rapid Excavator    | if DIG > enemy ARM: strikes ignore 1/2/3 ARM            | +3/5/7 DIG, +2/3/4 SPD                               | **Complex** - requires armor piercing mechanic        |
| G-SC-08 | Gear-Link Medallion | Phantom Pickaxe    | OnHit effects trigger 2x (1/turn)                       | +5/8/11 DIG, FirstTurn: +3/4/5 SPD                   | **Complex** - requires effect doubling mechanic       |

---

## GREED Tag (T-GR, G-GR)

| ID      | GDD Name        | Code Name            | GDD Effect                                          | Code Effect                                   | Status                                           |
| ------- | --------------- | -------------------- | --------------------------------------------------- | --------------------------------------------- | ------------------------------------------------ |
| T-GR-01 | Glittering Pick | Glittering Pick      | +1/2/3 ATK; OnHit (1/turn): +1 Gold                 | +1/2/3 ATK; OnHit (1/turn): +1 Gold           | **FIXED**                                        |
| T-GR-02 | Gemfinder Staff | Fortune Finder       | +1 ATK, +1 ARM, +1 DIG; first hit triggers Shards   | +2/3/4 ATK; OnHit: +2/3/4 Gold                | **Complex** - requires Shard triggering mechanic |
| G-GR-01 | Loose Nuggets   | Coin Pouch           | Start of each Day: +3/6/9 Gold                      | BattleStart: +3/5/7 Gold                      | **Complex** - requires Day phase trigger         |
| G-GR-02 | Lucky Coin      | Lucky Coin           | Victory: +2/4/6 Gold                                | Victory: +2/4/6 Gold                          | **FIXED**                                        |
| G-GR-03 | Gilded Band     | Treasure Hunter Belt | BattleStart: ARM = Gold/10 (cap 2/3/4)              | BattleStart: +5/8/11 Gold                     | **Complex** - requires GoldToArmor mechanic      |
| G-GR-04 | Royal Bracer    | Gold Converter       | TurnStart: 1 Gold -> 2/3/4 ARM                      | TurnStart: +1/2/3 ARM, +1/2/3 Gold            | **Complex** - requires Gold consumption mechanic |
| G-GR-05 | Emerald Shard   | Shard Collector      | EveryOtherTurn (first hit): heal 1/2/3              | EveryOtherTurn: +4/6/8 Gold                   | **Complex** - requires "first hit" sub-trigger   |
| G-GR-06 | Ruby Shard      | Wealth Amplifier     | EveryOtherTurn (first hit): 1/2/3 non-weapon damage | BattleStart: +8/12/16 Gold                    | **Complex** - requires "first hit" sub-trigger   |
| G-GR-07 | Sapphire Shard  | Shard Matrix         | EveryOtherTurn (first hit): +1/2/3 ARM              | EveryOtherTurn: +6/9/12 Gold                  | **Complex** - requires "first hit" sub-trigger   |
| G-GR-08 | Citrine Shard   | Midas Touch          | EveryOtherTurn (first hit): +1/2/3 Gold             | OnHit: +3/5/7 Gold; BattleStart: +5/8/11 Gold | **Complex** - requires "first hit" sub-trigger   |

---

## BLAST Tag (T-BL, G-BL)

| ID      | GDD Name          | Code Name           | GDD Effect                                        | Code Effect                                               | Status                                           |
| ------- | ----------------- | ------------------- | ------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------ |
| T-BL-01 | Fuse Pick         | Fuse Pick           | +1/2/3 ATK; first hit: 1 non-weapon damage        | +1/2/3 ATK; OnHit (1/turn): 1 non-weapon damage           | **FIXED**                                        |
| T-BL-02 | Spark Pick        | Demolition Hammer   | +1/2/3 ATK; OnHit (1/turn): reduce Countdown by 1 | +2/3/4 ATK; BattleStart: ApplyBomb 2/3/4                  | **Complex** - requires Countdown manipulation    |
| G-BL-01 | Small Charge      | Fuse Box            | Countdown(2): 8/10/12 damage to both              | BattleStart: ApplyBomb 1/2/3                              | **Mismatch** - values and mechanics differ       |
| G-BL-02 | Blast Suit        | Powder Keg          | Ignore damage from own BLAST items                | BattleStart: 3/5/7 non-weapon damage                      | **Complex** - requires damage immunity mechanic  |
| G-BL-03 | Explosive Powder  | Detonator Belt      | Non-weapon damage +1/2/3                          | BattleStart: ApplyBomb 2/3/4                              | **Complex** - requires damage amplification      |
| G-BL-04 | Double Detonation | Shockwave Gauntlets | 2nd non-weapon damage/turn: +2/3/4                | OnHit: 2/4/6 non-weapon damage                            | **Complex** - requires counting damage instances |
| G-BL-05 | Bomb Satchel      | Bomb Amplifier      | BattleStart: reduce all Countdowns by 1           | BattleStart: ApplyBomb 1/2/3, 2/3/4 non-weapon            | **Complex** - requires Countdown manipulation    |
| G-BL-06 | Kindling Charge   | Chain Reaction Core | BattleStart: 1/2/3 damage; next bomb +3/5/7       | TurnStart: ApplyBomb 1/1/2                                | **Complex** - requires bomb damage amplification |
| G-BL-07 | Time Charge       | Volatile Container  | TurnStart: +1/2/3 stored; Exposed: deal stored    | BattleStart: ApplyBomb 4/6/8                              | **Complex** - requires stored damage mechanic    |
| G-BL-08 | Twin-Fuse Knot    | Nuclear Core        | Bomb triggers happen 2x                           | BattleStart: ApplyBomb 3/5/7; TurnStart: 2/3/4 non-weapon | **Complex** - requires effect doubling           |

---

## FROST Tag (T-FR, G-FR)

| ID      | GDD Name           | Code Name          | GDD Effect                                               | Code Effect                                                  | Status    |
| ------- | ------------------ | ------------------ | -------------------------------------------------------- | ------------------------------------------------------------ | --------- |
| T-FR-01 | Rime Pike          | Rime Pike          | +2/3/4 ATK; OnHit (1/turn): 1 Chill                      | +2/3/4 ATK; OnHit (1/turn): 1 Chill                          | **FIXED** |
| T-FR-02 | Glacier Fang       | Glacier Fang       | +2/3/4 ATK; OnHit: 1 Chill; if enemy has Chill, +1 SPD   | +2/3/4 ATK; OnHit (1/turn): 1 Chill; +1 SPD (if enemy Chill) | **FIXED** |
| G-FR-01 | Frost Lantern      | Frost Lantern      | BattleStart: 1/2/3 Chill                                 | BattleStart: 1/2/3 Chill                                     | **FIXED** |
| G-FR-02 | Frostguard Buckler | Frostguard Buckler | +6/8/10 ARM; BattleStart: if enemy has Chill, +2/3/4 ARM | +6/8/10 ARM; BattleStart: +2/3/4 ARM (if enemy has Chill)    | **FIXED** |
| G-FR-03 | Cold Snap Charm    | Cold Snap Charm    | FirstTurnIfFaster: 2/3/4 Chill                           | FirstTurnIfFaster: 2/3/4 Chill                               | **FIXED** |
| G-FR-04 | Ice Skates         | Ice Skates         | +1/2/3 SPD                                               | +1/2/3 SPD                                                   | **FIXED** |
| G-FR-05 | Rime Cloak         | Rime Cloak         | +3/5/7 ARM; when struck (1/turn): 1 Chill                | +3/5/7 ARM; OnStruck (1/turn): 1 Chill                       | **FIXED** |
| G-FR-06 | Permafrost Core    | Permafrost Core    | TurnStart: if enemy has Chill, +1/2/3 ARM                | TurnStart: +1/2/3 ARM (if enemy has Chill)                   | **FIXED** |
| G-FR-07 | Cold Front Idol    | Cold Front Idol    | EveryOtherTurn: 1 Chill; if enemy has Chill, +1 SPD      | EveryOtherTurn: 1 Chill; +1 SPD (if enemy has Chill)         | **FIXED** |
| G-FR-08 | Deep Freeze Charm  | Deep Freeze Charm  | Wounded: 2/3/4 Chill, -1 enemy SPD                       | Wounded: 2/3/4 Chill; Wounded: ReduceEnemySpd 1              | **FIXED** |

---

## RUST Tag (T-RU, G-RU)

| ID      | GDD Name           | Code Name            | GDD Effect                                                  | Code Effect                                            | Status                                            |
| ------- | ------------------ | -------------------- | ----------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------- |
| T-RU-01 | Corrosive Pick     | Corrosive Pick       | +1/2/3 ATK; OnHit (1/turn): 1 Rust                          | +1/2/3 ATK; OnHit (1/turn): 1 Rust                     | **FIXED**                                         |
| T-RU-02 | Etched Burrowblade | Acid Excavator       | +2/3/4 ATK, +1/2/3 SPD; if enemy has Rust, ignore 1/2/3 ARM | +2/3/4 ATK; OnHit: 2/3/4 Rust                          | **Complex** - requires conditional armor piercing |
| G-RU-01 | Oxidizer Vial      | Rust Vial            | BattleStart: 1/2/3 Rust (if enemy has ARM, +1)              | BattleStart: 2/3/4 Rust                                | **Complex** - requires conditional bonus Rust     |
| G-RU-02 | Rust Spike         | Rust Spike           | OnHit (1/turn): 1 Rust                                      | OnHit (1/turn): 1 Rust                                 | **FIXED**                                         |
| G-RU-03 | Corroded Greaves   | Corroded Greaves     | +1/2/3 SPD; Wounded: 2/3/4 Rust                             | +1/2/3 SPD; Wounded: 2/3/4 Rust                        | **FIXED**                                         |
| G-RU-04 | Acid Phial         | Acid Phial           | BattleStart: -2/3/4 enemy ARM                               | BattleStart: RemoveArmor 2/3/4                         | **FIXED**                                         |
| G-RU-05 | Flaking Plating    | Flaking Plating      | +6/8/10 ARM; Exposed: 2/3/4 Rust                            | +6/8/10 ARM; Exposed: 2/3/4 Rust                       | **FIXED**                                         |
| G-RU-06 | Rust Engine        | Rust Engine          | TurnStart: if enemy has Rust, 1/2/3 non-weapon              | TurnStart: 1/2/3 non-weapon (if enemy has Rust)        | **FIXED**                                         |
| G-RU-07 | Corrosion Loop     | Disintegration Field | OnHit (1/turn): if enemy has ARM, +1 Rust                   | TurnStart: 2/3/4 Rust                                  | **Complex** - requires conditional Rust           |
| G-RU-08 | Salvage Clamp      | Total Corrosion      | OnApplyRust (1/turn): +1 Gold                               | BattleStart: 5/8/11 Rust; TurnStart: RemoveArmor 1/2/3 | **Complex** - requires "on apply Rust" trigger    |

---

## BLOOD Tag (T-BO, G-BO)

| ID      | GDD Name          | Code Name         | GDD Effect                                         | Code Effect                                  | Status                                           |
| ------- | ----------------- | ----------------- | -------------------------------------------------- | -------------------------------------------- | ------------------------------------------------ |
| T-BO-01 | Serrated Drill    | Serrated Drill    | +1/2/3 ATK; OnHit (1/turn): 1 Bleed                | +1/2/3 ATK; OnHit (1/turn): 1 Bleed          | **FIXED**                                        |
| T-BO-02 | Reaper Pick       | Crimson Excavator | +2/3/4 ATK; OnHit: 1 Bleed (if Wounded, +1)        | +2/3/4 ATK; OnHit: 2/3/4 Bleed               | **Complex** - requires conditional bonus Bleed   |
| G-BO-01 | Last Breath Sigil | Blood Vial        | One use: prevent death, heal 2/3/4                 | BattleStart: 2/3/4 Bleed                     | **Complex** - requires death prevention mechanic |
| G-BO-02 | Bloodletting Fang | Leech Ring        | +1/2/3 damage vs Bleeding enemies                  | OnHit (1/turn): heal 1/2/3                   | **Complex** - requires conditional damage bonus  |
| G-BO-03 | Leech Wraps       | Hemorrhage Gloves | When enemy takes Bleed damage: heal 1/2/3 (1/turn) | OnHit (1/turn): 2/3/4 Bleed                  | **Complex** - requires "on Bleed damage" trigger |
| G-BO-04 | Blood Chalice     | Blood Chalice     | Victory: heal 3/5/7                                | Victory: heal 3/5/7                          | **FIXED**                                        |
| G-BO-05 | Hemorrhage Hook   | Hemorrhage Hook   | Wounded: 2/3/4 Bleed                               | Wounded: 2/3/4 Bleed                         | **FIXED**                                        |
| G-BO-06 | Execution Emblem  | Crimson Tide      | If enemy Wounded, first strike +2/3/4 damage       | BattleStart: 4/6/8 Bleed                     | **Complex** - requires conditional damage        |
| G-BO-07 | Gore Mantle       | Life Drain Aura   | First time Wounded: +4/6/8 ARM                     | TurnStart: heal 2/3/4, 1/2/3 Bleed           | **Complex** - requires one-time Wounded trigger  |
| G-BO-08 | Vampiric Tooth    | Exsanguinate      | First hit vs Bleeding: heal 2                      | BattleStart: 5/8/11 Bleed; OnHit: heal 2/3/4 | **Complex** - requires conditional healing       |

---

## TEMPO Tag (T-TE, G-TE)

| ID      | GDD Name             | Code Name            | GDD Effect                                            | Code Effect                                           | Status    |
| ------- | -------------------- | -------------------- | ----------------------------------------------------- | ----------------------------------------------------- | --------- |
| T-TE-01 | Quickpick            | Quickpick            | +1/2/3 ATK, +1/2/3 SPD                                | +1/2/3 ATK, +1/2/3 SPD                                | **FIXED** |
| T-TE-02 | Chrono Rapier        | Chrono Rapier        | +1/2/3 ATK, +2/3/4 SPD; FirstTurnIfFaster: +2/3/4 ATK | +1/2/3 ATK, +2/3/4 SPD; FirstTurnIfFaster: +2/3/4 ATK | **FIXED** |
| G-TE-01 | Wind-Up Spring       | Wind-Up Spring       | Turn 1: +1/2/3 SPD, +2/3/4 ATK                        | FirstTurn: +1/2/3 SPD, +2/3/4 ATK                     | **FIXED** |
| G-TE-02 | Ambush Charm         | Ambush Charm         | FirstTurnIfFaster: first strike +3/5/7 damage         | FirstTurnIfFaster: 3/5/7 damage (1/turn)              | **FIXED** |
| G-TE-03 | Counterweight Buckle | Counterweight Buckle | FirstTurnIfSlower: +5/7/9 ARM before damage           | FirstTurnIfSlower: +5/7/9 ARM                         | **FIXED** |
| G-TE-04 | Hourglass Charge     | Hourglass Charge     | Turn 5: +2/3/4 ATK, +1 SPD                            | TurnN(5): +2/3/4 ATK, +1/1/1 SPD                      | **FIXED** |
| G-TE-05 | Initiative Lens      | Initiative Lens      | +1/2/3 SPD; BattleStart: if SPD > enemy, +3/5/7 ARM   | +1/2/3 SPD; BattleStart: +3/5/7 ARM (if SPD > enemy)  | **FIXED** |
| G-TE-06 | Backstep Buckle      | Backstep Buckle      | FirstTurnIfSlower: first strike +3/5/7 damage         | FirstTurnIfSlower: 3/5/7 damage (1/turn)              | **FIXED** |
| G-TE-07 | Tempo Battery        | Tempo Battery        | EveryOtherTurn: +1/2/3 SPD                            | EveryOtherTurn: +1/2/3 SPD                            | **FIXED** |
| G-TE-08 | Second Wind Clock    | Second Wind Clock    | Turn 5: heal 4/6/8, +1 SPD                            | TurnN(5): heal 4/6/8, +1/1/1 SPD                      | **FIXED** |

---

## Summary Statistics

| Category     | Count | Percentage |
| ------------ | ----- | ---------- |
| **FIXED**    | 38    | 47.5%      |
| **Match**    | 4     | 5%         |
| **Mismatch** | 1     | 1.25%      |
| **Complex**  | 37    | 46.25%     |
| **TOTAL**    | 80    | 100%       |

---

## Progress Summary

### Items Fixed This Session (11 additional):

**SCOUT (2):**

1. G-SC-03 - Tunnel Instinct (BattleStart SPD with DigGreaterThanEnemyDig condition)
2. G-SC-05 - Wall-Sense Visor (BattleStart ARM with DigGreaterThanEnemyDig condition)

**FROST (6):** 3. T-FR-02 - Glacier Fang (OnHit Chill + conditional SPD if enemy has Chill) 4. G-FR-02 - Frostguard Buckler (ARM + conditional ARM if enemy has Chill) 5. G-FR-05 - Rime Cloak (ARM + OnStruck Chill trigger) 6. G-FR-06 - Permafrost Core (TurnStart ARM with EnemyHasStatus(Chill) condition) 7. G-FR-07 - Cold Front Idol (EveryOtherTurn Chill + conditional SPD) 8. G-FR-08 - Deep Freeze Charm (Wounded Chill + ReduceEnemySpd)

**RUST (1):** 9. G-RU-06 - Rust Engine (TurnStart non-weapon damage with EnemyHasStatus(Rust) condition)

**TEMPO (2):** 10. G-TE-04 - Hourglass Charge (TurnN{turn:5} trigger for ATK + SPD) 11. G-TE-05 - Initiative Lens (SPD + conditional ARM with SpdGreaterThanEnemySpd) 12. G-TE-08 - Second Wind Clock (TurnN{turn:5} trigger for Heal + SPD)

### All Items Fixed (38 total):

**STONE (5):**

1. T-ST-02 - Cragbreaker Hammer
2. G-ST-02 - Work Vest
3. G-ST-03 - Spiked Bracers
4. G-ST-04 - Reinforcement Plate
5. G-ST-05 - Rebar Carapace

**SCOUT (5):** 6. T-SC-01 - Twin Picks 7. T-SC-02 - Pneumatic Drill 8. G-SC-02 - Leather Gloves 9. G-SC-03 - Tunnel Instinct 10. G-SC-05 - Wall-Sense Visor

**GREED (2):** 11. T-GR-01 - Glittering Pick 12. G-GR-02 - Lucky Coin

**BLAST (1):** 13. T-BL-01 - Fuse Pick

**FROST (10):** 14. T-FR-01 - Rime Pike 15. T-FR-02 - Glacier Fang 16. G-FR-01 - Frost Lantern 17. G-FR-02 - Frostguard Buckler 18. G-FR-03 - Cold Snap Charm 19. G-FR-04 - Ice Skates 20. G-FR-05 - Rime Cloak 21. G-FR-06 - Permafrost Core 22. G-FR-07 - Cold Front Idol 23. G-FR-08 - Deep Freeze Charm

**RUST (6):** 24. T-RU-01 - Corrosive Pick 25. G-RU-02 - Rust Spike 26. G-RU-03 - Corroded Greaves 27. G-RU-04 - Acid Phial 28. G-RU-05 - Flaking Plating 29. G-RU-06 - Rust Engine

**BLOOD (3):** 30. T-BO-01 - Serrated Drill 31. G-BO-04 - Blood Chalice 32. G-BO-05 - Hemorrhage Hook

**TEMPO (6):** 33. T-TE-01 - Quickpick 34. T-TE-02 - Chrono Rapier 35. G-TE-01 - Wind-Up Spring 36. G-TE-02 - Ambush Charm 37. G-TE-03 - Counterweight Buckle 38. G-TE-04 - Hourglass Charge 39. G-TE-05 - Initiative Lens 40. G-TE-06 - Backstep Buckle 41. G-TE-07 - Tempo Battery 42. G-TE-08 - Second Wind Clock

### Mechanics Now Implemented:

- **Condition System**: `EnemyHasStatus(Chill/Rust/etc)`, `DigGreaterThanEnemyDig`, `SpdGreaterThanEnemySpd`, `OwnerWounded`, `OwnerExposed`
- **OnStruck Trigger**: Fires when combatant takes damage
- **TurnN { turn: u8 } Trigger**: Fires on a specific turn number (e.g., Turn 5)
- **ReduceEnemySpd Effect**: Reduces enemy's SPD stat

### Still Remaining (Complex Mechanics Required):

These 37 items need mechanics not yet implemented:

- **Counter/Storage**: Stored damage mechanics, countdown manipulation
- **Advanced Triggers**: OnGainShrapnel, OnBleedDamage, OnApplyRust
- **One-time Effects**: First time Wounded, death prevention, one-use items
- **Effect Modifiers**: Effect doubling, damage amplification, armor piercing
- **Day Phase**: Triggers that fire at start of each Day (not combat)
- **Gold Mechanics**: Gold consumption, Gold-to-Armor conversion

---

## Recommended Next Steps

### Phase 1: Implement OwnerHasArmor Condition

For items like G-ST-08 (Stone Sigil): "EndOfTurn: if has ARM, +1/2/3 ARM"

### Phase 2: Implement EveryOtherTurnFirstHit Trigger

For Shard items (G-GR-05 through G-GR-08): EveryOtherTurn effects that only trigger on first hit

### Phase 3: Implement EnemyWounded Condition

For items like G-BO-06 (Execution Emblem): "If enemy Wounded, first strike +2/3/4 damage"

### Phase 4: Advanced Mechanics

- Stored damage accumulation
- Countdown manipulation
- Effect doubling
- Death prevention
