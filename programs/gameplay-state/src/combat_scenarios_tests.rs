use super::{build_player_combatant, preprocess_enemy_effects};
use crate::stats::calculate_stats;
use anchor_lang::prelude::Pubkey;
use combat_system::{
    resolve_combat_with_player_gold, CombatLogEntry, CombatOutcome, LogAction, STATUS_BLEED,
    STATUS_CHILL, STATUS_RUST, STATUS_SHRAPNEL,
};
use field_enemies::archetypes::{get_enemy_combatant_input, ids};
use player_inventory::effects::generate_combat_effects;
use player_inventory::items::{BASIC_PICKAXE, G_ST_01, G_ST_02};
use player_inventory::state::{ItemInstance, PlayerInventory, Tier, ToolOilModification};

fn make_inventory() -> PlayerInventory {
    PlayerInventory {
        session: Pubkey::default(),
        player: Pubkey::default(),
        tool: None,
        gear: [None; 12],
        gear_slot_capacity: 4,
        bump: 0,
    }
}

fn tool_with_oils(item_id: [u8; 8], tier: Tier, oils: &[ToolOilModification]) -> ItemInstance {
    let mut tool = ItemInstance::new(item_id, tier);
    for oil in oils {
        tool.apply_oil(*oil);
    }
    tool
}

fn basic_pickaxe_with_oils(oils: &[ToolOilModification]) -> ItemInstance {
    tool_with_oils(*BASIC_PICKAXE.id, Tier::I, oils)
}

fn miner_helmet() -> ItemInstance {
    ItemInstance::new(*G_ST_01.id, Tier::I)
}

fn work_vest() -> ItemInstance {
    ItemInstance::new(*G_ST_02.id, Tier::I)
}

#[derive(Clone, Copy)]
enum ExpectedHp {
    Exact(i16),
    NonPositive,
}

impl ExpectedHp {
    fn assert(&self, actual: i16, name: &str, label: &str) {
        match self {
            ExpectedHp::Exact(expected) => {
                assert_eq!(actual, *expected, "{}: {} mismatch", name, label)
            }
            ExpectedHp::NonPositive => assert!(
                actual <= 0,
                "{}: {} expected <= 0, got {}",
                name,
                label,
                actual
            ),
        }
    }
}

struct Scenario {
    name: &'static str,
    player_hp: i16,
    tool: ItemInstance,
    gear: Vec<ItemInstance>,
    enemy_archetype: u8,
    enemy_tier: u8,
    player_gold: u16,
    expected_player_won: bool,
    expected_final_player_hp: ExpectedHp,
    expected_final_enemy_hp: ExpectedHp,
    expected_turns: u8,
    expected_gold_change: Option<i16>,
}

fn run_combat(
    player_hp: i16,
    tool: ItemInstance,
    gear: Vec<ItemInstance>,
    enemy_archetype: u8,
    enemy_tier: u8,
    player_gold: u16,
) -> CombatOutcome {
    let mut inventory = make_inventory();
    inventory.tool = Some(tool);
    for (slot, item) in gear.iter().enumerate() {
        inventory.gear[slot] = Some(*item);
    }

    let player_stats = calculate_stats(&inventory);
    let player_effects = generate_combat_effects(&inventory);
    let player_input = build_player_combatant(player_hp, &player_stats, &player_effects);

    let enemy_input = get_enemy_combatant_input(enemy_archetype, enemy_tier)
        .expect("enemy combatant input missing");
    let enemy_effects = preprocess_enemy_effects(enemy_archetype, player_gold);

    resolve_combat_with_player_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        player_gold,
    )
    .expect("combat resolution failed")
}

fn resolve_scenario(scenario: &Scenario) -> CombatOutcome {
    run_combat(
        scenario.player_hp,
        scenario.tool,
        scenario.gear.clone(),
        scenario.enemy_archetype,
        scenario.enemy_tier,
        scenario.player_gold,
    )
}

fn assert_scenario(scenario: &Scenario) {
    let outcome = resolve_scenario(scenario);
    assert_eq!(
        outcome.player_won, scenario.expected_player_won,
        "{}: player_won mismatch",
        scenario.name
    );
    scenario.expected_final_player_hp.assert(
        outcome.final_player_hp,
        scenario.name,
        "final_player_hp",
    );
    scenario.expected_final_enemy_hp.assert(
        outcome.final_enemy_hp,
        scenario.name,
        "final_enemy_hp",
    );
    assert_eq!(
        outcome.turns_taken, scenario.expected_turns,
        "{}: turns_taken mismatch",
        scenario.name
    );

    if let Some(expected_gold_change) = scenario.expected_gold_change {
        assert_eq!(
            outcome.gold_change, expected_gold_change,
            "{}: gold_change mismatch",
            scenario.name
        );
    }
}

