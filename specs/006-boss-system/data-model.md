# Data Model: Boss System

**Feature**: 006-boss-system
**Date**: 2026-01-19

## Entity Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         STATIC DATA (Compiled)                               │
├─────────────────────────────────────────────────────────────────────────────┤
│  BossDefinition (24)                                                         │
│  ├── id: [u8; 12]          "B-A-W1-01\0\0\0"                                │
│  ├── name: &'static str    "The Broodmother"                                │
│  ├── emoji: [u8; 4]        🕷️                                               │
│  ├── biome: Biome          A or B                                           │
│  ├── week: Week            One, Two, or Three                               │
│  ├── base_stats: BossStats HP/ATK/ARM/SPD/DIG                               │
│  ├── weakness_tags: [ItemTag; 2]                                            │
│  └── traits: &'static [BossTrait]                                           │
│                                                                              │
│  BossTrait                                                                   │
│  ├── trigger: TriggerType                                                   │
│  ├── effect_type: EffectType                                                │
│  ├── value: i16                                                             │
│  ├── once_per_turn: bool                                                    │
│  └── special: Option<SpecialMechanic>                                       │
│         ├── Countdown { turns: u8 }                                         │
│         ├── Reflection { stacks: u8 }                                       │
│         ├── Phase { threshold: u8, effects: &[BossTrait] }                  │
│         ├── Conditional { condition: Condition }                            │
│         └── GoldInteraction { steal_amount: u16, armor_ratio: u8 }          │
└─────────────────────────────────────────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       RUNTIME DATA (Computed)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│  BossInstance (computed per combat)                                          │
│  ├── definition: &'static BossDefinition                                    │
│  ├── scaled_stats: BossStats     (after act/tier modifiers)                 │
│  └── combat_state: BossCombatState                                          │
│         ├── phase_state: PhaseState                                         │
│         ├── reflection_state: Option<ReflectionState>                       │
│         └── countdown_state: Vec<CountdownState>                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Entities

### 1. BossDefinition (Compile-time Constant)

Static data for a boss. Stored as compile-time constants in the program binary.

| Field | Type | Description |
|-------|------|-------------|
| id | [u8; 12] | Unique ID (e.g., "B-A-W1-01\0\0\0") |
| name | &'static str | Display name (e.g., "The Broodmother") |
| emoji | [u8; 4] | UTF-8 encoded emoji |
| biome | Biome | A or B |
| week | Week | One, Two, or Three |
| boss_index | u8 | Index within week (0-4 for W1/W2, 0-1 for W3) |
| base_stats | BossStats | Base HP/ATK/ARM/SPD/DIG before scaling |
| weakness_tags | [ItemTag; 2] | Two weakness tags for item weighting |
| traits | &'static [BossTrait] | 2-3 trait definitions |

### 2. BossStats (Struct)

Stats for a boss, used both in definitions (base) and instances (scaled).

| Field | Type | Description |
|-------|------|-------------|
| hp | u16 | Hit points |
| atk | u8 | Attack power |
| arm | u8 | Armor |
| spd | u8 | Speed |
| dig | u8 | Dig stat |

**Size**: 6 bytes

### 3. BossTrait (Compile-time Constant)

A boss ability that triggers during combat.

| Field | Type | Description |
|-------|------|-------------|
| trigger | TriggerType | When effect activates |
| effect_type | EffectType | What the effect does |
| value | i16 | Effect magnitude |
| once_per_turn | bool | Limit to once per turn |
| special | Option<SpecialMechanic> | Extended mechanics |

### 4. SpecialMechanic (Enum)

Extended mechanics for complex boss traits.

| Variant | Fields | Description |
|---------|--------|-------------|
| Countdown | turns: u8 | Effect triggers after N turns |
| Reflection | stacks: u8 | Reflects N status applications back to player |
| Phase | threshold: u8, effects: &'static [BossTrait] | Activates at HP% threshold |
| Conditional | condition: Condition | Only triggers if condition met |
| GoldInteraction | steal: u16, armor_ratio: u8 | Steals gold and/or converts to armor |
| ModifyOnWounded | effect_type: EffectType, value: i16 | Buff when HP < 50% |

