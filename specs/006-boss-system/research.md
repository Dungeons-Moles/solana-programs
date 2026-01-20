# Research: Boss System

**Feature**: 006-boss-system
**Date**: 2026-01-19
**Status**: Complete

## Research Questions

### 1. Boss Definition Storage Architecture

**Decision**: Compile-time constants with static arrays (same as items and field enemies)

**Rationale**:
- 24 boss definitions are immutable game data from GDD
- Compile-time constants are free to access (no account reads)
- Follows established pattern from `player-inventory` (80 items) and `field-enemies` (12 archetypes)
- Program size increase is minimal (~5KB for all boss data)

**Alternatives Considered**:
- **On-chain config accounts**: Rejected - adds 24 PDAs, rent costs, no runtime modification needed
- **External data loading**: Rejected - complicates build, no benefit for static data

### 2. Boss Trait Representation

**Decision**: Reuse `ItemEffect` struct from combat-system with extensions

**Rationale**:
- Most boss traits map directly to existing TriggerType + EffectType combinations
- BattleStart, TurnStart, OnHit, EveryOtherTurn, Wounded, FirstTurn already exist
- Combat system's `process_triggers_for_phase` handles these without modification
- Only new mechanics needed: Countdown (already added for Powder Tick), Reflection, Phases

**New Trait Mechanics Required**:

| Mechanic | Boss(es) | Implementation |
|----------|----------|----------------|
| Countdown | Powder Keg Baron | Use existing Countdown from field-enemies; track turns_remaining |
| Reflection | Crystal Mimic | New: ReflectionState { stacks: u8 }; intercept status applications |
| Phases | Eldritch Mole, Gilded Devourer, Frostbound Leviathan, Rusted Chronomancer | New: PhaseState tracking HP thresholds crossed |
| Gold Steal | Greedkeeper, Gilded Devourer | New: EffectType::StealGold; BattleStart trigger |
| Conditional Triggers | Mad Miner (DIG compare), Frost Wisp (act first Turn 1) | Extend trigger conditions |

### 3. Biome B Variant Handling

**Decision**: Separate boss definitions for Biome B variants, not runtime modification

**Rationale**:
- Biome B bosses have +1 SPD baseline and one modified trait line
- Storing as separate definitions (B-B-W1-01 through B-B-W2-05) is clearer
- GDD doesn't specify exact trait modifications for Biome B variants
- For now: copy Biome A stats with +1 SPD (cap 4), same traits

**Implementation**:
- 12 Biome A bosses (5 W1, 5 W2, 2 W3 finals)
- 12 Biome B entries (5 W1 variants, 5 W2 variants, 2 W3 finals - unique)
- Total: 24 boss definitions

### 4. Boss Selection Algorithm

**Decision**: Deterministic formula based on stage number

**Rationale**:
- Stage 1-20 = Act 1 (Biome A), 21-40 = Act 2 (Biome B), 41-60 = Act 3 (Biome A+), 61-80 = Act 4 (Biome B+)
- Within act: `stage_in_act = (stage - 1) % 20 + 1`
- Week 1: `boss_index = (stage_in_act - 1) % 5`
- Week 2: `boss_index = (stage_in_act - 1 + 2) % 5` (offset by 2 to avoid same pairing)
- Week 3: `final_index = stage_in_act % 2` (odd=Final1, even=Final2)

**Algorithm**:
```rust
fn select_boss(stage: u8, week: Week) -> &'static BossDefinition {
    let act = (stage - 1) / 20;           // 0-3
    let stage_in_act = (stage - 1) % 20 + 1;  // 1-20
    let biome = if act % 2 == 0 { Biome::A } else { Biome::B };

    match week {
        Week::One => {
            let boss_idx = (stage_in_act - 1) % 5;
            get_week1_boss(biome, boss_idx)
        }
        Week::Two => {
            let boss_idx = (stage_in_act - 1 + 2) % 5;
            get_week2_boss(biome, boss_idx)
        }
        Week::Three => {
            let final_idx = stage_in_act % 2;  // 0 or 1
            get_week3_boss(biome, final_idx)
        }
    }
}
```

### 5. Stat Scaling Implementation

**Decision**: Pure function that computes scaled stats from base stats + act + tier

**Rationale**:
- Scaling formulas are clearly defined in GDD
- No state needed; compute on demand
- Returns new BossInstance with scaled values