fn assert_log_contains<F: Fn(&CombatLogEntry) -> bool>(
    log: &[CombatLogEntry],
    name: &str,
    predicate: F,
) {
    assert!(
        log.iter().any(predicate),
        "{}: expected log entry not found",
        name
    );
}

// -----------------------------------------------------------------------------
// Spore Slime T1 outcome scenarios
// -----------------------------------------------------------------------------

#[test]
fn test_spore_slime_t1_miner_helmet_atk_oil_full_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_atk_oil_full_hp",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(7),
        expected_final_enemy_hp: ExpectedHp::NonPositive,
        expected_turns: 5,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_miner_helmet_arm_oil_low_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_arm_oil_low_hp",
        player_hp: 7,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusArm]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(1),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 9,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_miner_helmet_no_oil_low_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_no_oil_low_hp",
        player_hp: 7,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: false,
        expected_final_player_hp: ExpectedHp::NonPositive,
        expected_final_enemy_hp: ExpectedHp::Exact(1),
        expected_turns: 9,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_no_helmet_atk_oil() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_no_helmet_atk_oil",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(4),
        expected_final_enemy_hp: ExpectedHp::NonPositive,
        expected_turns: 5,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_no_helmet_no_oil() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_no_helmet_no_oil",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: false,
        expected_final_player_hp: ExpectedHp::NonPositive,
        expected_final_enemy_hp: ExpectedHp::Exact(1),
        expected_turns: 9,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_miner_helmet_spd_oil() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_spd_oil",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusSpd]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(4),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 9,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_miner_helmet_atk_oil_low_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_atk_oil_low_hp",
        player_hp: 5,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(2),
        expected_final_enemy_hp: ExpectedHp::NonPositive,
        expected_turns: 5,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_no_helmet_atk_oil_low_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_no_helmet_atk_oil_low_hp",
        player_hp: 6,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: false,
        expected_final_player_hp: ExpectedHp::NonPositive,
        expected_final_enemy_hp: ExpectedHp::Exact(1),
        expected_turns: 5,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_t1_miner_helmet_arm_oil_full_hp() {
    assert_scenario(&Scenario {
        name: "spore_slime_t1_miner_helmet_arm_oil_full_hp",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusArm]),
        gear: vec![miner_helmet()],
        enemy_archetype: ids::SPORE_SLIME,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(4),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 9,
        expected_gold_change: None,
    });
}

#[test]
fn test_spore_slime_arm_oil_flips_outcome() {
    let with_arm_oil = run_combat(
        7,
        basic_pickaxe_with_oils(&[ToolOilModification::PlusArm]),
        vec![miner_helmet()],
        ids::SPORE_SLIME,
        0,
        0,
    );
    let without_oil = run_combat(
        7,
        basic_pickaxe_with_oils(&[]),
        vec![miner_helmet()],
        ids::SPORE_SLIME,
        0,
        0,
    );

    // Without forced minimum-through-armor damage, ARM oil now flips the 7 HP
    // matchup: with oil the player survives, without oil the player dies.
    assert!(
        with_arm_oil.player_won,
        "spore_slime_arm_oil_flips_outcome: expected win with armor oil"
    );
    assert!(
        with_arm_oil.final_player_hp > 0,
        "spore_slime_arm_oil_flips_outcome: expected positive HP with armor oil"
    );
    assert!(
        without_oil.final_player_hp <= 0,
        "spore_slime_arm_oil_flips_outcome: expected death without oil"
    );
}

// -----------------------------------------------------------------------------
// Other outcome scenarios
// -----------------------------------------------------------------------------

#[test]
fn test_tunnel_rat_t1_basic_pickaxe() {
    assert_scenario(&Scenario {
        name: "tunnel_rat_t1_basic_pickaxe",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![],
        enemy_archetype: ids::TUNNEL_RAT,
        enemy_tier: 0,
        player_gold: 10,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(5),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 5,
        expected_gold_change: Some(-5),
    });
}

#[test]
fn test_tunnel_rat_t1_atk_oil() {
    assert_scenario(&Scenario {
        name: "tunnel_rat_t1_atk_oil",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![],
        enemy_archetype: ids::TUNNEL_RAT,
        enemy_tier: 0,
        player_gold: 10,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(7),
        expected_final_enemy_hp: ExpectedHp::NonPositive,
        expected_turns: 3,
        expected_gold_change: Some(-3),
    });
}

#[test]
fn test_cave_bat_t1_atk_oil() {
    assert_scenario(&Scenario {
        name: "cave_bat_t1_atk_oil",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        gear: vec![],
        enemy_archetype: ids::CAVE_BAT,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(6),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 4,
        expected_gold_change: None,
    });
}

