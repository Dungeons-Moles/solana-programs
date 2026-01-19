# Quickstart: Field Enemies

**Feature**: 005-field-enemies  
**Date**: 2026-01-19

## Overview

This guide covers implementing the field enemies system for Dungeons & Moles. Field enemies are the primary combat encounters during exploration, providing Gold rewards and tactical variety through 12 unique archetypes with 3 tiers each.

## Prerequisites

- Existing workspace with: `session-manager`, `map-generator`, `gameplay-state`, `combat-system`
- Rust 1.75+ with Solana BPF target
- Anchor 0.32+
- Understanding of combat-system triggers/effects

## Quick Start

### 1. Create Program Crate

```bash
anchor new field-enemies
```

Update `Cargo.toml`:

```toml
[dependencies]
anchor-lang = "0.32.0"
combat-system = { path = "../combat-system" }
map-generator = { path = "../map-generator" }
```

### 2. Define Enemy Archetypes (Static Data)

```rust
// src/archetypes.rs

use combat_system::state::{ItemEffect, TriggerType, EffectType};

pub const ARCHETYPE_COUNT: usize = 12;
pub const TIER_COUNT: usize = 3;

#[derive(Clone, Copy)]
pub struct EnemyArchetype {
    pub id: u8,
    pub name: &'static str,
    pub emoji: &'static str,
    pub biome_a_weight: u8,
    pub biome_b_weight: u8,
}

#[derive(Clone, Copy)]
pub struct EnemyStats {
    pub hp: u16,
    pub atk: u8,
    pub arm: u8,
    pub spd: u8,
    pub dig: u8,
}

// Stats indexed by archetype_id * 3 + tier
pub static ENEMY_STATS: [[EnemyStats; TIER_COUNT]; ARCHETYPE_COUNT] = [
    // Tunnel Rat (id=0)
    [
        EnemyStats { hp: 5, atk: 1, arm: 0, spd: 3, dig: 1 },
        EnemyStats { hp: 7, atk: 2, arm: 0, spd: 4, dig: 1 },
        EnemyStats { hp: 9, atk: 3, arm: 1, spd: 5, dig: 2 },
    ],
    // ... (11 more archetypes)
];
```

### 3. Implement Spawn Logic

```rust
// programs/field-enemies/src/lib.rs

/// Simple linear congruential generator for deterministic pseudo-random values
#[inline]
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

// In initialize_map_enemies instruction:
pub fn initialize_map_enemies(ctx: Context<InitializeMapEnemies>, act: u8, level: u8) -> Result<()> {
    // Generate seed from session key and level
    let session_bytes = ctx.accounts.session.key().to_bytes();
    let mut seed: u64 = level as u64;
    for (i, byte) in session_bytes.iter().enumerate().take(8) {
        seed = seed.wrapping_add((*byte as u64) << (i * 8));
    }

    for i in 0..spawn_count {
        // Generate pseudo-random values
        seed = lcg_next(seed);
        let tier_rand = (seed >> 8) as u8;
        seed = lcg_next(seed);
        let arch_rand = (seed >> 8) as u8;
        seed = lcg_next(seed);
        let pos_rand = seed;

        let tier = spawner::sample_tier(tier_rand, act);
        let archetype_id = spawner::sample_archetype(arch_rand, act);

        // Simple grid distribution
        let x = ((pos_rand as u8).wrapping_add(i)) % MAP_WIDTH;
        let y = (((pos_rand >> 8) as u8).wrapping_add(i.wrapping_mul(3))) % MAP_HEIGHT;

        // ... push to enemies vector
    }
}
```

### 4. Define Enemy Traits

```rust
// src/traits.rs

use combat_system::state::{ItemEffect, TriggerType, EffectType};

pub fn get_enemy_traits(archetype_id: u8) -> &'static [ItemEffect] {
    match archetype_id {
        0 => &TUNNEL_RAT_TRAITS,      // On Hit: steal 1 Gold
        1 => &CAVE_BAT_TRAITS,        // Every other turn: restore 1 HP
        2 => &SPORE_SLIME_TRAITS,     // Battle Start: apply 2 Chill
        // ... etc
        _ => &[],
    }
}

static TUNNEL_RAT_TRAITS: [ItemEffect; 1] = [
    ItemEffect {
        trigger: TriggerType::OnHit,
        once_per_turn: true,
        effect_type: EffectType::StealGold, // Note: may need new EffectType
        value: 1,
    },
];

static SPORE_SLIME_TRAITS: [ItemEffect; 1] = [
    ItemEffect {
        trigger: TriggerType::BattleStart,
        once_per_turn: false,
        effect_type: EffectType::ApplyChill,
        value: 2,
    },
];
```

