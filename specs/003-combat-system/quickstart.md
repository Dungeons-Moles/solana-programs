# Quickstart: Combat System & Status Effects

**Feature**: 003-combat-system
**Date**: 2026-01-18

## Overview

The combat system is a deterministic auto-battle engine for Dungeons & Moles. It resolves combat turn-by-turn based on combatant stats, processes status effects, and supports item trigger effects.

## Key Concepts

### Combat Flow

```
1. Initialize Combat (create CombatState)
2. Battle Start triggers fire
3. For each turn (1-50):
   a. Turn Start triggers
   b. First actor attacks (higher SPD)
   c. Second actor attacks (if alive)
   d. Status effects decay
   e. Check for victory
4. Failsafe at Turn 50 (HP% comparison)
5. Close Combat (return result)
```

### Damage Formula

```
Weapon damage = max(0, attacker_ATK - target_ARM)
Non-weapon damage = damage (ignores ARM)
```

### Status Effects

| Effect | Application | Each Turn | Decay |
|--------|-------------|-----------|-------|
| Chill | -N strikes (min 1) | At turn start | -1 stack at turn end |
| Shrapnel | N damage to attacker when struck | When hit | Clears at turn end |
| Rust | -N ARM at turn end | At turn end | Persists |
| Bleed | N damage at turn end | At turn end | -1 stack at turn end |

### Trigger Types

| Trigger | When |
|---------|------|
| BattleStart | Before Turn 1 |
| FirstTurn | During Turn 1 only |
| TurnStart | Start of each turn |
| EveryOtherTurn | Turns 2, 4, 6, ... |
| OnHit | When a strike lands (once/turn) |
| Exposed | When ARM = 0 |
| Wounded | When HP < 50% max |

## Usage Examples

### Basic Combat (TypeScript)

```typescript
import { Program } from "@coral-xyz/anchor";
import { CombatSystem } from "../target/types/combat_system";

// Initialize combat
const playerStats = {
  hp: 10,
  maxHp: 10,
  atk: 3,
  arm: 2,
  spd: 2,
  dig: 1,
  strikes: 1,
};

const enemyStats = {
  hp: 8,
  maxHp: 8,
  atk: 2,
  arm: 1,
  spd: 1,
  dig: 1,
  strikes: 1,
};

await program.methods
  .initializeCombat(playerStats, enemyStats)
  .accounts({
    combatState: combatStatePda,
    gameState: gameStatePda,
    player: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

// Resolve combat (single transaction)
const playerEffects = [
  {
    trigger: { onHit: {} },
    oncePerTurn: true,
    effectType: { applyChill: {} },
    value: 1,
  },
];

const enemyEffects = [
  {
    trigger: { battleStart: {} },
    oncePerTurn: false,
    effectType: { gainArmor: {} },
    value: 2,
  },
];

await program.methods
  .resolveCombat(playerEffects, enemyEffects)
  .accounts({
    combatState: combatStatePda,
    player: wallet.publicKey,
  })
  .rpc();

// Fetch result
const state = await program.account.combatState.fetch(combatStatePda);
console.log(`Player won: ${state.playerWon}`);
console.log(`Final turn: ${state.turn}`);
```

### Unit Testing Combat Logic (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_calculation() {
        let attacker_atk = 5;
        let defender_arm = 3;
        let damage = calculate_weapon_damage(attacker_atk, defender_arm);
        assert_eq!(damage, 2);
    }

    #[test]
    fn test_damage_floors_at_zero() {
        let attacker_atk = 2;
        let defender_arm = 5;
        let damage = calculate_weapon_damage(attacker_atk, defender_arm);
        assert_eq!(damage, 0);
    }

    #[test]
    fn test_chill_reduces_strikes() {
        let base_strikes = 3;
        let chill_stacks = 2;
        let actual_strikes = apply_chill(base_strikes, chill_stacks);
        assert_eq!(actual_strikes, 1); // 3 - 2 = 1
    }

    #[test]
    fn test_chill_minimum_one_strike() {
        let base_strikes = 2;
        let chill_stacks = 5;
        let actual_strikes = apply_chill(base_strikes, chill_stacks);
        assert_eq!(actual_strikes, 1); // min 1
    }

    #[test]
    fn test_determinism() {
        let player = CombatantInput { hp: 10, max_hp: 10, atk: 3, arm: 2, spd: 2, dig: 1, strikes: 1 };
        let enemy = CombatantInput { hp: 8, max_hp: 8, atk: 2, arm: 1, spd: 1, dig: 1, strikes: 1 };

        let result1 = resolve_combat(&player, &enemy, &[], &[]);
        let result2 = resolve_combat(&player, &enemy, &[], &[]);

        assert_eq!(result1.player_won, result2.player_won);
        assert_eq!(result1.final_turn, result2.final_turn);
        assert_eq!(result1.player_remaining_hp, result2.player_remaining_hp);
    }
}
```

## Development Commands

```bash
# Build the program
anchor build -p combat_system

# Run unit tests
cargo test -p combat-system

# Run integration tests
anchor test --skip-local-validator

# Generate IDL
anchor idl parse -f programs/combat-system/src/lib.rs -o target/idl/combat_system.json
```

## Integration with gameplay-state

The combat system is invoked via CPI from gameplay-state when a player encounters an enemy:

```rust
// In gameplay-state program
pub fn handle_enemy_encounter(ctx: Context<HandleEncounter>) -> Result<()> {
    // 1. Get player stats from GameState
    let player_stats = CombatantInput {
        hp: ctx.accounts.game_state.hp as i16,
        max_hp: ctx.accounts.game_state.max_hp as u16,
        atk: ctx.accounts.game_state.atk as i16,
        arm: ctx.accounts.game_state.arm as i16,
        spd: ctx.accounts.game_state.spd as i16,
        dig: ctx.accounts.game_state.dig as i16,
        strikes: 1, // From equipped tool
    };

    // 2. Get enemy stats (passed as argument or from account)
    let enemy_stats = ctx.accounts.enemy_data.to_combatant_input();

    // 3. Initialize and resolve combat via CPI
    let cpi_accounts = InitializeCombat {
        combat_state: ctx.accounts.combat_state.to_account_info(),
        game_state: ctx.accounts.game_state.to_account_info(),
        player: ctx.accounts.player.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    combat_system::cpi::initialize_combat(
        CpiContext::new(ctx.accounts.combat_program.to_account_info(), cpi_accounts),
        player_stats,
        enemy_stats,
    )?;

    // 4. Resolve combat
    combat_system::cpi::resolve_combat(/* ... */)?;

    // 5. Handle result (update HP, grant gold, etc.)
    Ok(())
}
```

## Common Patterns

### Adding a New Status Effect

1. Add variant to `StatusEffects` struct
2. Add processing in `process_turn_end()`
3. Add application in `apply_effect()`
4. Add decay logic if applicable
5. Update tests

### Adding a New Trigger Type

1. Add variant to `TriggerType` enum
2. Add check location in combat loop
3. Add condition evaluation in `should_trigger()`
4. Update tests

### Adding a New Effect Type

1. Add variant to `EffectType` enum
2. Add processing in `apply_effect()`
3. Update IDL
4. Update tests