#[test]
fn test_cave_bat_t1_basic_pickaxe() {
    assert_scenario(&Scenario {
        name: "cave_bat_t1_basic_pickaxe",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![],
        enemy_archetype: ids::CAVE_BAT,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: false,
        expected_final_player_hp: ExpectedHp::NonPositive,
        expected_final_enemy_hp: ExpectedHp::Exact(2),
        expected_turns: 10,
        expected_gold_change: None,
    });
}

#[test]
fn test_rust_mite_t1_basic_pickaxe() {
    assert_scenario(&Scenario {
        name: "rust_mite_t1_basic_pickaxe",
        player_hp: 10,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![],
        enemy_archetype: ids::RUST_MITE_SWARM,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(4),
        expected_final_enemy_hp: ExpectedHp::Exact(0),
        expected_turns: 6,
        expected_gold_change: None,
    });
}

#[test]
fn test_shard_beetle_t1_basic_pickaxe() {
    assert_scenario(&Scenario {
        name: "shard_beetle_t1_basic_pickaxe",
        player_hp: 15,
        tool: basic_pickaxe_with_oils(&[]),
        gear: vec![],
        enemy_archetype: ids::SHARD_BEETLE,
        enemy_tier: 0,
        player_gold: 0,
        expected_player_won: true,
        expected_final_player_hp: ExpectedHp::Exact(4),
        expected_final_enemy_hp: ExpectedHp::NonPositive,
        expected_turns: 10,
        expected_gold_change: None,
    });
}

// -----------------------------------------------------------------------------
// Trait and effect log integration tests
// -----------------------------------------------------------------------------

#[test]
fn test_spore_slime_applies_chill_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::SPORE_SLIME,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "spore_slime_applies_chill_log", |entry| {
        entry.action == LogAction::ApplyStatus
            && entry.is_player
            && entry.extra == STATUS_CHILL
            && entry.value == 1
    });
}

#[test]
fn test_tunnel_rat_steals_gold_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::TUNNEL_RAT,
        0,
        10,
    );

    assert_log_contains(&outcome.log, "tunnel_rat_steals_gold_log", |entry| {
        entry.action == LogAction::GoldStolen && entry.value < 0
    });
}

#[test]
fn test_cave_bat_heals_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::CAVE_BAT,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "cave_bat_heals_log", |entry| {
        entry.action == LogAction::Heal && !entry.is_player && entry.value == 1
    });
}

#[test]
fn test_rust_mite_applies_rust_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::RUST_MITE_SWARM,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "rust_mite_applies_rust_log", |entry| {
        entry.action == LogAction::ApplyStatus
            && entry.is_player
            && entry.extra == STATUS_RUST
            && entry.value == 1
    });
}

#[test]
fn test_shard_beetle_applies_shrapnel_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::SHARD_BEETLE,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "shard_beetle_applies_shrapnel_log", |entry| {
        entry.action == LogAction::ApplyStatus
            && !entry.is_player
            && entry.extra == STATUS_SHRAPNEL
            && entry.value == 1
    });
}

#[test]
fn test_tunnel_warden_removes_armor_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![miner_helmet()],
        ids::TUNNEL_WARDEN,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "tunnel_warden_removes_armor_log", |entry| {
        entry.action == LogAction::ArmorChange && entry.is_player && entry.value < 0
    });
}

#[test]
fn test_burrow_ambusher_battle_start_damage_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::BURROW_AMBUSHER,
        0,
        0,
    );

    assert_log_contains(
        &outcome.log,
        "burrow_ambusher_battle_start_damage_log",
        |entry| entry.action == LogAction::NonWeaponDamage && entry.is_player && entry.value == 1,
    );
}

#[test]
fn test_coin_slug_armor_from_gold_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::COIN_SLUG,
        0,
        30,
    );

    assert_log_contains(&outcome.log, "coin_slug_armor_from_gold_log", |entry| {
        entry.action == LogAction::ArmorChange && !entry.is_player && entry.value == 3
    });
}

#[test]
fn test_blood_mosquito_applies_bleed_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::BLOOD_MOSQUITO,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "blood_mosquito_applies_bleed_log", |entry| {
        entry.action == LogAction::ApplyStatus
            && entry.is_player
            && entry.extra == STATUS_BLEED
            && entry.value == 1
    });
}

#[test]
fn test_arm_oil_battle_start_armor_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[ToolOilModification::PlusArm]),
        vec![],
        ids::TUNNEL_RAT,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "arm_oil_battle_start_armor_log", |entry| {
        entry.action == LogAction::ArmorChange && entry.is_player && entry.value == 1
    });
}