### 5. Condition (Enum)

Conditions for conditional triggers.

| Variant | Description |
|---------|-------------|
| PlayerDigLessThan | Player DIG < boss DIG |
| PlayerExposed | Player ARM = 0 |
| ActsFirstTurn1 | Boss acts before player on Turn 1 |
| ReflectionDepleted | All reflection stacks used |

### 6. Enums

#### Biome

| Variant | Value | Description |
|---------|-------|-------------|
| A | 0 | Biome A (Acts 1, 3) |
| B | 1 | Biome B (Acts 2, 4) |

#### Week

| Variant | Value | Description |
|---------|-------|-------------|
| One | 0 | Week 1 bosses (5 archetypes) |
| Two | 1 | Week 2 bosses (5 archetypes) |
| Three | 2 | Week 3 finals (2 per biome) |

### 7. BossInstance (Runtime)

Computed boss state for a specific stage combat.

| Field | Type | Description |
|-------|------|-------------|
| definition | &'static BossDefinition | Reference to static definition |
| scaled_stats | BossStats | Stats after act/tier scaling |
| effects | Vec<ItemEffect> | Traits converted to ItemEffect format |

### 8. BossCombatState (Runtime, in CombatState)

Extended state for boss-specific mechanics during combat.

| Field | Type | Description |
|-------|------|-------------|
| phase_state | PhaseState | Tracks phase transitions |
| reflection_state | Option<ReflectionState> | For Crystal Mimic |
| countdown_states | Vec<CountdownState> | Active countdowns |

### 9. PhaseState (Runtime)

Tracks HP threshold phase activations for Week 3 finals.

| Field | Type | Description |
|-------|------|-------------|
| phase_1_triggered | bool | HP crossed 75% threshold |
| phase_2_triggered | bool | HP crossed 50% threshold |
| phase_3_triggered | bool | HP crossed 25% threshold |

### 10. ReflectionState (Runtime)

Tracks reflection stacks for Crystal Mimic.

| Field | Type | Description |
|-------|------|-------------|
| stacks | u8 | Remaining reflection stacks |
| glass_heart_active | bool | True when stacks depleted |

### 11. CountdownState (Runtime)

Tracks countdown traits.

| Field | Type | Description |
|-------|------|-------------|
| trait_index | u8 | Which trait this countdown is for |
| turns_remaining | u8 | Turns until trigger |

## Boss Registry (24 Bosses)

### Biome A Bosses (12)

**Week 1 (5)**

| ID | Name | Stats (HP/ATK/ARM/SPD/DIG) | Weaknesses | Traits |
|----|------|----------------------------|------------|--------|
| B-A-W1-01 | The Broodmother | 32/2/2/3/1 | STONE, FROST | Swarm Queen (3 strikes), Webbed Strikes (every other turn 1 Chill) |
| B-A-W1-02 | Obsidian Golem | 40/3/14/0/3 | RUST, BLAST | Hardened Core (+4 ARM/turn), Cracked Shell (non-weapon removes 2 ARM) |
| B-A-W1-03 | Gas Anomaly | 34/2/0/2/2 | BLOOD, TEMPO | Toxic Seep (2 non-weapon dmg/turn), Fume Panic (Wounded +1 SPD) |
| B-A-W1-04 | Mad Miner | 36/3/6/2/4 | SCOUT, GREED | Undermine (Exposed if DIG<boss), Claim Jump (+1 strike if Exposed T1) |
| B-A-W1-05 | Shard Colossus | 38/2/6/1/2 | STONE, BLOOD | Prismatic Spines (+8 Shrapnel start), Refracting Hide (+4 Shrapnel/2 turns) |

**Week 2 (5)**

