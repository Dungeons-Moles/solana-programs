use crate::state::{
    AnnotatedItemEffect, CombatContribution, CombatLogEntry, CombatSourceKind, CombatSourceRef,
    Condition, EffectType, ItemEffect, StatusEffects, StatusType, TriggerType, STATUS_BLEED,
    STATUS_CHILL, STATUS_REFLECTION, STATUS_RUST, STATUS_SHRAPNEL,
};

#[derive(Clone, Debug, Default)]
pub struct CombatantStats {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub dig: i16,
    pub armor_piercing: i16,
    pub stored_damage: i16,
    pub gear_atk_bonus: i16,
    pub half_gear_atk_after_second_strike: bool,
    pub next_bomb_damage_bonus: i16,
    pub next_bomb_self_damage_reduction: i16,
    pub active_bomb_self_damage_reduction: i16,
    pub non_weapon_damage_bonus: i16,
    pub next_non_weapon_damage_bonus: i16,
    pub gold_gain_bonus: i16,
    pub non_weapon_hits_this_turn: u8,
    pub double_detonation_first: i16,
    pub double_detonation_second: i16,
    pub preserve_shrapnel_cap: u8,
    pub shards_every_turn: bool,
    pub attack_source: Option<CombatSourceRef>,
    pub attack_base_value: i16,
    pub atk_contributions: Vec<CombatContribution>,
}

/// Check if a trigger should fire.
///
/// `acts_first` is needed for `FirstTurnIfFaster` and `FirstTurnIfSlower` triggers.
/// For enemies, `acts_first` means enemy SPD >= player SPD (tie goes to enemy).
/// For players, `acts_first` means player SPD > enemy SPD.
#[allow(clippy::manual_is_multiple_of)]
pub fn should_trigger(
    trigger_type: TriggerType,
    turn: u8,
    is_first_turn: bool,
    acts_first: bool,
) -> bool {
    match trigger_type {
        TriggerType::BattleStart => turn == 1,
        TriggerType::FirstTurn => is_first_turn,
        TriggerType::FirstTurnIfFaster => is_first_turn && acts_first,
        TriggerType::FirstTurnIfSlower => is_first_turn && !acts_first,
        TriggerType::TurnStart => true,
        TriggerType::EveryOtherTurn => turn % 2 == 0,
        TriggerType::OnHit => true,
        TriggerType::Exposed | TriggerType::Wounded => true,
        // Countdown fires every N turns (turn 2, 4, 6... for turns=2)
        TriggerType::Countdown { turns } => turns > 0 && turn > 0 && turn % turns == 0,
        // Victory is processed outside combat system (in gameplay-state after combat ends)
        TriggerType::Victory => false,
        // OnStruck is handled specially in the combat engine when damage is dealt
        TriggerType::OnStruck => true,
        // TurnN fires only on the specified turn
        TriggerType::TurnN { turn: target_turn } => turn == target_turn,
        // EveryOtherTurnFirstHit fires on even turns, but once_per_turn flag handles the "first hit" part
        TriggerType::EveryOtherTurnFirstHit => turn % 2 == 0,
        // TurnEnd fires at the end of each turn
        TriggerType::TurnEnd => true,
        // OnEnemyBleedDamage is handled specially when bleed damage is processed
        TriggerType::OnEnemyBleedDamage => true,
        // OnApplyRust is handled specially when rust is applied
        TriggerType::OnApplyRust => true,
        // OnDealNonWeaponDamage is handled specially when non-weapon damage is dealt
        TriggerType::OnDealNonWeaponDamage => true,
        // OnGainShrapnel is handled specially when shrapnel is gained
        TriggerType::OnGainShrapnel => true,
        // OnGoldArmorConverted is handled specially after conversion succeeds
        TriggerType::OnGoldArmorConverted => true,
        // DayStart is processed outside combat system
        TriggerType::DayStart => false,
        // FirstTimeWounded is invoked by the combat loop only once when HP first
        // drops below 50%. The "first time" check is handled by CombatState flags.
        TriggerType::FirstTimeWounded => true,
        // FirstTimeExposed is invoked by the combat loop only once when ARM first
        // drops to zero or below. The "first time" check is handled by CombatState flags.
        TriggerType::FirstTimeExposed => true,
        // FirstTimeGainShrapnel is invoked by the combat loop only once when
        // shrapnel is first gained. The "first time" check is handled by CombatState flags.
        TriggerType::FirstTimeGainShrapnel => true,
    }
}

/// Check if a condition is satisfied
pub fn check_condition(
    condition: Condition,
    owner_stats: &CombatantStats,
    owner_status: &StatusEffects,
    opponent_stats: &CombatantStats,
    opponent_status: &StatusEffects,
    owner_gold: u16,
    opponent_gold: u16,
) -> bool {
    match condition {
        Condition::None => true,
        Condition::EnemyHasStatus(status_type) => status_stacks(opponent_status, status_type) > 0,
        Condition::EnemyHasArmor => opponent_stats.arm > 0,
        Condition::EnemyHasNoArmor => opponent_stats.arm <= 0,
        Condition::DigGreaterThanEnemyDig => owner_stats.dig > opponent_stats.dig,
        Condition::SpdGreaterThanEnemySpd => owner_stats.spd > opponent_stats.spd,
        Condition::OwnerWounded => check_wounded(owner_stats.hp, owner_stats.max_hp),
        Condition::OwnerExposed => check_exposed(owner_stats.arm),
        Condition::EnemyWounded => check_wounded(opponent_stats.hp, opponent_stats.max_hp),
        Condition::OwnerHasArmor => owner_stats.arm > 0,
        Condition::OwnerArmorAtLeast(value) => owner_stats.arm >= i16::from(value),
        Condition::OwnerHasStatus(status_type) => status_stacks(owner_status, status_type) > 0,
        Condition::OwnerDigGreaterThanEnemyArmor => owner_stats.dig > opponent_stats.arm,
        Condition::EnemyHasStatusAtLeast(status_type, min_stacks) => {
            status_stacks(opponent_status, status_type) >= min_stacks
        }
        Condition::EnemyHasNoArmorAndStatusAtLeast(status_type, min_stacks) => {
            if opponent_stats.arm > 0 {
                return false;
            }
            status_stacks(opponent_status, status_type) >= min_stacks
        }
        Condition::EnemyHasStatusOrNoArmor(status_type) => {
            status_stacks(opponent_status, status_type) > 0 || opponent_stats.arm <= 0
        }
        Condition::OwnerGoldAtLeast(min_gold) => owner_gold >= min_gold,
        Condition::EnemyGoldAtLeast(min_gold) => opponent_gold >= min_gold,
    }
}

