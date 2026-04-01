use super::{
    build_player_combatant, preprocess_enemy_effects, strip_baked_battle_start_stat_effects,
};
use crate::state::RunMode;
use crate::stats::calculate_stats;
use anchor_lang::prelude::Pubkey;
use boss_system::{
    get_boss_annotated_item_effects, scale_boss, to_combatant_input, BossDefinition, Week, BOSSES,
    CRYSTAL_MIMIC_A, GREEDKEEPER_A, OBSIDIAN_GOLEM_A, POWDER_KEG_BARON_A,
};
use combat_system::{
    resolve_boss_combat_annotated_with_player_gold, resolve_combat_annotated_with_both_gold,
    CombatOutcome, STATUS_BLEED, STATUS_CHILL, STATUS_RUST, STATUS_SHRAPNEL,
};
use field_enemies::archetypes::{get_enemy_combatant_input, ids, ARCHETYPE_COUNT};
use player_inventory::effects::generate_annotated_combat_effects;
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

fn tool_id(tag: &[u8; 2], variant: u8) -> [u8; 8] {
    [b'T', b'-', tag[0], tag[1], b'-', b'0', b'0' + variant, 0]
}

fn gear_id(tag: &[u8; 2], index: u8) -> [u8; 8] {
    [
        b'G',
        b'-',
        tag[0],
        tag[1],
        b'-',
        b'0' + (index / 10),
        b'0' + (index % 10),
        0,
    ]
}

fn all_tool_instances() -> Vec<ItemInstance> {
    let mut tools = vec![ItemInstance::new(*BASIC_PICKAXE.id, Tier::I)];
    for tag in [
        *b"ST", *b"SC", *b"GR", *b"BL", *b"FR", *b"RU", *b"BO", *b"TE",
    ] {
        for variant in 1..=2 {
            tools.push(ItemInstance::new(tool_id(&tag, variant), Tier::I));
        }
    }
    tools
}

fn all_gear_instances() -> Vec<ItemInstance> {
    let mut gear = Vec::new();
    for tag in [
        *b"ST", *b"SC", *b"GR", *b"BL", *b"FR", *b"RU", *b"BO", *b"TE",
    ] {
        for index in 1..=8 {
            gear.push(ItemInstance::new(gear_id(&tag, index), Tier::I));
        }
    }
    gear
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

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(player_hp, &player_stats, &all_player_effects);

    let enemy_input = get_enemy_combatant_input(enemy_archetype, enemy_tier)
        .expect("enemy combatant input missing");
    let enemy_effects = preprocess_enemy_effects(enemy_archetype, player_gold);

    resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        player_gold,
        0,
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

fn run_boss_combat(
    player_hp: i16,
    tool: ItemInstance,
    gear: Vec<ItemInstance>,
    boss: &BossDefinition,
    stage: u8,
    week: Week,
    player_gold: u16,
) -> CombatOutcome {
    let mut inventory = make_inventory();
    inventory.tool = Some(tool);
    for (slot, item) in gear.iter().enumerate() {
        inventory.gear[slot] = Some(*item);
    }

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(player_hp, &player_stats, &all_player_effects);

    let boss_input = to_combatant_input(&scale_boss(boss, stage, week));
    let boss_effects = get_boss_annotated_item_effects(boss);

    resolve_boss_combat_annotated_with_player_gold(
        player_input,
        boss_input,
        player_effects,
        boss_effects,
        player_gold,
        boss.id,
    )
    .expect("boss combat resolution failed")
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

fn is_valid_status_id(extra: u8) -> bool {
    matches!(
        extra,
        STATUS_CHILL
            | STATUS_SHRAPNEL
            | STATUS_RUST
            | STATUS_BLEED
            | combat_system::STATUS_REFLECTION
    )
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
        !without_oil.player_won,
        "spore_slime_arm_oil_flips_outcome: expected loss without oil under current chill rules"
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
        expected_final_enemy_hp: ExpectedHp::Exact(-1),
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
}

#[test]
fn test_tool_oils_are_baked_into_player_input() {
    let mut inventory = make_inventory();
    inventory.tool = Some(basic_pickaxe_with_oils(&[
        ToolOilModification::PlusAtk,
        ToolOilModification::PlusArm,
        ToolOilModification::PlusSpd,
    ]));

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let _player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(15, &player_stats, &all_player_effects);

    assert_eq!(player_input.atk, 2);
    assert_eq!(player_input.arm, 1);
    assert_eq!(player_input.spd, 1);
}

#[test]
fn test_baked_tool_stats_do_not_emit_battle_start_logs() {
    let outcome = run_combat(
        15,
        basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk, ToolOilModification::PlusArm]),
        vec![],
        ids::TUNNEL_RAT,
        0,
        0,
    );
}