| ID | Name | Stats (HP/ATK/ARM/SPD/DIG) | Weaknesses | Traits |
|----|------|----------------------------|------------|--------|
| B-A-W2-01 | Drill Sergeant | 46/2/10/3/3 | FROST, TEMPO | Rev Up (+1 ATK +1 SPD/turn), Formation (+2 ARM/2 turns) |
| B-A-W2-02 | Crystal Mimic | 50/4/8/2/2 | BLAST, SCOUT | Prismatic Reflection (2 stacks), Glass Heart (+2 non-weapon taken) |
| B-A-W2-03 | Rust Regent | 48/3/8/2/3 | BLOOD, TEMPO | Corroding Edict (1 Rust/hit), Execution Tax (2 dmg if Exposed at turn start) |
| B-A-W2-04 | Powder Keg Baron | 44/3/6/2/2 | STONE, FROST | Volatile Countdown(3) (10 dmg both), Short Fuse (Wounded -1 countdown) |
| B-A-W2-05 | Greedkeeper | 52/2/12/1/2 | GREED, RUST | Toll Collector (steal 10 Gold), Gilded Barrier (ARM = stolen/5 cap 6) |

**Week 3 Finals (2)**

| ID | Name | Stats (HP/ATK/ARM/SPD/DIG) | Weaknesses | Traits |
|----|------|----------------------------|------------|--------|
| B-A-W3-01 | The Eldritch Mole | 72/5/12/3/4 | RUST, TEMPO | Three Phases (75%: +10 ARM, 50%: 2 strikes, 25%: 2 Bleed/turn), Deep Dig (DIG reduces Phase 1) |
| B-A-W3-02 | The Gilded Devourer | 68/4/10/2/3 | GREED, BLOOD | Tax Feast (Gold→ARM +1/5 cap 10), Hunger (Wounded: 3 Bleed) |

### Biome B Bosses (12)

**Week 1 Variants (5)** - Same archetypes as Biome A with +1 SPD baseline (cap 4)

| ID | Name | Stats | Notes |
|----|------|-------|-------|
| B-B-W1-01 | The Broodmother | 32/2/2/4/1 | SPD 3→4 |
| B-B-W1-02 | Obsidian Golem | 40/3/14/1/3 | SPD 0→1 |
| B-B-W1-03 | Gas Anomaly | 34/2/0/3/2 | SPD 2→3 |
| B-B-W1-04 | Mad Miner | 36/3/6/3/4 | SPD 2→3 |
| B-B-W1-05 | Shard Colossus | 38/2/6/2/2 | SPD 1→2 |

**Week 2 Variants (5)** - Same archetypes with +1 SPD baseline (cap 4)

| ID | Name | Stats | Notes |
|----|------|-------|-------|
| B-B-W2-01 | Drill Sergeant | 46/2/10/4/3 | SPD 3→4 (capped) |
| B-B-W2-02 | Crystal Mimic | 50/4/8/3/2 | SPD 2→3 |
| B-B-W2-03 | Rust Regent | 48/3/8/3/3 | SPD 2→3 |
| B-B-W2-04 | Powder Keg Baron | 44/3/6/3/2 | SPD 2→3 |
| B-B-W2-05 | Greedkeeper | 52/2/12/2/2 | SPD 1→2 |

**Week 3 Finals (2)** - Unique Biome B bosses

| ID | Name | Stats (HP/ATK/ARM/SPD/DIG) | Weaknesses | Traits |
|----|------|----------------------------|------------|--------|
| B-B-W3-01 | The Frostbound Leviathan | 74/4/14/2/3 | TEMPO, STONE | Whiteout (3 Chill start), Glacial Bulk (+4 ARM/2 turns), Crack Ice (Exposed: clear Chill +2 SPD) |
| B-B-W3-02 | The Rusted Chronomancer | 66/5/8/4/2 | RUST, BLOOD | Time Shear (2 strikes T1), Oxidized Future (1 Rust/turn), Blood Price (Wounded: 4 Bleed) |

## Stat Scaling Rules

### Tier Calculation

```
tier = floor((stage_in_act - 1) / 5)  // 0, 1, 2, or 3
```

### Week 1 Scaling

| Tier | HP Bonus | ARM Bonus | ATK Bonus |
|------|----------|-----------|-----------|
| 0 | +0 | +0 | +0 |
| 1 | +2 | +1 | +0 |
| 2 | +4 | +2 | +0 |
| 3 | +6 | +3 | +0 |

### Week 2 Scaling