pub fn check_exposed(arm: i16) -> bool {
    arm <= 0
}

pub fn check_wounded(hp: i16, max_hp: u16) -> bool {
    let hp_value = i32::from(hp);
    let max_value = i32::from(max_hp);
    hp_value * 2 < max_value
}

#[inline]
fn status_stacks(status: &StatusEffects, status_type: StatusType) -> u8 {
    match status_type {
        StatusType::Chill => status.chill,
        StatusType::Shrapnel => status.shrapnel,
        StatusType::Rust => status.rust,
        StatusType::Bleed => status.bleed,
        StatusType::Reflection => status.reflection,
    }
}

#[inline]
fn apply_gold_gain_bonus(value: i16, bonus_percent: i16) -> i16 {
    let base = value.max(0);
    if base == 0 || bonus_percent <= 0 {
        return base;
    }

    let total = i32::from(base) + (i32::from(base) * i32::from(bonus_percent) / 100);
    i16::try_from(total).unwrap_or(i16::MAX)
}

#[inline]
fn increase_gold(player_gold: &mut u16, gold_change: Option<&mut i16>, value: i16) {
    let gain_u16 = u16::try_from(value).unwrap_or(u16::MAX);
    *player_gold = player_gold.saturating_add(gain_u16);
    if let Some(gold_change_ref) = gold_change {
        let gain_i16 = i16::try_from(gain_u16).unwrap_or(i16::MAX);
        *gold_change_ref = gold_change_ref.saturating_add(gain_i16);
    }
}

#[inline]
fn add_attack_contribution(
    stats: &mut CombatantStats,
    source: Option<&CombatSourceRef>,
    value: i16,
) {
    if value <= 0 {
        return;
    }

    let Some(source) = source else {
        return;
    };

    if let Some(existing) = stats
        .atk_contributions
        .iter_mut()
        .find(|entry| entry.source == *source)
    {
        existing.value = existing.value.saturating_add(value);
        return;
    }

    stats.atk_contributions.push(CombatContribution {
        source: *source,
        value,
    });
}

#[inline]
fn reflection_source() -> CombatSourceRef {
    let mut id = [0u8; 16];
    id[..10].copy_from_slice(b"reflection");
    CombatSourceRef {
        kind: CombatSourceKind::Status,
        id,
    }
}

#[inline]
fn push_log_entry(
    log: &mut Vec<CombatLogEntry>,
    entry: CombatLogEntry,
    source: Option<&CombatSourceRef>,
) {
    if let Some(source) = source {
        log.push(entry.with_source(*source));
    } else {
        log.push(entry);
    }
}

#[inline]
fn apply_status_effect(
    status_field: &mut u8,
    value: i16,
    turn: u8,
    is_target_player: bool,
    status_id: u8,
    source: Option<&CombatSourceRef>,
    log: &mut Vec<CombatLogEntry>,
) {
    let add = u8::try_from(value).unwrap_or(u8::MAX);
    *status_field = status_field.saturating_add(add);
    if value > 0 {
        let mut entry = CombatLogEntry::apply_status(turn, is_target_player, status_id, value);
        if let Some(source) = source {
            entry = entry.with_source(*source);
        }
        log.push(entry);
    }
}