**Scaling Formulas**:
```rust
fn scale_boss(base: &BossDefinition, act: u8, stage_in_act: u8, week: Week) -> BossInstance {
    let tier = (stage_in_act - 1) / 5;  // 0-3

    let mut hp = base.hp;
    let mut atk = base.atk;
    let mut arm = base.arm;
    let mut spd = base.spd;

    // Tier scaling (Acts 3 & 4 only, but formulas work for all)
    match week {
        Week::One => {
            hp += 2 * tier;
            arm += tier;
        }
        Week::Two => {
            hp += 3 * tier;
            arm += tier;
            if tier >= 2 { atk += 1; }
        }
        Week::Three => {
            hp += 4 * tier;
            arm += tier;
            if tier >= 1 { atk += 1; }
        }
    }

    // Act baseline bonuses
    match act {
        2 => {  // Act 3 (A+)
            atk += if week == Week::Three { 2 } else { 1 };
        }
        3 => {  // Act 4 (B+)
            atk += if week == Week::Three { 2 } else { 1 };
            spd += 1;
        }
        _ => {}
    }

    BossInstance { hp, atk, arm, spd, dig: base.dig, traits: base.traits }
}
```

### 6. Phase Mechanics Implementation

**Decision**: PhaseState struct tracked during combat, checked after damage

**Rationale**:
- Only 4 bosses have phases (Week 3 finals)
- Phase thresholds: 75%, 50%, 25% of max HP
- Each phase activates once when HP crosses threshold
- Phase effects are additional trait activations

**Implementation**:
```rust
pub struct PhaseState {
    pub phase_1_active: bool,  // HP <= 75%
    pub phase_2_active: bool,  // HP <= 50%
    pub phase_3_active: bool,  // HP <= 25%
}

fn check_phase_transition(boss_hp: i16, boss_max_hp: u16, phases: &mut PhaseState) -> Option<Phase> {
    let hp_percent = (boss_hp as u32 * 100) / boss_max_hp as u32;

    if hp_percent <= 25 && !phases.phase_3_active {
        phases.phase_3_active = true;
        return Some(Phase::Three);
    }
    if hp_percent <= 50 && !phases.phase_2_active {
        phases.phase_2_active = true;
        return Some(Phase::Two);
    }
    if hp_percent <= 75 && !phases.phase_1_active {
        phases.phase_1_active = true;
        return Some(Phase::One);
    }
    None
}
```

### 7. Reflection Mechanic Implementation

**Decision**: ReflectionState struct, intercept status effect application

**Rationale**:
- Crystal Mimic has 2 reflection stacks
- When player applies status (Chill, Rust, Bleed, Shrapnel), it reflects back
- After stacks depleted, Glass Heart activates (+2 non-weapon damage taken)

**Implementation**:
```rust
pub struct ReflectionState {
    pub stacks: u8,
    pub glass_heart_active: bool,
}

fn apply_status_with_reflection(
    target: &mut StatusEffects,
    source: &mut StatusEffects,
    effect: EffectType,
    value: i16,
    reflection: &mut Option<ReflectionState>,
) {
    if let Some(ref mut r) = reflection {
        if r.stacks > 0 {
            // Reflect to source instead
            apply_status_effect(source, effect, value);
            r.stacks -= 1;
            if r.stacks == 0 {
                r.glass_heart_active = true;
            }
            return;
        }
    }
    apply_status_effect(target, effect, value);
}
```

### 8. Combat System Integration

**Decision**: Boss traits generate ItemEffect arrays, passed to combat system

**Rationale**:
- Combat system accepts `Vec<ItemEffect>` for combatant effects
- Boss traits use same format with additional runtime state for special mechanics
- New mechanics (Reflection, Phases) require combat system extension

**Interface**:
```rust
// In boss-system
pub fn get_boss_effects(boss: &BossDefinition, tier: Tier) -> Vec<ItemEffect> {
    boss.traits.iter()
        .map(|t| t.to_item_effect(tier))
        .collect()
}

// Called by gameplay-state when starting boss combat
let boss = select_boss(stage, week);
let scaled = scale_boss(&boss, act, stage_in_act, week);
let effects = get_boss_effects(&boss, Tier::I);  // Tier for effect values

combat_system::initialize_combat(
    player_input,
    CombatantInput::from_boss(&scaled),
    player_effects,
    effects,
)?;
```

## Dependencies Confirmed

| Dependency | Status | Notes |
|------------|--------|-------|
| combat-system TriggerType | EXISTS | BattleStart, TurnStart, OnHit, EveryOtherTurn, Wounded, FirstTurn, Countdown |
| combat-system EffectType | EXISTS | All needed except StealGold (new) |
| combat-system ItemEffect | EXISTS | Reusable for boss traits |
| gameplay-state GameState | EXISTS | Has player Gold for steal mechanics |
| field-enemies Countdown | EXISTS | Reusable for Powder Keg Baron |

## New Types Required

| Type | Purpose | Location |
|------|---------|----------|
| BossDefinition | Static boss data | boss-system/bosses.rs |
| BossTrait | Trait with optional special mechanics | boss-system/traits.rs |
| BossInstance | Scaled boss for specific stage | boss-system/scaling.rs |
| PhaseState | Track HP threshold phases | combat-system extension |
| ReflectionState | Track reflection stacks | combat-system extension |
| EffectType::StealGold | Gold theft mechanic | combat-system extension |

## Open Items

None. All research questions resolved.