| Tier | HP Bonus | ARM Bonus | ATK Bonus |
|------|----------|-----------|-----------|
| 0 | +0 | +0 | +0 |
| 1 | +3 | +1 | +0 |
| 2 | +6 | +2 | +1 |
| 3 | +9 | +3 | +1 |

### Week 3 Scaling

| Tier | HP Bonus | ARM Bonus | ATK Bonus |
|------|----------|-----------|-----------|
| 0 | +0 | +0 | +0 |
| 1 | +4 | +1 | +1 |
| 2 | +8 | +2 | +1 |
| 3 | +12 | +3 | +1 |

### Act Baseline Bonuses

| Act | Week 1/2 | Week 3 |
|-----|----------|--------|
| 1 (A) | +0 ATK, +0 SPD | +0 ATK, +0 SPD |
| 2 (B) | +0 ATK, +0 SPD | +0 ATK, +0 SPD |
| 3 (A+) | +1 ATK | +2 ATK |
| 4 (B+) | +1 ATK, +1 SPD | +2 ATK, +1 SPD |

## Relationships

```
GameState (gameplay-state)
    │
    ├── current_stage: u8 ──────┐
    │                           │
    │                           ▼
    │               ┌───────────────────────┐
    │               │ select_boss(stage,    │
    │               │             week)     │
    │               └───────────┬───────────┘
    │                           │
    │                           ▼
    │               ┌───────────────────────┐
    │               │ BossDefinition        │
    │               │ (static, 24 total)    │
    │               └───────────┬───────────┘
    │                           │
    │                           ▼
    │               ┌───────────────────────┐
    │               │ scale_boss(def, act,  │
    │               │            tier)      │
    │               └───────────┬───────────┘
    │                           │
    │                           ▼
    │               ┌───────────────────────┐
    │               │ BossInstance          │
    │               │ (scaled stats)        │
    │               └───────────┬───────────┘
    │                           │
    └────► CombatState ◄────────┘
           (combat-system)
```

## Validation Rules

### Boss Selection

- `stage` MUST be 1-80
- `week` MUST be One, Two, or Three
- Act = floor((stage-1)/20), MUST be 0-3
- Biome = A if act is even, B if act is odd

### Stat Scaling

- All scaled stats MUST be >= base stats
- SPD cap of 4 for Biome B +1 SPD baseline
- No overflow possible (max HP ~90, max stats ~20)

### Phase Mechanics

- Phase thresholds: 75%, 50%, 25%
- Each phase triggers exactly once
- Phase check occurs after any damage
- Multiple phases can trigger in same turn if HP crosses multiple thresholds

### Reflection Mechanics

- Initial stacks = 2 for Crystal Mimic
- Reflects Chill, Rust, Bleed, Shrapnel applications
- Does NOT reflect direct damage
- Glass Heart activates immediately when stacks reach 0

## State Transitions

### Boss Selection Flow

```
Stage Number
    │
    ├── Calculate Act (0-3)
    │       act = (stage - 1) / 20
    │
    ├── Calculate Stage in Act (1-20)
    │       stage_in_act = (stage - 1) % 20 + 1
    │
    ├── Determine Biome
    │       biome = (act % 2 == 0) ? A : B
    │
    └── Select Boss Index
            Week 1: (stage_in_act - 1) % 5
            Week 2: (stage_in_act - 1 + 2) % 5
            Week 3: stage_in_act % 2
```

### Combat Phase Transitions (Week 3 Finals)

```
┌────────────────┐
│ Combat Start   │
│ HP = 100%      │
└───────┬────────┘
        │
        ▼
┌────────────────┐
│ Normal Combat  │◄──────────────────────┐
│ (no phases)    │                       │
└───────┬────────┘                       │
        │ damage dealt                   │
        ▼                                │
┌────────────────┐                       │
│ Check HP %     │                       │
└───────┬────────┘                       │
        │                                │
   ┌────┴────┬────────┬────────┐        │
   ▼         ▼        ▼        ▼        │
HP>75%   HP≤75%   HP≤50%   HP≤25%      │
   │     Phase1   Phase2   Phase3       │
   │     trigger  trigger  trigger      │
   │         │        │        │        │
   └─────────┴────────┴────────┴────────┘
```