/// Applies an effect and logs it.
/// `is_target_player` indicates whether the effect target is the player (for logging purposes).
/// `gold_change` tracks net gold changes during combat for the first combatant
/// (positive = first combatant gains).
#[allow(clippy::too_many_arguments)]
pub fn apply_effect(
    phase: TriggerType,
    effect_type: EffectType,
    value: i16,
    stats: &mut CombatantStats,
    status: &mut StatusEffects,
    turn: u8,
    is_target_player: bool,
    player_gold: &mut u16,
    enemy_gold: &mut u16,
    gold_change: &mut i16,
    source: Option<&CombatSourceRef>,
    log: &mut Vec<CombatLogEntry>,
) {
    let value = value.max(0);

    match effect_type {
        EffectType::DealDamage => {
            // ARM is "HP before HP": deplete ARM first, overflow to HP
            let arm_damage = value.min(stats.arm.max(0));
            stats.arm = stats.arm.saturating_sub(arm_damage);
            let hp_damage = value.saturating_sub(arm_damage);
            stats.hp = stats.hp.checked_sub(hp_damage).unwrap_or(i16::MIN);
            if arm_damage > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::armor_change(turn, is_target_player, -arm_damage),
                    source,
                );
            }
            if hp_damage > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::attack(turn, !is_target_player, hp_damage),
                    source,
                );
            }
        }
        EffectType::DealNonWeaponDamage => {
            stats.hp = stats.hp.checked_sub(value).unwrap_or(i16::MIN);
            if value > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::non_weapon_damage(turn, is_target_player, value),
                    source,
                );
            }
        }
        EffectType::Heal => {
            let max_hp = i16::try_from(stats.max_hp).unwrap_or(i16::MAX);
            let old_hp = stats.hp;
            let healed = stats.hp.checked_add(value).unwrap_or(i16::MAX);
            stats.hp = healed.min(max_hp);
            let actual_heal = stats.hp - old_hp;
            if actual_heal > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::heal(turn, is_target_player, actual_heal),
                    source,
                );
            }
        }
        EffectType::GainArmor => {
            stats.arm = stats.arm.checked_add(value).unwrap_or(i16::MAX);
            if value > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::armor_change(turn, is_target_player, value),
                    source,
                );
            }
        }
        EffectType::GainAtk => {
            stats.atk = stats.atk.checked_add(value).unwrap_or(i16::MAX);
            add_attack_contribution(stats, source, value);
            if value > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::atk_change(turn, is_target_player, value),
                    source,
                );
            }
        }
        EffectType::GainGearAtk => {
            stats.atk = stats.atk.checked_add(value).unwrap_or(i16::MAX);
            stats.gear_atk_bonus = stats.gear_atk_bonus.saturating_add(value);
            add_attack_contribution(stats, source, value);
            if value > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::atk_change(turn, is_target_player, value),
                    source,
                );
            }
        }
        EffectType::GainSpd => {
            stats.spd = stats.spd.checked_add(value).unwrap_or(i16::MAX);
            if value > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::spd_change(turn, is_target_player, value),
                    source,
                );
            }
        }
        EffectType::ApplyChill => {
            apply_status_effect(
                &mut status.chill,
                value,
                turn,
                is_target_player,
                STATUS_CHILL,
                source,
                log,
            );
        }
        EffectType::ApplyShrapnel => {
            apply_status_effect(
                &mut status.shrapnel,
                value,
                turn,
                is_target_player,
                STATUS_SHRAPNEL,
                source,
                log,
            );
        }
        EffectType::ApplyRust => {
            apply_status_effect(
                &mut status.rust,
                value,
                turn,
                is_target_player,
                STATUS_RUST,
                source,
                log,
            );
        }
        EffectType::ApplyBleed => {
            apply_status_effect(
                &mut status.bleed,
                value,
                turn,
                is_target_player,
                STATUS_BLEED,
                source,
                log,
            );
        }
        EffectType::RemoveArmor => {
            let old_arm = stats.arm;
            let reduced = stats.arm.checked_sub(value).unwrap_or(i16::MIN);
            stats.arm = reduced.max(0);
            let actual_reduction = old_arm - stats.arm;
            if actual_reduction > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::armor_change(turn, is_target_player, -actual_reduction),
                    source,
                );
            }
        }
        EffectType::RemoveOwnArmor => {
            let old_arm = stats.arm;
            let reduced = stats.arm.checked_sub(value).unwrap_or(i16::MIN);
            stats.arm = reduced.max(0);
            let actual_reduction = old_arm - stats.arm;
            if actual_reduction > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::armor_change(turn, is_target_player, -actual_reduction),
                    source,
                );
            }
        }
        EffectType::StealGold => {
            if is_target_player {
                let stolen = value.min(i16::try_from(*player_gold).unwrap_or(i16::MAX));
                if stolen > 0 {
                    *player_gold = player_gold.saturating_sub(stolen as u16);
                    *enemy_gold = enemy_gold.saturating_add(stolen as u16);
                    *gold_change = gold_change.saturating_sub(stolen);
                    push_log_entry(log, CombatLogEntry::gold_stolen(turn, false, -stolen), source);
                }
            } else {
                let stolen = value.min(i16::try_from(*enemy_gold).unwrap_or(i16::MAX));
                if stolen > 0 {
                    *enemy_gold = enemy_gold.saturating_sub(stolen as u16);
                    *player_gold = player_gold.saturating_add(stolen as u16);
                    *gold_change = gold_change.saturating_add(stolen);
                    push_log_entry(log, CombatLogEntry::gold_stolen(turn, true, stolen), source);
                }
            }
        }
        EffectType::GoldToArmor => {
            let ratio = value.max(1);
            let ratio_u16 = u16::try_from(ratio).unwrap_or(1);
            let available_gold = if is_target_player {
                *player_gold
            } else {
                *enemy_gold
            };
            let gained_armor = i16::try_from(available_gold / ratio_u16).unwrap_or(i16::MAX);
            if gained_armor > 0 {
                stats.arm = stats.arm.saturating_add(gained_armor);
                push_log_entry(
                    log,
                    CombatLogEntry::armor_change(turn, is_target_player, gained_armor),
                    source,
                );
            }
        }
        EffectType::ApplyReflection => {
            apply_status_effect(
                &mut status.reflection,
                value,
                turn,
                is_target_player,
                STATUS_REFLECTION,
                source,
                log,
            );
        }
        EffectType::ReduceEnemySpd => {
            let old_spd = stats.spd;
            stats.spd = stats.spd.saturating_sub(value);
            let actual_reduction = old_spd - stats.spd;
            if actual_reduction > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::spd_change(turn, is_target_player, -actual_reduction),
                    source,
                );
            }
        }
        EffectType::DealSelfNonWeaponDamage => {
            // Deal non-weapon damage to self (for bomb self-damage)
            let mut damage = value;

            if matches!(phase, TriggerType::Countdown { .. }) {
                let reduction = stats.active_bomb_self_damage_reduction.max(0);
                damage = damage.saturating_sub(reduction);
                stats.active_bomb_self_damage_reduction = 0;
            }

            stats.hp = stats.hp.checked_sub(damage).unwrap_or(i16::MIN);
            if damage > 0 {
                push_log_entry(
                    log,
                    CombatLogEntry::non_weapon_damage(turn, is_target_player, damage),
                    source,
                );
            }
        }
        EffectType::SetArmorPiercing => {
            stats.armor_piercing = stats.armor_piercing.max(value);
        }
        EffectType::ArmorToMaxHp => {
            // Convert half of current armor (rounded up) to max HP, capped by value.
            // Also heal by the granted amount so the new max is immediately usable.
            let armor = stats.arm.max(0);
            let converted = ((armor + 1) / 2).min(value.max(0));
            if converted > 0 {
                stats.max_hp =
                    u16::try_from((i32::from(stats.max_hp) + i32::from(converted)).max(0))
                        .unwrap_or(u16::MAX);
                let max_hp_i16 = i16::try_from(stats.max_hp).unwrap_or(i16::MAX);
                stats.hp = stats.hp.saturating_add(converted).min(max_hp_i16);
                push_log_entry(
                    log,
                    CombatLogEntry::heal(turn, is_target_player, converted),
                    source,
                );
            }
        }
        EffectType::StoreDamage => {
            stats.stored_damage = stats.stored_damage.saturating_add(value.max(0));
        }
        EffectType::AmplifyNonWeaponDamage => {
            stats.non_weapon_damage_bonus =
                stats.non_weapon_damage_bonus.saturating_add(value.max(0));
        }
        EffectType::EmpowerNextNonWeaponDamage => {
            stats.next_non_weapon_damage_bonus =
                stats.next_non_weapon_damage_bonus.max(value.max(0));
        }
        EffectType::EmpowerNextBombDamage => {
            stats.next_bomb_damage_bonus =
                stats.next_bomb_damage_bonus.saturating_add(value.max(0));
        }
        EffectType::ReduceNextBombSelfDamage => {
            // TurnStart-based mitigation should refresh to a floor each turn rather than
            // accumulating indefinitely if no bomb triggers.
            if matches!(phase, TriggerType::TurnStart) {
                stats.next_bomb_self_damage_reduction =
                    stats.next_bomb_self_damage_reduction.max(value.max(0));
            } else {
                stats.next_bomb_self_damage_reduction = stats
                    .next_bomb_self_damage_reduction
                    .saturating_add(value.max(0));
            }
        }
        EffectType::HalfGearAtkAfterSecondStrike => {
            stats.half_gear_atk_after_second_strike = true;
        }
        EffectType::ShardsEveryTurn => {
            stats.shards_every_turn = true;
        }
        EffectType::PreserveShrapnel => {
            let cap = u8::try_from(value.max(0)).unwrap_or(u8::MAX);
            stats.preserve_shrapnel_cap = stats.preserve_shrapnel_cap.max(cap);
        }
        EffectType::GainGold => {
            let gain = apply_gold_gain_bonus(value, stats.gold_gain_bonus);
            if is_target_player {
                increase_gold(player_gold, Some(gold_change), gain);
            } else {
                increase_gold(enemy_gold, None, gain);
            }
        }
        EffectType::ConsumeGoldForArmor => {
            if is_target_player && *player_gold > 0 {
                *player_gold = player_gold.saturating_sub(1);
                *gold_change = gold_change.saturating_sub(1);
                stats.arm = stats.arm.saturating_add(value.max(0));
                if value > 0 {
                    push_log_entry(
                        log,
                        CombatLogEntry::armor_change(turn, is_target_player, value),
                        source,
                    );
                }
            } else if !is_target_player && *enemy_gold > 0 {
                *enemy_gold = enemy_gold.saturating_sub(1);
                stats.arm = stats.arm.saturating_add(value.max(0));
                if value > 0 {
                    push_log_entry(
                        log,
                        CombatLogEntry::armor_change(turn, is_target_player, value),
                        source,
                    );
                }
            }
        }
        // These effects are processed outside the combat system or need special handling
        EffectType::GainStrikes
        | EffectType::GainDig
        | EffectType::ApplyBomb
        | EffectType::MaxHp
        | EffectType::GoldToArmorScaled
        | EffectType::PreventDeath
        | EffectType::ReduceAllCountdowns
        | EffectType::BlastImmunity
        | EffectType::DoubleBombTrigger
        | EffectType::DoubleOnHitEffects
        | EffectType::TriggerAllShards
        | EffectType::AmplifyGoldGain
        | EffectType::DoubleDetonationFirst
        | EffectType::DoubleDetonationSecond => {}
    }
}