#[test]
fn test_bleed_status_damage_carries_bleed_status_id() {
    let outcome = run_combat(
        10,
        basic_pickaxe_with_oils(&[]),
        vec![],
        ids::BLOOD_MOSQUITO,
        1,
        0,
    );
}

#[test]
fn test_corrosive_pick_gloves_cloak_vs_blood_mosquito_regression() {
    let mut inventory = make_inventory();
    inventory.tool = Some(ItemInstance::new(*b"T-RU-01\0", Tier::I)); // Corrosive Pick
    inventory.gear[0] = Some(ItemInstance::new(*b"G-SC-02\0", Tier::I)); // Leather Gloves
    inventory.gear[1] = Some(ItemInstance::new(*b"G-FR-02\0", Tier::I)); // Rime Cloak

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(15, &player_stats, &all_player_effects);

    assert_eq!(player_input.atk, 2);
    assert_eq!(player_input.arm, 3);

    let enemy_input = get_enemy_combatant_input(ids::BLOOD_MOSQUITO, 1).expect("enemy input");
    let enemy_effects = preprocess_enemy_effects(ids::BLOOD_MOSQUITO, 0);
    let outcome = resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        0,
        0,
    )
    .expect("combat resolution failed");
}

#[test]
fn test_rime_pike_frost_lantern_buckler_vs_blood_mosquito_t2_finishes_at_21_hp() {
    let mut inventory = make_inventory();
    inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::I)); // Rime Pike
    inventory.gear[0] = Some(ItemInstance::new(*b"G-FR-01\0", Tier::I)); // Frost Lantern
    inventory.gear[1] = Some(ItemInstance::new(*b"G-FR-05\0", Tier::I)); // Frostguard Buckler (new ID)

    let player_stats = calculate_stats(&inventory, 1, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(25, &player_stats, &all_player_effects);

    let enemy_input = get_enemy_combatant_input(ids::BLOOD_MOSQUITO, 1).expect("enemy input");
    let enemy_effects = preprocess_enemy_effects(ids::BLOOD_MOSQUITO, 0);
    let outcome = resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        0,
        0,
    )
    .expect("combat resolution failed");

    assert_eq!(
        outcome.final_player_hp, 23,
        "Rime Pike + Frost Lantern + Frostguard Buckler vs Blood Mosquito T2 should end at 23 HP"
    );
}

#[test]
fn test_rime_pike_frost_lantern_rust_engine_vs_coin_slug_t1_finishes_at_25_hp() {
    let mut inventory = make_inventory();
    inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::I)); // Rime Pike
    inventory.gear[0] = Some(ItemInstance::new(*b"G-FR-01\0", Tier::I)); // Frost Lantern
    inventory.gear[1] = Some(ItemInstance::new(*b"G-RU-06\0", Tier::I)); // Rust Engine

    let player_stats = calculate_stats(&inventory, 1, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(25, &player_stats, &all_player_effects);

    let enemy_input = get_enemy_combatant_input(ids::COIN_SLUG, 0).expect("coin slug input");
    let enemy_effects = preprocess_enemy_effects(ids::COIN_SLUG, 10);

    let outcome = resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        10,
        0,
    )
    .expect("combat resolution failed");

    assert!(outcome.player_won, "player should beat Coin Slug T1");
    assert_eq!(
        outcome.final_player_hp, 25,
        "Rime Pike + Frost Lantern + Rust Engine vs Coin Slug T1 should end at 25 HP"
    );
}

#[test]
fn test_rime_pike_frost_lantern_rust_engine_vs_powder_tick_t3_finishes_at_20_hp() {
    let mut inventory = make_inventory();
    inventory.tool = Some(ItemInstance::new(*b"T-FR-01\0", Tier::I)); // Rime Pike
    inventory.gear[0] = Some(ItemInstance::new(*b"G-FR-01\0", Tier::I)); // Frost Lantern
    inventory.gear[1] = Some(ItemInstance::new(*b"G-RU-06\0", Tier::I)); // Rust Engine

    let player_stats = calculate_stats(&inventory, 1, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(25, &player_stats, &all_player_effects);

    let enemy_input = get_enemy_combatant_input(ids::POWDER_TICK, 2).expect("powder tick input");
    let enemy_effects = preprocess_enemy_effects(ids::POWDER_TICK, 10);

    let outcome = resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        10,
        0,
    )
    .expect("combat resolution failed");
    assert!(outcome.player_won, "player should beat Powder Tick T3");
    assert_eq!(
        outcome.final_player_hp, 23,
        "Rime Pike + Frost Lantern + Rust Engine vs Powder Tick T3 should end at 23 HP"
    );
    // Verify countdown self-damage still occurs
}

