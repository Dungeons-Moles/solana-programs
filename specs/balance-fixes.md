# Dungeons & Moles — Balance Patch v0.4

**Purpose:** This document contains all balance changes required to address issues identified in v0.3. Each section is formatted as an actionable prompt for AI coding agents.

**Priority Legend:**
- 🔴 CRITICAL — Blocks competitive integrity, fix immediately
- 🟠 HIGH — Significant advantage/disadvantage, fix before launch
- 🟡 MEDIUM — Quality of life / build diversity, fix in next pass

---

## Table of Contents

1. [T1 Enemy Stat Adjustments](#1-t1-enemy-stat-adjustments)
2. [Tool Rebalancing](#2-tool-rebalancing)
3. [Gear Rebalancing](#3-gear-rebalancing)
4. [Mythic Item Overhaul](#4-mythic-item-overhaul)
5. [Status Effect Rework (Chill)](#5-status-effect-rework-chill)
6. [Itemset Adjustments](#6-itemset-adjustments)
7. [POI Economy Fixes](#7-poi-economy-fixes)
8. [Map Generation Rules](#8-map-generation-rules)
9. [Combat System Tweaks](#9-combat-system-tweaks)
10. [New Items (Fill Gaps)](#10-new-items-fill-gaps)

---

## 1. T1 Enemy Stat Adjustments

**Priority:** 🔴 CRITICAL

**Problem:** At base player stats (10 HP, 1 ATK, 0 ARM, 0 SPD), T1 enemies range from trivial (3 damage taken) to mathematically impossible (player dies). This 10× variance bricks early runs based on RNG.

### Prompt for AI Agent:

```
Update the T1 enemy stats in the game data. For each enemy listed below, replace the existing T1 stat line with the new values.

Format: Enemy Name | HP/ATK/ARM/SPD/DIG | Trait changes (if any)

CHANGES:

1. Collapsed Miner
   OLD: 10/2/0/1/3 | Wounded: gain +3 ATK (this battle)
   NEW: 7/1/0/1/3 | Wounded: gain +2 ATK (this battle)
   REASON: 10 HP + 2 ATK base + 5 ATK wounded = unkillable at base stats. 7 HP lets player kill before Wounded triggers. +2 instead of +3 is still threatening but survivable.

2. Tunnel Warden
   OLD: 8/2/4/2/2 | First strike each turn: remove 3 Armor from you before damage
   NEW: 8/2/2/2/2 | First strike each turn: remove 2 Armor from you before damage
   REASON: 4 ARM vs 1 ATK = 4 dead turns. 2 ARM is still a wall but beatable in 4 turns total.

3. Shard Beetle
   OLD: 9/1/3/1/2 | Battle Start: gain 6 Shrapnel
   NEW: 8/1/2/1/2 | Battle Start: gain 3 Shrapnel
   REASON: 6 Shrapnel = 6 damage on player's first attack (60% base HP). 3 is punishing but survivable.

4. Burrow Ambusher
   OLD: 6/3/0/4/2 | Battle Start: deal 3 damage ignoring Armor
   NEW: 6/2/0/4/2 | Battle Start: deal 2 damage ignoring Armor
   REASON: 3 ambush + 3 ATK = instant kill potential on Turn 1-2. 2 ambush + 2 ATK gives player counterplay window.

5. Powder Tick
   OLD: 7/1/0/2/1 | Countdown(2): deal 6 damage to you and itself (non-weapon)
   NEW: 6/1/0/2/1 | Countdown(3): deal 5 damage to you and itself (non-weapon)
   REASON: Countdown(2) means it detonates Turn 2 before player can kill it. Countdown(3) gives player a race window.

6. Spore Slime
   OLD: 8/1/2/0/1 | Battle Start: apply 2 Chill to you
   NEW: 7/1/2/0/1 | Battle Start: apply 1 Chill to you
   REASON: 2 Chill on a 0-SPD enemy is overkill CC. 1 Chill is sufficient slowdown.

All other T1 enemies (Tunnel Rat, Cave Bat, Rust Mite Swarm, Frost Wisp, Coin Slug, Blood Mosquito) remain unchanged.
```

### T2 and T3 Cascade

```
After updating T1 stats, cascade the changes to T2 and T3 using this formula:

T2 = T1 base + (original T2 - original T1)
T3 = T1 base + (original T3 - original T1)

Example for Collapsed Miner:
- Original T1→T2→T3: 10/2/0/1/3 → 14/3/0/2/3 → 18/4/1/3/4
- Deltas: T2 = +4/+1/0/+1/0, T3 = +8/+2/+1/+2/+1
- New T1: 7/1/0/1/3
- New T2: 11/2/0/2/3
- New T3: 15/3/1/3/4

Apply this formula to all 6 changed enemies.
```

---

## 2. Tool Rebalancing

**Priority:** 🟠 HIGH

**Problem:** Rime Pike (Common) has +2 ATK while all other Commons have +1 ATK, plus it applies Chill. SCOUT multi-strike tools create multiplicative ATK scaling that breaks late-game.

### Prompt for AI Agent:

```
Update the following Tool items in the item database:

1. Rime Pike (T-FR-01)
   OLD: +2/3/4 ATK | On Hit (once/turn): apply 1 Chill
   NEW: +1/2/3 ATK | On Hit (once/turn): apply 1 Chill; if enemy has Chill, deal +1 bonus damage
   REASON: Align ATK with other Commons. The bonus damage vs Chilled compensates for lost base ATK and creates synergy incentive.

2. Glacier Fang (T-FR-02)
   OLD: +2/3/4 ATK | On Hit (once/turn): apply 1 Chill; if enemy has Chill, gain +1 SPD this turn
   NEW: +2/3/4 ATK | On Hit (once/turn): apply 1 Chill; if enemy has Chill, gain +1 SPD this turn and deal +1 bonus damage
   REASON: Rare should be better than Common. Adding bonus damage maintains the upgrade path from Rime Pike.

3. Pneumatic Drill (T-SC-02)
   OLD: +1/2/3 ATK | Strike 3 times per turn
   NEW: +1/2/3 ATK | Strike 3 times per turn; bonus ATK from Gear applies at 50% effectiveness (round down) to strikes beyond the 2nd
   REASON: Caps the multiplicative scaling. At +4 gear ATK, strikes 1-2 deal 5 each, strike 3 deals 3. Total = 13 DPT instead of 15. Still best DPT but not runaway.

4. Quickpick (T-TE-01)
   OLD: +1/2/3 ATK, +1/2/3 SPD
   NEW: +1/2/3 ATK, +2/3/4 SPD
   REASON: Quickpick was underpowered compared to other Commons. +2 SPD base makes it the definitive initiative tool.

5. Glittering Pick (T-GR-01)
   OLD: +1/2/3 ATK | On Hit (once/turn): gain 1 Gold
   NEW: +1/2/3 ATK | On Hit (once/turn): gain 1 Gold; Victory: gain +2 Gold
   REASON: GREED tool was weakest in combat. Victory bonus creates meaningful gold generation without combat power.

6. Fuse Pick (T-BL-01)
   OLD: +1/2/3 ATK | First hit each turn: deal 1 non-weapon damage
   NEW: +1/2/3 ATK | First hit each turn: deal 1/2/2 non-weapon damage
   REASON: Tier scaling was absent on the effect. Now Tier II+ have stronger chip damage.
```

---

## 3. Gear Rebalancing

**Priority:** 🟠 HIGH (dominated items, snowball loops, binary dependencies)

### Prompt for AI Agent — Dominated Items:

```
Update these Gear items to eliminate strict dominance relationships:

1. Lucky Coin (G-GR-02)
   OLD: Victory: gain 2/4/6 Gold
   NEW: Victory: gain 2/4/6 Gold AND heal 2/3/4 HP
   REASON: Loose Nuggets dominated Lucky Coin (9g/run vs 6g/run). Adding heal creates a sustain-economy hybrid niche.

2. Backstep Buckle (G-TE-06)
   OLD: If enemy acts first on Turn 1, your first strike deals +3/5/7 damage
   NEW: If enemy acts first on Turn 1, gain 4/6/8 Armor AND your first strike deals +3/5/7 damage
   REASON: Was dominated by Ambush Charm. Now it's the defensive-reactive counterpart (armor + damage when slower) vs Ambush Charm's offensive-proactive approach.

3. Ice Skates (G-FR-04)
   OLD: +1/2/3 SPD
   NEW: +1/2/3 SPD; reduce dig cost by 1 (minimum 2)
   REASON: Was dominated by Initiative Lens. Exploration utility differentiates it from combat-focused Initiative Lens.

4. Rust Spike (G-RU-02)
   OLD: On Hit (once/turn): apply 1 Rust
   NEW: On Hit (once/turn): apply 1 Rust; if enemy has Rust ≥ 3, deal 1/2/2 non-weapon damage
   REASON: Was dominated by Corrosion Loop. Payoff threshold makes it better in long fights while Loop is better for fast Rust stacking.

5. Salvage Clamp (G-RU-08)
   OLD: Whenever you apply Rust (once/turn): gain 1 Gold
   NEW: Whenever you apply Rust (once/turn): gain 1 Gold; if enemy has no Armor, apply 1 Rust anyway at battle start
   REASON: RUST was useless vs 0-ARM enemies. This gives RUST a foothold in those matchups.
```

### Prompt for AI Agent — Snowball/Loop Prevention:

```
Update these Gear items to prevent degenerate snowball loops:

1. Stone Sigil (G-ST-08)
   OLD: End of turn: if you have Armor, gain 1/2/3 Armor
   NEW: End of turn: if you have ≥3 Armor, gain 1/2/3 Armor
   REASON: The condition was trivially met. Now requires meaningful armor investment to trigger.

2. Rebar Carapace (G-ST-05)
   OLD: Exposed: gain 3/5/7 Armor
   NEW: Exposed (once per battle): gain 4/6/8 Armor
   REASON: Prevented repeated cycling. One safety net per fight, but it's now stronger for that one trigger.

3. Crystal Crown (G-ST-07)
   OLD: Battle Start: gain Max HP equal to your starting Armor (cap 12/18/24)
   NEW: Battle Start: gain Max HP equal to half your starting Armor, rounded up (cap 8/12/16)
   REASON: Full ARM→HP conversion at high caps made STONE effectively invincible. Half conversion with lower caps is still strong but not game-breaking.

4. Drill Servo (G-SC-06)
   OLD: Wounded: gain +1/2/3 additional strikes (this battle)
   NEW: Wounded: gain +1/1/2 additional strikes (this battle)
   REASON: +3 strikes at Tier III created 6+ strike builds. Capped at +2 keeps ceiling at 5 strikes.

5. Shrapnel Talisman (G-ST-06)
   OLD: Whenever you gain Shrapnel (once/turn): gain 1/2/3 Armor
   NEW: Whenever you gain Shrapnel (once per battle): gain 2/3/4 Armor
   REASON: Once/turn with multiple Shrapnel sources = infinite armor. Once/battle with higher value is a burst of defense, not a loop.
```

### Prompt for AI Agent — BLAST Binary Dependency Fix:

```
Rework these BLAST items to remove Blast Suit binary dependency:

1. Small Charge (G-BL-01)
   OLD: Countdown(2): deal 8/10/12 damage to enemy and you (non-weapon)
   NEW: Countdown(2): deal 10/12/14 damage to enemy and 4/5/6 damage to you (non-weapon)
   REASON: Asymmetric damage makes bombs usable (if painful) without Blast Suit. Still want the Suit, but not gated by it.

2. Blast Suit (G-BL-02)
   OLD: You ignore damage from your own BLAST items
   NEW: You ignore damage from your own BLAST items; Battle Start: gain 2/3/4 Armor
   REASON: Since self-damage is now halved baseline, Blast Suit needs added value to remain desirable. Armor synergizes with the "survive your own bombs" fantasy.

3. Time Charge (G-BL-07)
   OLD: Turn Start: gain +1/2/3 stored damage (this battle); when Exposed: deal stored damage
   NEW: Turn Start: gain +1/2/3 stored damage (this battle); when Exposed OR at end of battle: deal stored damage to enemy
   REASON: Exposed condition was too narrow (required losing all armor). End-of-battle trigger ensures value even if you never become Exposed.

4. Kindling Charge (G-BL-06)
   OLD: Battle Start: deal 1/2/3 damage; your next bomb this battle deals +3/5/7
   NEW: Battle Start: deal 2/3/4 damage to enemy; your next bomb this battle deals +3/5/7 and its self-damage is reduced by 2/3/4
   REASON: Adds self-damage mitigation to make BLAST more self-sufficient without Blast Suit.
```

---

## 4. Mythic Item Overhaul

**Priority:** 🟠 HIGH

**Problem:** Gear-Link Medallion is universally best-in-slot regardless of build. Vampiric Tooth is dramatically underpowered. Twin-Fuse Knot is appropriately tag-locked.

### Prompt for AI Agent:

```
Rework Mythic items for power parity and tag identity:

1. Gear-Link Medallion (G-SC-08)
   OLD: Your On Hit effects trigger twice (once/turn)
   NEW: Your On Hit effects from SCOUT items trigger twice (once/turn); +1/2/3 DIG
   REASON: Tag-lock to SCOUT prevents it from being auto-include in every on-hit build. DIG bonus reinforces SCOUT identity.

2. Vampiric Tooth (G-BO-08)
   OLD: Your first hit each turn vs a Bleeding enemy heals 2 HP
   NEW: Your first hit each turn vs a Bleeding enemy heals HP equal to their Bleed stacks (max 5); On Hit (once/turn): apply 1 Bleed
   REASON: Self-enabling (applies Bleed) and scales with Bleed investment. Now rewards building into BLOOD rather than being a weak splash pickup.

3. Twin-Fuse Knot (G-BL-08)
   UNCHANGED: Your bomb triggers happen twice
   REASON: Appropriately tag-locked and powerful. No changes needed.

4. NEW MYTHIC — Add to GREED tag:
   ID: G-GR-09
   Name: Crown of Avarice
   Type: Gear
   Tag: GREED
   Rarity: Mythic
   Effect: Your Gold gains are doubled; Battle Start: gain Armor equal to floor(Gold/8), cap 8/10/12
   REASON: GREED lacks a Mythic payoff. This creates a gold-scaling tank fantasy unique to GREED.

5. NEW MYTHIC — Add to FROST tag:
   ID: G-FR-09
   Name: Heart of the Glacier
   Type: Gear
   Tag: FROST
   Rarity: Mythic
   Effect: Enemies cannot remove Chill by natural decay (Chill only removed by effects that specifically clear it); Chilled enemies deal -1 damage (min 1)
   REASON: FROST lacks a Mythic. This turns Chill from tempo into permanent lockdown, creating the control fantasy FROST promises.

6. NEW MYTHIC — Add to TEMPO tag:
   ID: G-TE-09
   Name: Chrono Anchor
   Type: Gear
   Tag: TEMPO
   Rarity: Mythic
   Effect: You always act first regardless of SPD; Turn 1: gain +3/4/5 ATK (this battle)
   REASON: TEMPO lacks a Mythic. Guaranteed first strike + massive Turn 1 burst is the ultimate TEMPO payoff.

7. NEW MYTHIC — Add to STONE tag:
   ID: G-ST-09
   Name: Adamantine Core
   Type: Gear
   Tag: STONE
   Rarity: Mythic
   Effect: Your Armor cannot be reduced by more than 3 per turn from any source; Shrapnel you have persists between turns
   REASON: STONE lacks a Mythic. Armor damage cap + persistent Shrapnel is the ultimate wall fantasy.

8. NEW MYTHIC — Add to RUST tag:
   ID: G-RU-09
   Name: Corrosion Nexus
   Type: Gear
   Tag: RUST
   Rarity: Mythic
   Effect: Rust stacks on enemies also reduce their ATK by 1 per 2 stacks (max -3 ATK); applying Rust heals you for 1 HP (once/turn)
   REASON: RUST lacks a Mythic. Gives RUST value vs 0-ARM enemies and adds sustain to a tag that had none.
```

---

## 5. Status Effect Rework (Chill)

**Priority:** 🟡 MEDIUM

**Problem:** Chill reduces strikes by 1 (min 1), but almost all enemies have 1 strike. Chill is effectively just a Turn-1 initiative debuff.

### Prompt for AI Agent:

```
Update the Chill status effect definition in the combat system:

OLD CHILL BEHAVIOR:
- At Turn Start: reduce the holder's strikes this turn by 1 (min 1 strike)
- At end of turn: remove 1 Chill stack

NEW CHILL BEHAVIOR:
- At Turn Start: reduce the holder's strikes this turn by 1 (min 1 strike)
- Chilled combatants take +1 damage from all sources (per Chill stack, max +3 bonus damage)
- At end of turn: remove 1 Chill stack

REASON: The +damage component gives Chill value against 1-strike enemies. FROST builds can now meaningfully debuff enemies even when strike reduction is irrelevant.

IMPLEMENTATION NOTES:
- The +1 damage applies to each instance of damage (weapon strikes, non-weapon damage, Bleed ticks, Shrapnel retaliation, bomb damage)
- Cap the bonus at +3 to prevent Chill-stacking from becoming the dominant strategy
- Update all FROST item descriptions to mention "Chill: -1 strikes, +1 damage taken per stack"
```

---

## 6. Itemset Adjustments

**Priority:** 🟡 MEDIUM

### Prompt for AI Agent:

```
Update these Itemsets:

1. Swift Digger Kit
   OLD: G-SC-01 + G-SC-06 + T-SC-01 → Battle Start: if DIG > enemy DIG, gain +2 strikes (this battle)
   NEW: G-SC-01 + G-SC-06 + T-SC-01 → Battle Start: if DIG > enemy DIG, gain +1 strike (this battle) and +2 ATK (this battle)
   REASON: +2 strikes was too much ceiling. +1 strike + ATK gives similar power but without pushing strike count to 7+.

2. Rust Ritual
   OLD: T-RU-01 + G-RU-02 + G-RU-03 → On Hit (once/turn): apply +1 extra Rust
   NEW: T-RU-01 + G-RU-02 + G-RU-03 → On Hit (once/turn): apply +1 extra Rust; if enemy has 0 Armor, your Rust applications also deal 1 non-weapon damage
   REASON: Gives RUST a payoff vs unarmored enemies, solving the tag's structural weakness.

3. Shrapnel Harness
   OLD: G-ST-03 + G-ST-06 + T-ST-01 → Keep up to 3 Shrapnel at end of turn
   NEW: G-ST-03 + G-ST-06 + T-ST-01 → Keep up to 2 Shrapnel at end of turn; when struck, if you have Shrapnel, gain +1 Armor
   REASON: Reduced Shrapnel retention (synergy with Shrapnel Talisman nerf) but added defensive payoff.

4. Whiteout Initiative
   OLD: G-FR-04 + G-FR-03 + G-TE-05 → Battle Start: +1 SPD; if you act first Turn 1, apply +2 Chill
   NEW: G-FR-04 + G-FR-03 + G-TE-05 → Battle Start: +1 SPD; if you act first Turn 1, apply +2 Chill and enemy takes +2 damage on your first strike
   REASON: With Chill now providing +damage, this set doubles down on the Turn 1 spike fantasy.

5. Demolition Permit
   OLD: G-BL-01 + G-BL-02 + G-BL-03 → Countdown bombs tick 1 turn faster
   NEW: G-BL-01 + G-BL-02 + G-BL-03 → Countdown bombs tick 1 turn faster; your bomb self-damage is reduced by 2
   REASON: Adds self-damage mitigation to reduce Blast Suit dependency within the set.

6. Royal Extraction
   OLD: G-GR-01 + G-GR-04 + T-GR-02 → Gold→Armor becomes 1→4
   NEW: G-GR-01 + G-GR-04 + T-GR-02 → Gold→Armor becomes 1→3; you gain +1 Gold at the start of each battle
   REASON: 1→4 was too efficient. 1→3 with passive gold income is more balanced and creates interesting decisions.
```

---

## 7. POI Economy Fixes

**Priority:** 🟡 MEDIUM

**Problem:** Tool Oil Rack is free permanent stats. Expected gold sinks exceed gold income, but this is fine — the issue is GREED builds break the economy curve.

### Prompt for AI Agent:

```
Update POI interactions:

1. Tool Oil Rack (L4)
   OLD: Modify current tool: +1 ATK or +1 SPD or +1 DIG (once per tool), no cost
   NEW: Modify current tool: +1 ATK or +1 SPD or +1 DIG (once per tool), costs 6/8/10 Gold by act
   REASON: Free stats disproportionately benefit multi-strike builds. Gold cost creates meaningful decision.

2. Scrap Chute (L14)
   OLD: Destroy 1 Gear item (no reward). Costs 8/8/10/12 Gold by act.
   NEW: Destroy 1 Gear item, gain 3/4/5/6 Gold (scales by item rarity: Common/Rare/Heroic/Mythic). Costs 4/4/6/8 Gold by act.
   REASON: Current design punishes players for using it. Refund based on rarity makes it a real economy tool, not just emergency slot clearing.

3. Rusty Anvil (L10)
   OLD: Upgrade Tool tier (I→II costs 8 Gold; II→III costs 16 Gold)
   NEW: Upgrade Tool tier (I→II costs 10 Gold; II→III costs 18 Gold)
   REASON: Slight cost increase to compensate for GREED economy dominance.

4. Smuggler Hatch (L9)
   ADD NEW RULE: Maximum 2 rerolls per visit (then shop locks)
   REASON: Prevents gold-rich GREED builds from infinite rerolling to find perfect items.
```

---

## 8. Map Generation Rules

**Priority:** 🔴 CRITICAL

**Problem:** Random enemy placement can brick early runs with unkillable enemies.

### Prompt for AI Agent:

```
Add enemy placement rules to the map generator:

RULE 1 — SAFE START ZONE
The first 3 enemies the player can encounter (calculated by tile distance from spawn) must be drawn from the "Easy Pool":
- Easy Pool: Tunnel Rat, Cave Bat, Frost Wisp, Coin Slug, Blood Mosquito

RULE 2 — DIFFICULTY RAMP
Enemies are assigned to pools:
- Easy Pool: Tunnel Rat, Cave Bat, Frost Wisp, Coin Slug, Blood Mosquito (HP ≤ 7, simple traits)
- Medium Pool: Spore Slime, Rust Mite Swarm, Powder Tick, Shard Beetle (HP 6-9, debuff traits)
- Hard Pool: Collapsed Miner, Tunnel Warden, Burrow Ambusher (HP 6-10, dangerous traits)

Week 1: 60% Easy / 30% Medium / 10% Hard
Week 2: 40% Easy / 40% Medium / 20% Hard
Week 3: 30% Easy / 40% Medium / 30% Hard

RULE 3 — TIER DISTRIBUTION BY DISTANCE
Enemies closer to spawn skew toward T1. Enemies farther (>50% map distance from spawn) skew toward T2/T3.
- Near spawn (0-33% distance): 80% T1 / 15% T2 / 5% T3
- Mid map (34-66% distance): Use act defaults
- Far map (67-100% distance): 50% T1 / 35% T2 / 15% T3

RULE 4 — BOSS COUNTER GUARANTEE
At least 1 Counter Cache POI must be reachable within the first 30 moves of Day 1.
```

---

## 9. Combat System Tweaks

**Priority:** 🟡 MEDIUM

### Prompt for AI Agent:

```
Update combat system rules:

1. STRIKE CAP
   Add rule: Maximum strikes per turn is 5 (regardless of item combinations)
   REASON: Prevents degenerate 7-8 strike builds.

2. SUDDEN DEATH RAMP
   OLD: Turn 25+: both combatants gain +1 ATK per turn
   NEW: Turn 20+: both combatants gain +1 ATK per turn; Turn 30+: both combatants gain +2 ATK per turn (stacking)
   REASON: Faster sudden death punishes pure stall builds (STONE) while still allowing tactical attrition.

3. ARMOR EFFECTIVENESS CAP
   Add rule: Armor reduces incoming weapon damage, but cannot reduce a strike below 1 damage
   REASON: Prevents complete damage immunity through high armor stacking. Every hit does something.

4. STATUS DURATION DISPLAY
   Add combat log entries showing:
   - Current Chill stacks on each combatant
   - Current Rust stacks on each combatant
   - Current Bleed stacks on each combatant
   - Countdown timers for bomb items
   REASON: Player needs visibility into status state for strategic decisions.

5. ON-HIT ONCE-PER-TURN CLARIFICATION
   Clarify rule: "Once per turn" on-hit effects trigger on the FIRST eligible hit that turn, not the first hit overall
   Example: If player has Bleed on-hit and Rust on-hit, both trigger on the first strike (not one on first strike, one on second)
   REASON: Reduces confusion about effect ordering.
```

---

## 10. New Items (Fill Gaps)

**Priority:** 🟡 MEDIUM

**Problem:** Some tags lack Heroic/Mythic options. RUST and FROST have structural gaps.

### Prompt for AI Agent:

```
Add these new items to the item database:

1. G-RU-09 — Oxidized Blade
   Type: Gear
   Tag: RUST
   Rarity: Heroic
   Effect: +2/3/4 ATK; your attacks deal +1 damage per Rust stack on enemy (max +4)
   REASON: RUST payoff item that converts Rust stacks into damage, solving the "Rust is useless vs 0-ARM" problem.

2. G-FR-09 — Frostbite Gauntlets
   Type: Gear
   Tag: FROST
   Rarity: Heroic
   Effect: On Hit (once/turn): if enemy is Chilled, deal 2/3/4 non-weapon damage; if enemy has ≥3 Chill, apply 1 additional Chill
   REASON: FROST damage payoff that rewards Chill stacking with actual kill pressure.

3. G-TE-05b — Patience Stone
   Type: Gear
   Tag: TEMPO
   Rarity: Rare
   Effect: Turn 3+: gain +1/2/2 ATK (this battle); Turn 5+: gain +1/2/3 ARM (this battle)
   REASON: TEMPO currently only rewards Turn 1 burst. This creates a "slow TEMPO" playstyle that rewards surviving to mid-fight.

4. G-BL-09 — Shaped Charge
   Type: Gear
   Tag: BLAST
   Rarity: Heroic
   Effect: Countdown(3): deal 12/15/18 damage to enemy only (non-weapon); this bomb does not damage you
   REASON: Safe bomb option for BLAST builds without Blast Suit. Longer countdown + no self-damage is the tradeoff.

5. G-GR-10 — Prospector's Instinct
   Type: Gear
   Tag: GREED
   Rarity: Rare
   Effect: The first time you open a Supply Cache or Tool Crate each week, see +1 additional option
   REASON: GREED utility outside of combat. More item choices = better build shaping.

6. T-ST-03 — Earthshaker Maul
   Type: Tool
   Tag: STONE
   Rarity: Heroic
   Effect: +2/3/4 ATK, +4/6/8 ARM; On Hit (once/turn): if you have ≥5 Armor, deal 2 non-weapon damage
   REASON: STONE lacks a Heroic tool. This rewards armor investment with damage output.

7. T-RU-03 — Entropy Spike
   Type: Tool
   Tag: RUST
   Rarity: Heroic
   Effect: +2/3/4 ATK, +1/2/3 SPD; On Hit (once/turn): apply 1 Rust; if enemy has ≥4 Rust, your strikes ignore all Armor
   REASON: RUST lacks a Heroic tool. Full armor penetration at high Rust stacks is the ultimate anti-tank payoff.

8. T-FR-03 — Blizzard Fang
   Type: Tool
   Tag: FROST
   Rarity: Heroic
   Effect: +2/3/4 ATK; On Hit (once/turn): apply 2 Chill; enemies with ≥3 Chill cannot gain Armor
   REASON: FROST lacks a Heroic tool. Armor denial is a unique defensive counter that fits FROST's control identity.
```

---

## Summary Checklist

Use this checklist to track implementation:

### 🔴 CRITICAL
- [x] T1 Enemy stat adjustments (6 enemies)
- [x] Map generation safe start zone
- [x] Strike cap (max 5)

### 🟠 HIGH
- [x] Tool rebalancing (6 tools)
- [x] Dominated item fixes (5 items)
- [x] Snowball loop prevention (5 items)
- [x] BLAST binary dependency fix (4 items)
- [x] Mythic overhaul (2 reworks + 6 new)

### 🟡 MEDIUM
- [x] Chill status effect rework
- [x] Itemset adjustments (6 sets)
- [x] POI economy fixes (4 POIs)
- [x] Combat system tweaks (5 rules)
- [x] New items (8 items)

---

## Appendix: Updated Bitmask

After adding new items, update the item bitmask documentation:

```
NEW ITEMS ADDED:
- G-GR-09 (Crown of Avarice) — Index 23
- G-FR-09 (Heart of the Glacier) — Index 39
- G-TE-09 (Chrono Anchor) — Index 63
- G-ST-09 (Adamantine Core) — Index 7 (shift existing)
- G-RU-09 (Corrosion Nexus) — Index 47
- G-RU-10 (Oxidized Blade) — requires expanding bitmask
- G-FR-10 (Frostbite Gauntlets) — requires expanding bitmask
- G-TE-10 (Patience Stone) — requires expanding bitmask
- G-BL-09 (Shaped Charge) — Index 31
- G-GR-10 (Prospector's Instinct) — requires expanding bitmask

TOOLS ADDED:
- T-ST-03 (Earthshaker Maul) — Index 65 (new slot)
- T-RU-03 (Entropy Spike) — Index 75 (new slot)
- T-FR-03 (Blizzard Fang) — Index 73 (new slot)

Expand bitmask from 80 bits (10 bytes) to 96 bits (12 bytes) to accommodate new items.
```

---

*Document generated by balance analysis system. Validate changes through playtesting before final implementation.*