/// Returns true if this effect applies status to opponent and has no condition.
/// These effects should be processed first so conditional effects can see the applied status.
fn is_unconditional_status_application(effect: &ItemEffect) -> bool {
    effect.condition == Condition::None
        && is_status_effect(effect.effect_type)
        && targets_opponent(effect.effect_type)
}

#[allow(clippy::too_many_arguments)]
pub fn process_triggers_for_phase(
    effects: &mut [AnnotatedItemEffect],
    phase: TriggerType,
    turn: u8,
    owner_stats: &mut CombatantStats,
    owner_status: &mut StatusEffects,
    opponent_stats: &mut CombatantStats,
    opponent_status: &mut StatusEffects,
    triggered_flags: &mut [bool],
    is_owner_player: bool,
    owner_acts_first: bool,
    player_gold: &mut u16,
    enemy_gold: &mut u16,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
) {
    if phase == TriggerType::TurnStart {
        owner_stats.non_weapon_hits_this_turn = 0;
    }

    // Two-pass processing to ensure status effects are applied before conditional effects
    // that check status are evaluated. This makes item synergies (e.g., Frost Lantern +
    // Frostguard Buckler) work regardless of inventory slot order.
    //
    // Pass 1: Unconditional status-applying effects (apply status to opponent)
    // Pass 2: All other effects (including conditional ones that may check status)

    process_effects_pass(
        effects,
        phase,
        turn,
        owner_stats,
        owner_status,
        opponent_stats,
        opponent_status,
        triggered_flags,
        is_owner_player,
        owner_acts_first,
        player_gold,
        enemy_gold,
        gold_change,
        log,
        true, // first pass: only unconditional status effects
    );

    process_effects_pass(
        effects,
        phase,
        turn,
        owner_stats,
        owner_status,
        opponent_stats,
        opponent_status,
        triggered_flags,
        is_owner_player,
        owner_acts_first,
        player_gold,
        enemy_gold,
        gold_change,
        log,
        false, // second pass: everything else
    );
}