### 5. Integrate with Combat System

```rust
// In gameplay-state when player steps on enemy

let enemy = map_enemies.get_enemy_at(player_x, player_y)?;
if enemy.is_some() && !enemy.defeated {
    let stats = field_enemies::get_enemy_stats(enemy.archetype_id, enemy.tier);
    let traits = field_enemies::get_enemy_traits(enemy.archetype_id);

    let enemy_input = CombatantInput {
        hp: stats.hp as i16,
        max_hp: stats.hp,
        atk: stats.atk as i16,
        arm: stats.arm as i16,
        spd: stats.spd as i16,
        dig: stats.dig as i16,
        strikes: 1,
    };

    // Start combat with enemy_input and traits
    combat_system::start_combat(player_input, enemy_input, player_effects, traits)?;
}
```

### 6. Handle Combat Victory

```rust
// After combat resolves with player victory

let tier = field_enemies::cpi_mark_enemy_defeated(ctx, x, y)?;
let gold_reward = (tier + 1) * 2;  // T1=2, T2=4, T3=6
game_state.gold += gold_reward;

emit!(EnemyDefeated {
    session: game_state.session,
    archetype_id: enemy.archetype_id,
    tier,
    x,
    y,
    gold_reward,
});
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enemy_stats_match_gdd() {
        let tunnel_rat_t1 = get_enemy_stats(0, EnemyTier::T1);
        assert_eq!(tunnel_rat_t1.hp, 5);
        assert_eq!(tunnel_rat_t1.atk, 1);
        assert_eq!(tunnel_rat_t1.arm, 0);
        assert_eq!(tunnel_rat_t1.spd, 3);
        assert_eq!(tunnel_rat_t1.dig, 1);
    }

    #[test]
    fn test_tier_distribution() {
        let mut rng = SeededRng::new(12345);
        let mut counts = [0u32; 3];

        for _ in 0..1000 {
            let tier = sample_tier(&mut rng, &TIER_DISTRIBUTIONS[0]);
            counts[tier as usize] += 1;
        }

        // Act 1: 70/25/5 distribution
        assert!((counts[0] as f32 / 1000.0 - 0.70).abs() < 0.05);
        assert!((counts[1] as f32 / 1000.0 - 0.25).abs() < 0.05);
    }
}
```

### Integration Tests

```typescript
// tests/field-enemies.ts

describe("field-enemies", () => {
  it("spawns correct enemy count per act", async () => {
    for (const act of [1, 2, 3, 4]) {
      const expected = [36, 40, 44, 48][act - 1];
      const mapEnemies = await initializeMapEnemies(session, act);
      expect(mapEnemies.count).to.equal(expected);
    }
  });

  it("awards correct gold on defeat", async () => {
    // Spawn T2 enemy, defeat it, verify +4 gold
    const enemy = mapEnemies.enemies[0];
    enemy.tier = 1; // T2

    await startCombat(player, enemy);
    await resolveCombat(); // Player wins

    const gameState = await getGameState();
    expect(gameState.gold).to.equal(4);
  });
});
```

## Key Files

| File                | Purpose                           |
| ------------------- | --------------------------------- |
| `src/lib.rs`        | Program entry, instructions       |
| `src/state.rs`      | MapEnemies, EnemyInstance structs |
| `src/archetypes.rs` | Static enemy definitions          |
| `src/spawner.rs`    | Spawn logic, tier/biome sampling  |
| `src/traits.rs`     | Enemy trait definitions           |
| `src/constants.rs`  | Distribution tables, spawn counts |
| `src/errors.rs`     | Custom errors                     |

## Common Issues

1. **Trait not triggering**: Verify TriggerType matches combat phase
2. **Wrong stats**: Check archetype_id \* 3 + tier indexing
3. **Spawn distribution off**: Ensure RNG is properly seeded per level
4. **Position collisions**: Verify floor tile sampling excludes used positions

## Next Steps

After implementation:

1. Run `anchor build` and verify no warnings
2. Run `cargo clippy` for linting
3. Run full test suite with `anchor test`
4. Update `Anchor.toml` with new program ID
