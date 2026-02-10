# Dungeons & Moles — Balance Patch v0.4 (Revised)

**Purpose:** All balance changes to the existing 80-item pool. NO new items, enemies, POIs, or bosses — modifications only.

**Item Count:** 80 items (16 tools + 64 gear) — UNCHANGED

**Priority Legend:**
- 🔴 CRITICAL — Blocks competitive integrity, fix immediately
- 🟠 HIGH — Significant advantage/disadvantage, fix before launch
- 🟡 MEDIUM — Quality of life / build diversity, fix in next pass

---

## Table of Contents

1. [T1 Enemy Stat Adjustments](#1-t1-enemy-stat-adjustments)
2. [Tool Rebalancing](#2-tool-rebalancing)
3. [Gear Rebalancing](#3-gear-rebalancing)
4. [Mythic Item Rebalancing](#4-mythic-item-rebalancing)
5. [Rarity Promotions (Fill Mythic Gaps)](#5-rarity-promotions)
6. [Status Effect Rework (Chill)](#6-status-effect-rework-chill)
7. [Itemset Adjustments](#7-itemset-adjustments)
8. [POI Economy Fixes](#8-poi-economy-fixes)
9. [Map Generation Rules](#9-map-generation-rules)
10. [Combat System Tweaks](#10-combat-system-tweaks)

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

**Priority:** 🟠 HIGH

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
   NEW: Whenever you apply Rust (once/turn): gain 1 Gold; Battle Start: if enemy has 0 Armor, apply 1 Rust
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
   NEW: Turn Start: gain +1/2/3 stored damage (this battle); when Exposed OR Turn 6+: deal stored damage to enemy
   REASON: Exposed condition was too narrow (required losing all armor). Turn 6+ trigger ensures value even if you never become Exposed.

4. Kindling Charge (G-BL-06)
   OLD: Battle Start: deal 1/2/3 damage; your next bomb this battle deals +3/5/7
   NEW: Battle Start: deal 2/3/4 damage to enemy; your next bomb this battle deals +3/5/7 and its self-damage is reduced by 2/3/4
   REASON: Adds self-damage mitigation to make BLAST more self-sufficient without Blast Suit.
```

### Prompt for AI Agent — RUST/FROST Gap Fixes:

```
Update these items to solve structural tag weaknesses:

1. Rust Engine (G-RU-06)
   OLD: Turn Start: if enemy has Rust, deal 1/2/3 non-weapon damage
   NEW: Turn Start: if enemy has Rust OR 0 Armor, deal 1/2/3 non-weapon damage
   REASON: Gives RUST payoff even vs 0-ARM enemies. Rust Engine now works as general chip damage when Rust can't stick.

2. Corrosion Loop (G-RU-07)
   OLD: On Hit (once/turn): if enemy has Armor, apply +1 additional Rust
   NEW: On Hit (once/turn): apply +1 additional Rust; if enemy has 0 Armor, deal 1 non-weapon damage instead
   REASON: Loop is useless vs 0-ARM. Now converts to chip damage when Rust isn't relevant.

3. Permafrost Core (G-FR-06)
   OLD: Turn Start: if enemy has Chill, gain 1/2/3 Armor
   NEW: Turn Start: if enemy has Chill, gain 1/2/3 Armor and deal 1 non-weapon damage
   REASON: FROST lacked damage payoff. Now Chill maintenance generates both defense AND offense.

4. Cold Front Idol (G-FR-07)
   OLD: Every other turn: apply 1 Chill; if enemy already has Chill, gain +1 SPD this turn
   NEW: Every other turn: apply 1 Chill and deal 1 non-weapon damage; if enemy already has Chill, gain +1 SPD this turn
   REASON: FROST needed damage. Cold Front now provides consistent chip while maintaining Chill.

5. Deep Freeze Charm (G-FR-08)
   OLD: Wounded: apply 2/3/4 Chill and reduce enemy SPD by 1 (this battle)
   NEW: Wounded: apply 2/3/4 Chill, reduce enemy SPD by 1 (this battle), and enemy takes +1 damage from all sources while Chilled (this battle)
   REASON: Deep Freeze is Heroic but effect was weak. The damage amp makes it a true Heroic payoff.
```

---

## 4. Mythic Item Rebalancing

**Priority:** 🟠 HIGH

**Problem:** Gear-Link Medallion is universally best-in-slot regardless of build. Vampiric Tooth is dramatically underpowered. Twin-Fuse Knot is appropriately tag-locked.

### Prompt for AI Agent:

```
Rebalance the existing 3 Mythic items:

1. Gear-Link Medallion (G-SC-08)
   OLD: Your On Hit effects trigger twice (once/turn)
   NEW: Your On Hit effects trigger twice (once/turn); this only applies to effects from SCOUT-tagged items OR your equipped Tool
   REASON: Partial tag-lock. Still works with any Tool's on-hit, but BLOOD/RUST/FROST gear on-hits no longer double. Rewards SCOUT investment.

2. Vampiric Tooth (G-BO-08)
   OLD: Your first hit each turn vs a Bleeding enemy heals 2 HP
   NEW: Your first hit each turn applies 1 Bleed; if enemy is already Bleeding, heal HP equal to their Bleed stacks instead (max 5 HP)
   REASON: Now self-enabling (applies Bleed) and scales with Bleed investment. Rewards building into BLOOD rather than being a weak splash pickup.

3. Twin-Fuse Knot (G-BL-08)
   UNCHANGED: Your bomb triggers happen twice
   REASON: Appropriately tag-locked and powerful. No changes needed.
```

---

## 5. Rarity Promotions

**Priority:** 🟡 MEDIUM

**Problem:** Only 3 tags have Mythics (SCOUT, BLOOD, BLAST). Other tags lack a capstone item. Rather than adding items, promote existing Heroics to Mythic with enhanced effects.

### Prompt for AI Agent:

```
Promote these Heroic items to Mythic rarity with upgraded effects:

1. Crystal Crown (G-ST-07) — STONE
   OLD RARITY: Heroic
   NEW RARITY: Mythic
   OLD EFFECT: Battle Start: gain Max HP equal to half your starting Armor, rounded up (cap 8/12/16)
   NEW EFFECT: Battle Start: gain Max HP equal to your starting Armor (cap 10/15/20); your Armor cannot be reduced below 1 by any single source
   REASON: STONE needed a Mythic. Crystal Crown was already the build-defining item; Mythic version adds armor floor protection.

2. Royal Bracer (G-GR-04) — GREED
   OLD RARITY: Heroic
   NEW RARITY: Mythic
   OLD EFFECT: Turn Start: convert 1 Gold → 2/3/4 Armor
   NEW EFFECT: Turn Start: convert 1 Gold → 3/4/5 Armor; your Gold gains from all sources are increased by 50% (round down)
   REASON: GREED needed a Mythic. Royal Bracer becomes the ultimate gold-to-power converter with income amplification.

3. Chrono Rapier (T-TE-02) — TEMPO
   OLD RARITY: Heroic
   NEW RARITY: Mythic
   OLD EFFECT: +1/2/3 ATK, +2/3/4 SPD; if you act first on Turn 1, gain +2/3/4 ATK (this battle)
   NEW EFFECT: +2/3/4 ATK, +3/4/5 SPD; you always act first on Turn 1 regardless of enemy SPD; if you act first, gain +3/4/5 ATK (this battle)
   REASON: TEMPO needed a Mythic Tool. Chrono Rapier becomes the ultimate Turn 1 spike with guaranteed initiative.

4. Permafrost Core (G-FR-06) — FROST
   OLD RARITY: Heroic
   NEW RARITY: Mythic
   OLD EFFECT: Turn Start: if enemy has Chill, gain 1/2/3 Armor and deal 1 non-weapon damage
   NEW EFFECT: Turn Start: if enemy has Chill, gain 2/3/4 Armor and deal 2 non-weapon damage; Chill on enemies decays 1 stack slower (minimum decay: 0)
   REASON: FROST needed a Mythic. Permafrost Core becomes the Chill-lock engine with slowed decay.

5. Corrosion Loop (G-RU-07) — RUST
   OLD RARITY: Heroic
   NEW RARITY: Mythic
   OLD EFFECT: On Hit (once/turn): apply +1 additional Rust; if enemy has 0 Armor, deal 1 non-weapon damage instead
   NEW EFFECT: On Hit (once/turn): apply +2 additional Rust; Rust stacks on enemies also reduce their ATK by 1 per 3 stacks (max -2 ATK); if enemy has 0 Armor, deal 2 non-weapon damage instead
   REASON: RUST needed a Mythic. Corrosion Loop becomes the Rust payoff engine with ATK reduction.

AFTER PROMOTIONS, update the rarity distribution:
- Mythics: 8 total (1 per tag)
  - STONE: Crystal Crown
  - SCOUT: Gear-Link Medallion
  - GREED: Royal Bracer
  - BLAST: Twin-Fuse Knot
  - FROST: Permafrost Core
  - RUST: Corrosion Loop
  - BLOOD: Vampiric Tooth
  - TEMPO: Chrono Rapier

- Heroics reduced by 5 (promoted items)
- Fill Heroic gaps by promoting these Rares to Heroic:

6. Frostguard Buckler (G-FR-02) — FROST
   OLD RARITY: Rare
   NEW RARITY: Heroic
   OLD EFFECT: +6/8/10 ARM; Battle Start: if enemy has Chill, gain +2/3/4 Armor
   NEW EFFECT: +8/10/12 ARM; Battle Start: if enemy has Chill, gain +3/4/5 Armor and apply 1 Chill
   REASON: Fills FROST Heroic gap left by Permafrost Core promotion.

7. Gilded Band (G-GR-03) — GREED
   OLD RARITY: Rare
   NEW RARITY: Heroic
   OLD EFFECT: Battle Start: gain Armor equal to floor(Gold/10) (cap 2/3/4)
   NEW EFFECT: Battle Start: gain Armor equal to floor(Gold/8) (cap 4/5/6); if Gold ≥ 30, also gain +1 SPD this battle
   REASON: Fills GREED Heroic gap left by Royal Bracer promotion.

8. Etched Burrowblade (T-RU-02) — RUST
   OLD RARITY: Rare
   NEW RARITY: Heroic
   OLD EFFECT: +2/3/4 ATK, +1/2/3 SPD; if enemy has Rust, your strikes ignore 1/2/3 Armor
   NEW EFFECT: +2/3/4 ATK, +2/3/4 SPD; if enemy has Rust, your strikes ignore 2/3/4 Armor; if enemy has ≥ 4 Rust, ignore ALL Armor
   REASON: Fills RUST Heroic gap left by Corrosion Loop promotion.
```

---

## 6. Status Effect Rework (Chill)

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
- Chilled combatants take +1 damage from all sources (stacks with multiple Chill, max +3 bonus damage)
- At end of turn: remove 1 Chill stack

REASON: The +damage component gives Chill value against 1-strike enemies. FROST builds can now meaningfully debuff enemies even when strike reduction is irrelevant.

IMPLEMENTATION NOTES:
- The +1 damage per Chill stack applies to each instance of damage (weapon strikes, non-weapon damage, Bleed ticks, Shrapnel retaliation, bomb damage)
- Cap the bonus at +3 to prevent Chill-stacking from becoming the dominant strategy
- Update all FROST item tooltips to reflect: "Chill: -1 strikes (min 1), +1 damage taken per stack (max +3)"
```

---

## 7. Itemset Adjustments

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
   NEW: T-RU-01 + G-RU-02 + G-RU-03 → On Hit (once/turn): apply +1 extra Rust; if enemy has 0 Armor, deal 1 non-weapon damage per Rust stack (max 3)
   REASON: Gives RUST a payoff vs unarmored enemies, solving the tag's structural weakness.

3. Shrapnel Harness
   OLD: G-ST-03 + G-ST-06 + T-ST-01 → Keep up to 3 Shrapnel at end of turn
   NEW: G-ST-03 + G-ST-06 + T-ST-01 → Keep up to 2 Shrapnel at end of turn; when struck while you have Shrapnel, gain +1 Armor
   REASON: Reduced Shrapnel retention (synergy with Shrapnel Talisman nerf) but added defensive payoff.

4. Whiteout Initiative
   OLD: G-FR-04 + G-FR-03 + G-TE-05 → Battle Start: +1 SPD; if you act first Turn 1, apply +2 Chill
   NEW: G-FR-04 + G-FR-03 + G-TE-05 → Battle Start: +1 SPD; if you act first Turn 1, apply +2 Chill and your first strike deals +3 damage
   REASON: With Chill now providing +damage taken, this set doubles down on the Turn 1 spike fantasy.

5. Demolition Permit
   OLD: G-BL-01 + G-BL-02 + G-BL-03 → Countdown bombs tick 1 turn faster
   NEW: G-BL-01 + G-BL-02 + G-BL-03 → Countdown bombs tick 1 turn faster; your bomb self-damage is reduced by 2
   REASON: Adds self-damage mitigation to reduce Blast Suit dependency within the set.

6. Royal Extraction
   OLD: G-GR-01 + G-GR-04 + T-GR-02 → Gold→Armor becomes 1→4
   NEW: G-GR-01 + G-GR-04 + T-GR-02 → Gold→Armor becomes 1→4; gain +1 Gold at the start of each battle
   REASON: Royal Bracer is now Mythic, so the set is harder to complete. Added gold income compensates.

Note: Royal Extraction now requires a Mythic (Royal Bracer). Consider if this set should be adjusted to use different items, or if Mythic-requiring sets are acceptable for late-game power.
```

---

## 8. POI Economy Fixes

**Priority:** 🟡 MEDIUM

### Prompt for AI Agent:

```
Update POI interactions:

1. Tool Oil Rack (L4)
   UNCHANGED: Modify current tool: +1 ATK or +1 SPD or +1 DIG (once per tool), no cost
   REASON: Free permanent upgrade is an intentional reward for exploration. The "once per tool" limit already prevents abuse.

2. Scrap Chute (L14)
   OLD: Destroy 1 Gear item (no reward). Costs 8/8/10/12 Gold by act.
   NEW: Destroy 1 Gear item, receive partial refund based on rarity: Common 2g, Rare 4g, Heroic 6g, Mythic 10g. Costs 4 Gold flat.
   REASON: Current design punishes players for using it. Refund makes it a real economy tool, not just emergency slot clearing.

3. Smuggler Hatch (L9)
   ADD NEW RULE: Maximum 3 rerolls per visit (then shop locks until next visit)
   REASON: Prevents gold-rich GREED builds from infinite rerolling to find perfect items.

4. Rusty Anvil (L10)
   OLD: Upgrade Tool tier (I→II costs 8 Gold; II→III costs 16 Gold)
   NEW: Upgrade Tool tier (I→II costs 10 Gold; II→III costs 20 Gold)
   REASON: Slight cost increase to compensate for Tool power increases and GREED economy strength.
```

---

## 9. Map Generation Rules

**Priority:** 🔴 CRITICAL

### Prompt for AI Agent:

```
Add enemy placement rules to the map generator:

RULE 1 — SAFE START ZONE
The first 3 enemies the player can encounter (calculated by tile distance from spawn) must be drawn from the "Easy Pool":
- Easy Pool: Tunnel Rat, Cave Bat, Frost Wisp, Coin Slug, Blood Mosquito

RULE 2 — DIFFICULTY RAMP
Enemies are assigned to pools:
- Easy Pool: Tunnel Rat, Cave Bat, Frost Wisp, Coin Slug, Blood Mosquito (5 enemies)
- Medium Pool: Spore Slime, Rust Mite Swarm, Powder Tick, Shard Beetle (4 enemies)
- Hard Pool: Collapsed Miner, Tunnel Warden, Burrow Ambusher (3 enemies)

Distribution by week:
- Week 1: 60% Easy / 30% Medium / 10% Hard
- Week 2: 40% Easy / 40% Medium / 20% Hard
- Week 3: 30% Easy / 40% Medium / 30% Hard

RULE 3 — TIER DISTRIBUTION BY DISTANCE
Enemies closer to spawn skew toward T1. Enemies farther skew toward T2/T3.
- Near spawn (0-33% map distance): 80% T1 / 15% T2 / 5% T3
- Mid map (34-66% distance): Use act tier defaults from GDD
- Far map (67-100% distance): 50% T1 / 35% T2 / 15% T3

RULE 4 — COUNTER CACHE GUARANTEE
At least 1 Counter Cache (L13) POI must be reachable within the first 30 moves of Day 1.
```

---

## 10. Combat System Tweaks

**Priority:** 🟡 MEDIUM

### Prompt for AI Agent:

```
Update combat system rules:

1. STRIKE CAP
   ADD RULE: Maximum strikes per turn is 5 (regardless of item/set combinations)
   REASON: Prevents degenerate 6+ strike builds even with all bonuses stacked.

2. SUDDEN DEATH RAMP
   OLD: Turn 25+: both combatants gain +1 ATK per turn (stacking)
   NEW: Turn 20+: both combatants gain +1 ATK per turn; Turn 30+: gain +2 ATK per turn instead (stacking)
   REASON: Faster sudden death punishes pure stall builds (STONE) while still allowing tactical attrition.

3. ARMOR MINIMUM DAMAGE
   ADD RULE: Each weapon strike deals minimum 1 damage, even if target Armor exceeds attack damage
   REASON: Prevents complete damage immunity through high armor stacking. Every hit does something.

4. ON-HIT SIMULTANEOUS TRIGGER
   CLARIFY RULE: All "once per turn" on-hit effects trigger simultaneously on the first eligible hit, not sequentially across multiple hits
   EXAMPLE: If player has Bleed on-hit and Rust on-hit, both trigger on the first strike (not Bleed on strike 1, Rust on strike 2)
   REASON: Reduces confusion about effect ordering and prevents multi-strike from spreading on-hits across more targets.

5. CHILL DAMAGE DISPLAY
   ADD TO COMBAT LOG: When damage is amplified by Chill, display as "X damage (+Y from Chill)"
   REASON: Player needs visibility into Chill's new damage amplification effect.
```

---

## Summary Checklist

**Final Item Count: 80 (unchanged)**
- 16 Tools (unchanged count)
- 64 Gear (unchanged count)
- 8 Mythics (was 3, now 8 via promotions)
- Heroics adjusted via promotion chain

### 🔴 CRITICAL
- [ ] T1 Enemy stat adjustments (6 enemies modified)
- [ ] Map generation safe start zone
- [ ] Strike cap (max 5)

### 🟠 HIGH
- [ ] Tool rebalancing (6 tools modified)
- [ ] Dominated item fixes (5 gear modified)
- [ ] Snowball loop prevention (5 gear modified)
- [ ] BLAST binary dependency fix (4 gear modified)
- [ ] RUST/FROST structural fixes (6 gear modified)
- [ ] Mythic rebalancing (3 existing Mythics)
- [ ] Rarity promotions (5 Heroic→Mythic, 3 Rare→Heroic)

### 🟡 MEDIUM
- [ ] Chill status effect rework
- [ ] Itemset adjustments (6 sets modified)
- [ ] POI economy fixes (3 POIs modified)
- [ ] Combat system tweaks (5 rules added/modified)

---

## Appendix: Items Modified Summary

**Total items with changes: 33 out of 80 (41%)**

### Tools Modified (6/16):
- T-FR-01 Rime Pike
- T-FR-02 Glacier Fang
- T-SC-02 Pneumatic Drill
- T-TE-01 Quickpick
- T-TE-02 Chrono Rapier (also promoted to Mythic)
- T-GR-01 Glittering Pick
- T-BL-01 Fuse Pick
- T-RU-02 Etched Burrowblade (promoted to Heroic)

### Gear Modified (25/64):
**STONE:** G-ST-05, G-ST-06, G-ST-07 (promoted), G-ST-08
**SCOUT:** G-SC-06, G-SC-08
**GREED:** G-GR-02, G-GR-03 (promoted), G-GR-04 (promoted)
**BLAST:** G-BL-01, G-BL-02, G-BL-06, G-BL-07
**FROST:** G-FR-02 (promoted), G-FR-04, G-FR-06 (promoted), G-FR-07, G-FR-08
**RUST:** G-RU-02, G-RU-06, G-RU-07 (promoted), G-RU-08
**BLOOD:** G-BO-08
**TEMPO:** G-TE-06

---

*Document generated by balance analysis system. Item count verified: 80 total. No new items created.*