#[allow(clippy::too_many_arguments)]
fn process_effects_pass(
    effects: &mut [AnnotatedItemEffect],
    phase: TriggerType,
    turn: u8,
    owner_stats: &mut CombatantStats,
    owner_status: &mut StatusEffects,
    opponent_stats: &mut CombatantStats,
    opponent_status: &mut StatusEffects,
    triggered_flags: &mut [bool],
    is_owner_player: bool,
    owner_acts_first: bool,
    player_gold: &mut u16,
    enemy_gold: &mut u16,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
    first_pass: bool,
) {
    let is_first_turn = turn == 1;
    let mut gold_armor_conversion_happened = false;
    let mut pending_countdown_reduction: u8 = 0;
    let mut dealt_non_weapon_damage = false;

    for (index, annotated) in effects.iter_mut().enumerate() {
        let effect = &mut annotated.effect;
        if effect.trigger != phase {
            continue;
        }

        // Determine if this effect should be processed in this pass
        let is_unconditional_status = is_unconditional_status_application(effect);
        if first_pass != is_unconditional_status {
            continue;
        }

        if effect.once_per_turn && triggered_flags.get(index).copied().unwrap_or(false) {
            continue;
        }

        let should_fire = match effect.trigger {
            TriggerType::Exposed => check_exposed(owner_stats.arm),
            TriggerType::Wounded => check_wounded(owner_stats.hp, owner_stats.max_hp),
            TriggerType::EveryOtherTurnFirstHit if owner_stats.shards_every_turn => true,
            _ => should_trigger(effect.trigger, turn, is_first_turn, owner_acts_first),
        };

        if !should_fire {
            continue;
        }

        // Check condition if one is specified
        let owner_gold_amount = if is_owner_player {
            *player_gold
        } else {
            *enemy_gold
        };
        let opponent_gold_amount = if is_owner_player {
            *enemy_gold
        } else {
            *player_gold
        };

        if !check_condition(
            effect.condition,
            owner_stats,
            owner_status,
            opponent_stats,
            opponent_status,
            owner_gold_amount,
            opponent_gold_amount,
        ) {
            continue;
        }

        match effect.effect_type {
            EffectType::AmplifyGoldGain => {
                owner_stats.gold_gain_bonus =
                    owner_stats.gold_gain_bonus.saturating_add(effect.value.max(0));
            }
            EffectType::DoubleDetonationFirst => {
                owner_stats.double_detonation_first = effect.value.max(0);
            }
            EffectType::DoubleDetonationSecond => {
                owner_stats.double_detonation_second = effect.value.max(0);
            }
            _ => {}
        }

        let mut effect_value = effect.value;
        if matches!(effect.effect_type, EffectType::DealNonWeaponDamage)
            && targets_opponent(effect.effect_type)
        {
            let mut extra = 0;
            match owner_stats.non_weapon_hits_this_turn {
                0 => extra += owner_stats.double_detonation_first.max(0),
                1 => extra += owner_stats.double_detonation_second.max(0),
                _ => {}
            }
            owner_stats.non_weapon_hits_this_turn =
                owner_stats.non_weapon_hits_this_turn.saturating_add(1);
            effect_value = effect_value
                .saturating_add(extra)
                .saturating_add(owner_stats.non_weapon_damage_bonus.max(0));
            if owner_stats.next_non_weapon_damage_bonus > 0 {
                effect_value =
                    effect_value.saturating_add(owner_stats.next_non_weapon_damage_bonus.max(0));
                owner_stats.next_non_weapon_damage_bonus = 0;
            }
        }

        if matches!(effect.effect_type, EffectType::ReduceAllCountdowns) {
            pending_countdown_reduction =
                pending_countdown_reduction.max(effect_value.max(0) as u8);
        }

        if targets_opponent(effect.effect_type)
            && matches!(phase, TriggerType::Countdown { .. })
            && matches!(effect.effect_type, EffectType::DealNonWeaponDamage)
        {
            if owner_stats.next_bomb_damage_bonus > 0 {
                effect_value = effect_value.saturating_add(owner_stats.next_bomb_damage_bonus);
                owner_stats.next_bomb_damage_bonus = 0;
            }
            if owner_stats.next_bomb_self_damage_reduction > 0 {
                owner_stats.active_bomb_self_damage_reduction =
                    owner_stats.next_bomb_self_damage_reduction;
                owner_stats.next_bomb_self_damage_reduction = 0;
            }
        }

        if targets_opponent(effect.effect_type) {
            // Check for reflection on status effects (excluding ApplyReflection itself)
            let is_reflectable_status = is_status_effect(effect.effect_type)
                && !matches!(effect.effect_type, EffectType::ApplyReflection);

            if is_reflectable_status && opponent_status.reflection > 0 {
                // Reflection: status is reflected back to the source (owner)
                opponent_status.reflection = opponent_status.reflection.saturating_sub(1);

                apply_effect(
                    phase,
                    effect.effect_type,
                    effect_value,
                    owner_stats,
                    owner_status,
                    turn,
                    is_owner_player, // Reflected back to owner
                    player_gold,
                    enemy_gold,
                    gold_change,
                    Some(&reflection_source()),
                    log,
                );
            } else {
                // Normal: effect targets the opponent
                apply_effect(
                    phase,
                    effect.effect_type,
                    effect_value,
                    opponent_stats,
                    opponent_status,
                    turn,
                    !is_owner_player, // Target is opponent
                    player_gold,
                    enemy_gold,
                    gold_change,
                    annotated.source.as_ref(),
                    log,
                );
            }

            if phase != TriggerType::OnDealNonWeaponDamage
                && matches!(effect.effect_type, EffectType::DealNonWeaponDamage)
                && effect_value > 0
            {
                dealt_non_weapon_damage = true;
            }
        } else {
            let owner_gold_before = if is_owner_player {
                *player_gold
            } else {
                *enemy_gold
            };
            let owner_arm_before = owner_stats.arm;
            // Effect targets self (owner)
            apply_effect(
                phase,
                effect.effect_type,
                effect_value,
                owner_stats,
                owner_status,
                turn,
                is_owner_player, // Target is owner
                player_gold,
                enemy_gold,
                gold_change,
                annotated.source.as_ref(),
                log,
            );
            let owner_gold_after = if is_owner_player {
                *player_gold
            } else {
                *enemy_gold
            };
            if matches!(
                effect.effect_type,
                EffectType::ConsumeGoldForArmor | EffectType::GoldToArmor
            ) && (owner_gold_after < owner_gold_before
                || owner_stats.arm > owner_arm_before)
            {
                gold_armor_conversion_happened = true;
            }
        }

        if effect.once_per_turn {
            if let Some(flag) = triggered_flags.get_mut(index) {
                *flag = true;
            }
        }
    }

    if !first_pass && matches!(phase, TriggerType::Countdown { .. }) {
        // Do not carry a "current bomb" self-damage reduction into future bombs.
        owner_stats.active_bomb_self_damage_reduction = 0;
    }
    if !first_pass && pending_countdown_reduction > 0 {
        reduce_all_countdowns(effects, pending_countdown_reduction);
    }

    if !first_pass && dealt_non_weapon_damage {
        process_triggers_for_phase(
            effects,
            TriggerType::OnDealNonWeaponDamage,
            turn,
            owner_stats,
            owner_status,
            opponent_stats,
            opponent_status,
            triggered_flags,
            is_owner_player,
            owner_acts_first,
            player_gold,
            enemy_gold,
            gold_change,
            log,
        );
    }

    if !first_pass && gold_armor_conversion_happened {
        process_triggers_for_phase(
            effects,
            TriggerType::OnGoldArmorConverted,
            turn,
            owner_stats,
            owner_status,
            opponent_stats,
            opponent_status,
            triggered_flags,
            is_owner_player,
            owner_acts_first,
            player_gold,
            enemy_gold,
            gold_change,
            log,
        );
    }
}