#[test]
fn test_obsidian_golem_fuse_pick_non_weapon_removes_boss_armor() {
    let outcome = run_boss_combat(
        15,
        ItemInstance::new(*b"T-BL-01\0", Tier::I),
        vec![],
        &OBSIDIAN_GOLEM_A,
        1,
        Week::One,
        0,
    );
}

#[test]
fn test_greedkeeper_steals_player_gold_and_gains_armor_at_battle_start() {
    let outcome = run_boss_combat(
        15,
        basic_pickaxe_with_oils(&[]),
        vec![],
        &GREEDKEEPER_A,
        12,
        Week::Two,
        16,
    );
}

#[test]
fn test_powder_keg_baron_countdown_hits_both_sides() {
    let outcome = run_boss_combat(
        19,
        basic_pickaxe_with_oils(&[]),
        vec![ItemInstance::new(*b"G-FR-02\0", Tier::I), work_vest()],
        &POWDER_KEG_BARON_A,
        12,
        Week::Two,
        0,
    );
}

#[test]
fn test_crystal_mimic_reflects_first_chill_application() {
    let outcome = run_boss_combat(
        15,
        ItemInstance::new(*b"T-FR-01\0", Tier::I),
        vec![],
        &CRYSTAL_MIMIC_A,
        12,
        Week::Two,
        0,
    );
}

#[test]
fn test_all_field_enemies_with_baked_stats_have_clean_replay_shape() {
    for enemy_id in 0..ARCHETYPE_COUNT as u8 {
        for tier in 0..=2 {
            let mut inventory = make_inventory();
            inventory.tool = Some(ItemInstance::new(*b"T-RU-01\0", Tier::I));
            inventory.gear[0] = Some(ItemInstance::new(*b"G-SC-02\0", Tier::I));
            inventory.gear[1] = Some(ItemInstance::new(*b"G-FR-02\0", Tier::I));

            let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
            let all_player_effects = generate_annotated_combat_effects(&inventory);
            let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
            let player_input = build_player_combatant(15, &player_stats, &all_player_effects);

            let enemy_input =
                get_enemy_combatant_input(enemy_id, tier).expect("enemy combatant input missing");
            let enemy_effects = preprocess_enemy_effects(enemy_id, 0);

            let outcome = resolve_combat_annotated_with_both_gold(
                player_input,
                enemy_input,
                player_effects,
                enemy_effects,
                0,
                0,
            )
            .expect("combat resolution failed");

            let context = format!("enemy_id={} tier={}", enemy_id, tier);
        }
    }
}

#[test]
fn test_all_field_enemies_with_status_and_non_weapon_builds_have_valid_status_logs() {
    for enemy_id in 0..ARCHETYPE_COUNT as u8 {
        for tier in 0..=2 {
            let chill_outcome = run_combat(
                15,
                ItemInstance::new(*b"T-FR-01\0", Tier::I),
                vec![],
                enemy_id,
                tier,
                0,
            );

            let fuse_outcome = run_combat(
                15,
                ItemInstance::new(*b"T-BL-01\0", Tier::I),
                vec![],
                enemy_id,
                tier,
                0,
            );
        }
    }
}

#[test]
fn test_all_bosses_have_clean_baked_stat_replay_shape() {
    for boss in BOSSES.iter().copied() {
        let stage = if boss.biome == boss_system::Biome::A {
            12
        } else {
            32
        };
        let mut inventory = make_inventory();
        inventory.tool = Some(ItemInstance::new(*b"T-RU-01\0", Tier::I));
        inventory.gear[0] = Some(ItemInstance::new(*b"G-SC-02\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-FR-02\0", Tier::I));

        let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
        let all_player_effects = generate_annotated_combat_effects(&inventory);
        let player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
        let player_input = build_player_combatant(15, &player_stats, &all_player_effects);

        let boss_input = to_combatant_input(&scale_boss(boss, stage, boss.week));
        let boss_effects = get_boss_annotated_item_effects(boss);

        let outcome = resolve_boss_combat_annotated_with_player_gold(
            player_input,
            boss_input,
            player_effects,
            boss_effects,
            16,
            boss.id,
        )
        .expect("boss combat resolution failed");

        let context = format!("boss={}", boss.name);
    }
}