#[test]
fn test_atk_oil_battle_start_atk_log() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        vec![],
        ids::TUNNEL_RAT,
        0,
        0,
    );

    assert_log_contains(&outcome.log, "atk_oil_battle_start_atk_log", |entry| {
        entry.action == LogAction::AtkChange && entry.is_player && entry.value == 1
    });
}

// =============================================================================
// Work Vest specific tests - user's exact scenario
// =============================================================================

/// This test matches the user's exact scenario:
/// - Work Vest T1 (+4 HP, +1 ARM) + Basic Pickaxe with ATK oil (+1 ATK)
/// - vs Spore Slime T1 (8 HP, 1 ATK, 2 ARM, 0 SPD, applies 2 Chill)
///
/// Expected combat flow:
/// - Player starts: 14 HP (10 base + 4 Work Vest), 2 ATK (1 base + 1 oil), 1 ARM (0 base + 1 vest)
/// - Spore Slime applies 1 Chill at BattleStart → player takes +1 damage from Chill
/// - Turn order: SPD tie (0 vs 0), enemy goes first
/// - Turn 1: Slime 1 ATK vs Player 1 ARM → ARM gone (Chill +1 dmg causes 1 HP loss). Player 2 ATK vs Slime 2 ARM → ARM gone.
/// - Turn 2-5: Slime 1 dmg/turn, Player 2 dmg/turn → Slime dies turn 5.
///
/// Expected outcome: Player wins with 9 HP (1 extra damage from Chill on Turn 1).
#[test]
fn test_work_vest_atk_oil_vs_spore_slime_t1() {
    // First verify the stats are calculated correctly
    let mut inventory = make_inventory();
    inventory.tool = Some(basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]));
    inventory.gear[0] = Some(work_vest());

    let player_stats = calculate_stats(&inventory);
    assert_eq!(
        player_stats.max_hp, 19,
        "max_hp should be 15 base + 4 Work Vest"
    );
    assert_eq!(player_stats.strikes, 1, "strikes should be 1");

    // Now run the combat
    let outcome = run_combat(
        19, // Player at full HP (19/19)
        basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        vec![work_vest()],
        ids::SPORE_SLIME,
        0, // T1
        0, // No gold
    );

    // Print debug info
    println!("=== Work Vest + ATK Oil vs Spore Slime T1 ===");
    println!("Player won: {}", outcome.player_won);
    println!("Final player HP: {}", outcome.final_player_hp);
    println!("Final enemy HP: {}", outcome.final_enemy_hp);
    println!("Turns taken: {}", outcome.turns_taken);
    println!("\nCombat log:");
    for entry in &outcome.log {
        println!(
            "  Turn {}: {} {:?} value={} extra={}",
            entry.turn,
            if entry.is_player { "PLAYER" } else { "ENEMY " },
            entry.action,
            entry.value,
            entry.extra
        );
    }

    // Verify outcome
    assert!(outcome.player_won, "Player should win");
    assert_eq!(
        outcome.final_player_hp, 14,
        "Player should end with 14 HP (Chill +1 damage on Turn 1)"
    );
    assert!(outcome.final_enemy_hp <= 0, "Enemy should be dead");
    assert_eq!(outcome.turns_taken, 5, "Combat should take 5 turns");
}

/// Test that Work Vest MaxHp is pre-calculated and not double-applied
#[test]
fn test_work_vest_max_hp_not_double_counted() {
    let mut inventory = make_inventory();
    inventory.gear[0] = Some(work_vest());

    let player_stats = calculate_stats(&inventory);
    let player_effects = generate_combat_effects(&inventory);
    let player_input = build_player_combatant(19, &player_stats, &player_effects);

    // MaxHp should be 19 (15 base + 4 Work Vest)
    assert_eq!(player_stats.max_hp, 19);
    assert_eq!(player_input.max_hp, 19);
    // HP should be 19 (we passed 19)
    assert_eq!(player_input.hp, 19);
    // ARM should be 0 at input time (Work Vest +1 ARM is applied during combat BattleStart)
    assert_eq!(player_input.arm, 0);
}

/// Test that Work Vest ARM is applied during combat
#[test]
fn test_work_vest_arm_applied_during_combat() {
    let outcome = run_combat(
        14,
        basic_pickaxe_with_oils(&[]),
        vec![work_vest()],
        ids::TUNNEL_RAT, // Tunnel Rat has 0 ARM, so we can see the damage clearly
        0,
        0,
    );

    // Work Vest should give +1 ARM at BattleStart (turn 1, since combat starts at turn 1)
    assert_log_contains(&outcome.log, "work_vest_arm_applied", |entry| {
        entry.action == LogAction::ArmorChange
            && entry.is_player
            && entry.turn == 1
            && entry.value == 1
    });
}