pub(crate) fn reduce_all_countdowns(effects: &mut [AnnotatedItemEffect], reduction: u8) {
    if reduction == 0 {
        return;
    }
    for effect in effects.iter_mut() {
        if let TriggerType::Countdown { turns } = effect.effect.trigger {
            let reduced = turns.saturating_sub(reduction).max(1);
            effect.effect.trigger = TriggerType::Countdown { turns: reduced };
        }
    }
}

/// Returns true if the effect applies a status that can be reflected
fn is_status_effect(effect_type: EffectType) -> bool {
    matches!(
        effect_type,
        EffectType::ApplyChill
            | EffectType::ApplyShrapnel
            | EffectType::ApplyRust
            | EffectType::ApplyBleed
            | EffectType::ApplyReflection
    )
}

pub fn reset_once_per_turn_flags(flags: &mut [bool]) {
    for flag in flags.iter_mut() {
        *flag = false;
    }
}

fn targets_opponent(effect_type: EffectType) -> bool {
    matches!(
        effect_type,
        EffectType::DealDamage
            | EffectType::DealNonWeaponDamage
            | EffectType::ApplyChill
            | EffectType::ApplyRust
            | EffectType::ApplyBleed
            | EffectType::RemoveArmor
            | EffectType::ApplyBomb
            | EffectType::StealGold
            | EffectType::ReduceEnemySpd
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AnnotatedItemEffect, Condition, ItemEffect, LogAction};

    fn annotate(effect: ItemEffect) -> AnnotatedItemEffect {
        AnnotatedItemEffect {
            effect,
            source: None,
        }
    }

    fn annotate_all(effects: Vec<ItemEffect>) -> Vec<AnnotatedItemEffect> {
        effects.into_iter().map(annotate).collect()
    }

    #[test]
    fn test_battle_start_trigger() {
        // acts_first doesn't matter for BattleStart
        assert!(should_trigger(TriggerType::BattleStart, 1, true, false));
        assert!(!should_trigger(TriggerType::BattleStart, 2, false, false));
    }

    #[test]
    fn test_first_turn_trigger() {
        // acts_first doesn't matter for unconditional FirstTurn
        assert!(should_trigger(TriggerType::FirstTurn, 1, true, false));
        assert!(!should_trigger(TriggerType::FirstTurn, 2, false, false));
    }

    #[test]
    fn test_first_turn_if_faster_trigger() {
        // Only fires on turn 1 AND if this combatant acts first
        assert!(should_trigger(
            TriggerType::FirstTurnIfFaster,
            1,
            true,
            true
        ));
        assert!(!should_trigger(
            TriggerType::FirstTurnIfFaster,
            1,
            true,
            false
        ));
        assert!(!should_trigger(
            TriggerType::FirstTurnIfFaster,
            2,
            false,
            true
        ));
    }

    #[test]
    fn test_first_turn_if_slower_trigger() {
        // Only fires on turn 1 AND if this combatant acts second
        assert!(should_trigger(
            TriggerType::FirstTurnIfSlower,
            1,
            true,
            false
        ));
        assert!(!should_trigger(
            TriggerType::FirstTurnIfSlower,
            1,
            true,
            true
        ));
        assert!(!should_trigger(
            TriggerType::FirstTurnIfSlower,
            2,
            false,
            false
        ));
    }

    #[test]
    fn test_first_turn_if_faster_effect_blocked_when_slower() {
        // Simulate Frost Wisp: "If it acts first on Turn 1: apply 2 Chill"
        // If player is faster (SPD 3 vs enemy SPD 1), enemy's FirstTurnIfFaster should NOT fire
        let mut owner_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1, // Enemy is slower
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 3, // Player is faster
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::FirstTurnIfFaster,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        // Enemy is slower (acts_first = false), so FirstTurnIfFaster should NOT fire
        process_triggers_for_phase(
            &mut effects,
            TriggerType::FirstTurnIfFaster,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            false, // is_owner_player (enemy is owner)
            false, // owner_acts_first: enemy is slower
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Chill should NOT be applied because enemy is slower
        assert_eq!(
            opponent_status.chill, 0,
            "Chill should NOT be applied when enemy is slower"
        );
        assert!(log.is_empty(), "No log entries should be created");
    }

    #[test]
    fn test_first_turn_if_faster_effect_fires_when_faster() {
        // Same scenario but enemy is faster this time
        let mut owner_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 3, // Enemy is faster
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1, // Player is slower
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::FirstTurnIfFaster,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        // Enemy is faster (acts_first = true), so FirstTurnIfFaster should fire
        process_triggers_for_phase(
            &mut effects,
            TriggerType::FirstTurnIfFaster,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            false, // is_owner_player (enemy is owner)
            true,  // owner_acts_first: enemy is faster
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Chill SHOULD be applied because enemy acts first
        assert_eq!(
            opponent_status.chill, 2,
            "Chill should be applied when enemy is faster"
        );
        assert_eq!(log.len(), 1, "One log entry for ApplyChill");
    }

    #[test]
    fn test_turn_start_trigger() {
        assert!(should_trigger(TriggerType::TurnStart, 1, true, false));
        assert!(should_trigger(TriggerType::TurnStart, 5, false, false));
    }

    #[test]
    fn test_every_other_turn_trigger() {
        assert!(!should_trigger(
            TriggerType::EveryOtherTurn,
            1,
            false,
            false
        ));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 2, false, false));
        assert!(!should_trigger(
            TriggerType::EveryOtherTurn,
            3,
            false,
            false
        ));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 4, false, false));
    }

    #[test]
    fn test_exposed_condition() {
        assert!(check_exposed(0));
        assert!(check_exposed(-1));
        assert!(!check_exposed(2));
    }

    #[test]
    fn test_wounded_condition() {
        assert!(check_wounded(4, 10));
        assert!(!check_wounded(5, 10));
        assert!(check_wounded(0, 10));
    }

    #[test]
    fn test_deterministic_effect_ordering() {
        let mut stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 1,
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 1,
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = annotate_all(vec![
            ItemEffect {
                trigger: TriggerType::TurnStart,
                once_per_turn: false,
                effect_type: EffectType::GainAtk,
                value: 2,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::TurnStart,
                once_per_turn: false,
                effect_type: EffectType::GainAtk,
                value: 1,
                condition: Condition::None,
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::TurnStart,
            1,
            &mut stats,
            &mut status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for TurnStart
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(stats.atk, 4);
        // Should have logged 2 ATK changes
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_next_bomb_damage_and_self_damage_reduction_apply_once() {
        let mut owner_stats = CombatantStats {
            hp: 30,
            max_hp: 30,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 3,
            next_bomb_self_damage_reduction: 2,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 30,
            max_hp: 30,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = annotate_all(vec![
            ItemEffect {
                trigger: TriggerType::Countdown { turns: 2 },
                once_per_turn: false,
                effect_type: EffectType::DealNonWeaponDamage,
                value: 10,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::Countdown { turns: 2 },
                once_per_turn: false,
                effect_type: EffectType::DealSelfNonWeaponDamage,
                value: 4,
                condition: Condition::None,
            },
        ]);

        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::Countdown { turns: 2 },
            2,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Enemy damage: 10 + 3 next-bomb bonus
        assert_eq!(opponent_stats.hp, 17);
        // Self damage: 4 reduced by 2
        assert_eq!(owner_stats.hp, 28);
        // Both next-bomb modifiers must be consumed.
        assert_eq!(owner_stats.next_bomb_damage_bonus, 0);
        assert_eq!(owner_stats.next_bomb_self_damage_reduction, 0);
        assert_eq!(owner_stats.active_bomb_self_damage_reduction, 0);
    }

    #[test]
    fn test_turn_start_bomb_self_damage_reduction_refreshes_without_stacking() {
        let mut owner_stats = CombatantStats {
            hp: 30,
            max_hp: 30,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::TurnStart,
            once_per_turn: false,
            effect_type: EffectType::ReduceNextBombSelfDamage,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::TurnStart,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );
        assert_eq!(owner_stats.next_bomb_self_damage_reduction, 2);

        // Next turn should refresh to the same floor, not accumulate to 4.
        process_triggers_for_phase(
            &mut effects,
            TriggerType::TurnStart,
            2,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );
        assert_eq!(owner_stats.next_bomb_self_damage_reduction, 2);
    }

    #[test]
    fn test_remove_own_armor_reduces_target_armor_and_logs_it() {
        let mut owner_stats = CombatantStats {
            arm: 5,
            ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnDealNonWeaponDamage,
            once_per_turn: false,
            effect_type: EffectType::RemoveOwnArmor,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold = 0u16;
        let mut enemy_gold = 0u16;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnDealNonWeaponDamage,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            false,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(owner_stats.arm, 3);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, LogAction::ArmorChange);
        assert_eq!(log[0].value, -2);
    }

    #[test]
    fn test_gold_to_armor_converts_owner_gold_by_ratio() {
        let mut owner_stats = CombatantStats {
            arm: 1,
            ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::BattleStart,
            once_per_turn: false,
            effect_type: EffectType::GoldToArmor,
            value: 4,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold = 16u16;
        let mut enemy_gold = 0u16;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::BattleStart,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(owner_stats.arm, 5);
        assert_eq!(player_gold, 16);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, LogAction::ArmorChange);
        assert_eq!(log[0].value, 4);
    }

    // ========================================================================
    // Reflection Status Effect Tests
    // ========================================================================

    fn make_test_combatant() -> (CombatantStats, StatusEffects) {
        (
            CombatantStats {
                hp: 10,
                max_hp: 10,
                atk: 3,
                arm: 0,
                spd: 1,
                dig: 0,
                armor_piercing: 0,
                stored_damage: 0,
                gear_atk_bonus: 0,
                half_gear_atk_after_second_strike: false,
                next_bomb_damage_bonus: 0,
                next_bomb_self_damage_reduction: 0,
                active_bomb_self_damage_reduction: 0,
                non_weapon_damage_bonus: 0,
                next_non_weapon_damage_bonus: 0,
                preserve_shrapnel_cap: 0,
                shards_every_turn: false,
                        ..Default::default()
            },
            StatusEffects::default(),
        )
    }

    #[test]
    fn test_reflection_blocks_and_redirects_chill() {
        let (mut owner_stats, mut owner_status) = make_test_combatant();
        let (mut opponent_stats, mut opponent_status) = make_test_combatant();
        opponent_status.reflection = 1; // Opponent has 1 Reflection stack

        // Owner tries to apply Chill to opponent
        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Chill should be reflected back to owner
        assert_eq!(owner_status.chill, 2, "Chill should be reflected to owner");
        assert_eq!(opponent_status.chill, 0, "Opponent should not have Chill");
        assert_eq!(
            opponent_status.reflection, 0,
            "Reflection stack should be consumed"
        );
    }

    #[test]
    fn test_reflection_blocks_and_redirects_bleed() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 2; // Opponent has 2 Reflection stacks

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyBleed,
            value: 3,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(owner_status.bleed, 3, "Bleed should be reflected to owner");
        assert_eq!(opponent_status.bleed, 0, "Opponent should not have Bleed");
        assert_eq!(
            opponent_status.reflection, 1,
            "One Reflection stack should remain"
        );
    }

    #[test]
    fn test_reflection_does_not_block_apply_reflection() {
        // ApplyReflection itself should NOT be reflected (prevents infinite loops)
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 1;

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyReflection,
            value: 2,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // ApplyReflection should not be reflected, but it is a self-targeting effect.
        assert_eq!(
            owner_status.reflection, 2,
            "Owner should gain Reflection from its own effect"
        );
        assert_eq!(
            opponent_status.reflection, 1,
            "Opponent reflection should remain unchanged"
        );
    }

    #[test]
    fn test_reflection_zero_stacks_does_not_block() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 0; // No Reflection

        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyRust,
            value: 1,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Without Reflection, Rust should apply to opponent normally
        assert_eq!(owner_status.rust, 0, "Owner should not have Rust");
        assert_eq!(opponent_status.rust, 1, "Opponent should have Rust applied");
    }

    #[test]
    fn test_reflection_does_not_affect_direct_damage() {
        let mut owner_stats = CombatantStats::default();
        owner_stats.hp = 20;
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        opponent_stats.hp = 20;
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 5; // Lots of Reflection

        // Direct damage effect (DealDamage targets opponent but is not a status effect)
        let mut effects = vec![annotate(ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::DealDamage,
            value: 5,
            condition: Condition::None,
        })];
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Direct damage should NOT be reflected
        assert_eq!(owner_stats.hp, 20, "Owner HP should be unchanged");
        assert_eq!(opponent_stats.hp, 15, "Opponent should take 5 damage");
        assert_eq!(
            opponent_status.reflection, 5,
            "Reflection should not be consumed for non-status effects"
        );
    }

    #[test]
    fn test_reflection_multiple_status_effects_consumes_multiple_stacks() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 2; // 2 Reflection stacks

        // Two different status effects that target opponent (Chill and Rust)
        // Note: ApplyShrapnel targets SELF (for Shard Beetle, Spiked Bracers), not opponent
        let mut effects = annotate_all(vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 1,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyRust,
                value: 2,
                condition: Condition::None,
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Both status effects should be reflected
        assert_eq!(owner_status.chill, 1, "Chill should be reflected to owner");
        assert_eq!(owner_status.rust, 2, "Rust should be reflected to owner");
        assert_eq!(opponent_status.chill, 0, "Opponent should not have Chill");
        assert_eq!(opponent_status.rust, 0, "Opponent should not have Rust");
        assert_eq!(
            opponent_status.reflection, 0,
            "Both Reflection stacks should be consumed"
        );
    }

    #[test]
    fn test_reflection_partial_block_when_stacks_run_out() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 1; // Only 1 Reflection stack

        // Three status effects, but only 1 Reflection
        let mut effects = annotate_all(vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 1,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyBleed,
                value: 2,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyRust,
                value: 3,
                condition: Condition::None,
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // First effect (Chill) should be reflected, rest should hit opponent
        assert_eq!(
            owner_status.chill, 1,
            "First effect (Chill) should be reflected to owner"
        );
        assert_eq!(owner_status.bleed, 0, "Bleed should NOT be on owner");
        assert_eq!(owner_status.rust, 0, "Rust should NOT be on owner");

        assert_eq!(opponent_status.chill, 0, "Chill should NOT be on opponent");
        assert_eq!(
            opponent_status.bleed, 2,
            "Bleed should hit opponent (no Reflection left)"
        );
        assert_eq!(
            opponent_status.rust, 3,
            "Rust should hit opponent (no Reflection left)"
        );
        assert_eq!(
            opponent_status.reflection, 0,
            "Reflection should be exhausted"
        );
    }

    #[test]
    fn test_conditional_effect_sees_status_from_earlier_effect_in_same_phase() {
        // Tests the Frost Lantern + Frostguard Buckler synergy:
        // With two-pass processing, unconditional status effects (Frost Lantern's Chill)
        // are applied first, then conditional effects (Frostguard Buckler's ARM bonus)
        // are evaluated - so the synergy works regardless of item order.
        let mut owner_stats = CombatantStats {
            hp: 100,
            max_hp: 100,
            atk: 10,
            arm: 0,
            spd: 5,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 100,
            max_hp: 100,
            atk: 10,
            arm: 0,
            spd: 5,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        // Frost Lantern first, Frostguard Buckler second
        let mut effects = annotate_all(vec![
            // Frost Lantern: BattleStart, apply 2 Chill to enemy
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 2,
                condition: Condition::None,
            },
            // Frostguard Buckler: BattleStart, gain 3 ARM if enemy has Chill
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::GainArmor,
                value: 3,
                condition: Condition::EnemyHasStatus(StatusType::Chill),
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::BattleStart,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true, // is_owner_player
            true, // owner_acts_first
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Frost Lantern should have applied Chill
        assert_eq!(
            opponent_status.chill, 2,
            "Enemy should have 2 Chill from Frost Lantern"
        );

        // Frostguard Buckler's conditional effect should fire because two-pass processing
        // ensures status effects are applied before conditional effects are evaluated
        assert_eq!(
            owner_stats.arm, 3,
            "Frostguard Buckler should gain 3 ARM because enemy has Chill (two-pass processing)"
        );
    }

    #[test]
    fn test_conditional_effect_works_regardless_of_item_order() {
        // With two-pass processing, item order no longer matters for status->conditional synergies.
        // Even if the conditional effect comes first in the array, it will be evaluated
        // in the second pass after status effects are applied.
        let mut owner_stats = CombatantStats {
            hp: 100,
            max_hp: 100,
            atk: 10,
            arm: 0,
            spd: 5,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 100,
            max_hp: 100,
            atk: 10,
            arm: 0,
            spd: 5,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
                    ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        // Reverse order: Frostguard Buckler first, Frost Lantern second
        let mut effects = annotate_all(vec![
            // Frostguard Buckler first: gain 3 ARM if enemy has Chill
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::GainArmor,
                value: 3,
                condition: Condition::EnemyHasStatus(StatusType::Chill),
            },
            // Frost Lantern second: apply 2 Chill to enemy
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 2,
                condition: Condition::None,
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::BattleStart,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true, // is_owner_player
            true, // owner_acts_first
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Frost Lantern should have applied Chill (even though it's second in the array)
        assert_eq!(
            opponent_status.chill, 2,
            "Enemy should have 2 Chill from Frost Lantern"
        );

        // Frostguard Buckler's conditional effect SHOULD fire now, because two-pass
        // processing applies status effects first, then evaluates conditional effects
        assert_eq!(
            owner_stats.arm, 3,
            "Frostguard Buckler should gain ARM - two-pass processing makes order irrelevant"
        );
    }

    #[test]
    fn test_on_deal_non_weapon_damage_triggers_once_per_turn() {
        let mut owner_stats = CombatantStats {
            hp: 20,
            max_hp: 20,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 20,
            max_hp: 20,
            ..Default::default()
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = annotate_all(vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::DealNonWeaponDamage,
                value: 2,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnDealNonWeaponDamage,
                once_per_turn: true,
                effect_type: EffectType::GainArmor,
                value: 1,
                condition: Condition::None,
            },
        ]);
        let mut flags = vec![false; effects.len()];
        let mut player_gold = 0u16;
        let mut enemy_gold = 0u16;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        // Two OnHit passes in same turn: non-weapon damage occurs twice.
        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );
        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(owner_stats.arm, 1, "Armor gain should trigger once per turn");

        // Next turn: reset once/turn flags and verify it can trigger again.
        reset_once_per_turn_flags(&mut flags);
        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            2,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );
        assert_eq!(owner_stats.arm, 2, "Armor gain should trigger again next turn");
    }
}