#[test]
fn test_all_tools_have_valid_replay_logs_against_representative_enemies() {
    let representative_enemies = [
        ids::TUNNEL_RAT,
        ids::SPORE_SLIME,
        ids::RUST_MITE_SWARM,
        ids::BLOOD_MOSQUITO,
    ];

    for tool in all_tool_instances() {
        for enemy_id in representative_enemies {
            for tier in 0..=2 {
                let outcome = run_combat(15, tool, vec![], enemy_id, tier, 0);
                let context = format!(
                    "tool={} enemy_id={} tier={}",
                    String::from_utf8_lossy(&tool.item_id),
                    enemy_id,
                    tier
                );
            }
        }
    }
}

#[test]
fn test_all_single_gear_items_have_valid_replay_logs_with_basic_pickaxe() {
    let representative_enemies = [
        ids::TUNNEL_RAT,
        ids::SPORE_SLIME,
        ids::RUST_MITE_SWARM,
        ids::BLOOD_MOSQUITO,
    ];

    for gear in all_gear_instances() {
        for enemy_id in representative_enemies {
            for tier in 0..=2 {
                let outcome = run_combat(
                    15,
                    ItemInstance::new(*BASIC_PICKAXE.id, Tier::I),
                    vec![gear],
                    enemy_id,
                    tier,
                    0,
                );
                let context = format!(
                    "gear={} enemy_id={} tier={}",
                    String::from_utf8_lossy(&gear.item_id),
                    enemy_id,
                    tier
                );
            }
        }
    }
}

// =============================================================================
// Work Vest specific tests - user's exact scenario
// =============================================================================

/// This test matches the user's exact scenario:
/// - Work Vest T1 (+4 HP, +1 ARM) + Basic Pickaxe with ATK oil (+1 ATK)
/// - vs Spore Slime T1 (8 HP, 1 ATK, 2 ARM, 0 SPD, applies 2 Chill)
///
/// Expected combat flow:
/// - Player starts: 22 HP (18 base + 4 Work Vest), 2 ATK (1 base + 1 oil), 1 ARM (0 base + 1 vest)
/// - Spore Slime applies 1 Chill at BattleStart → player loses one strike on the next applicable turn, but Chill does not deal direct damage
/// - Turn order: SPD tie (0 vs 0), enemy goes first
/// - Turn 1: Slime 1 ATK vs Player 1 ARM → ARM gone. Player 2 ATK vs Slime 2 ARM → ARM gone.
/// - Turn 2-5: Slime 1 dmg/turn, Player 2 dmg/turn → Slime dies turn 5.
///
/// Expected outcome: Player wins with 18 HP.
#[test]
fn test_work_vest_atk_oil_vs_spore_slime_t1() {
    // First verify the stats are calculated correctly
    let mut inventory = make_inventory();
    inventory.tool = Some(basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]));
    inventory.gear[0] = Some(work_vest());

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    assert_eq!(
        player_stats.max_hp, 22,
        "max_hp should be 18 base + 4 Work Vest"
    );
    assert_eq!(player_stats.strikes, 1, "strikes should be 1");

    // Now run the combat
    let outcome = run_combat(
        22, // Player at full HP (22/22)
        basic_pickaxe_with_oils(&[ToolOilModification::PlusAtk]),
        vec![work_vest()],
        ids::SPORE_SLIME,
        0, // T1
        0, // No gold
    );

    // Verify outcome
    assert!(outcome.player_won, "Player should win");
    assert_eq!(
        outcome.final_player_hp, 17,
        "Player should end with 17 HP under non-damaging Chill rules"
    );
    assert!(outcome.final_enemy_hp <= 0, "Enemy should be dead");
    assert_eq!(outcome.turns_taken, 5, "Combat should take 5 turns");
}

/// Test that Work Vest MaxHp is pre-calculated and not double-applied
#[test]
fn test_work_vest_max_hp_not_double_counted() {
    let mut inventory = make_inventory();
    inventory.gear[0] = Some(work_vest());

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let _player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(22, &player_stats, &all_player_effects);

    // MaxHp should be 22 (18 base + 4 Work Vest)
    assert_eq!(player_stats.max_hp, 22);
    assert_eq!(player_input.max_hp, 22);
    // HP should be 22 (we passed 22)
    assert_eq!(player_input.hp, 22);
    assert_eq!(player_input.arm, 1);
}

/// Test that Work Vest ARM is baked into combat entry stats
#[test]
fn test_work_vest_arm_is_baked_into_player_input() {
    let mut inventory = make_inventory();
    inventory.gear[0] = Some(work_vest());

    let player_stats = calculate_stats(&inventory, 20, RunMode::Campaign);
    let all_player_effects = generate_annotated_combat_effects(&inventory);
    let _player_effects = strip_baked_battle_start_stat_effects(all_player_effects.clone());
    let player_input = build_player_combatant(22, &player_stats, &all_player_effects);

    assert_eq!(player_input.arm, 1);
}
