use anchor_lang::prelude::*;

/// Actions that can be logged during combat.
/// Each action type has a specific meaning for the `value` and `extra` fields.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LogAction {
    /// Attack action. value = damage dealt (after armor reduction)
    Attack = 0,
    /// Healing action. value = HP restored
    Heal = 1,
    /// Status effect applied. value = stacks, extra = status_id (0=Chill, 1=Shrapnel, 2=Rust, 3=Bleed, 4=Reflection)
    ApplyStatus = 2,
    /// Damage from status effect (Bleed). value = damage taken
    StatusDamage = 3,
    /// Armor changed. value = amount changed (positive or negative)
    ArmorChange = 4,
    /// Attack stat changed. value = amount changed
    AtkChange = 5,
    /// Speed stat changed. value = amount changed
    SpdChange = 6,
    /// Non-weapon damage (ignores armor). value = damage dealt
    NonWeaponDamage = 7,
    /// Shrapnel retaliation damage. value = damage taken
    ShrapnelRetaliation = 8,
    /// Gold stolen. value = amount stolen (positive = player gained, negative = player lost)
    GoldStolen = 9,
}

/// Status effect IDs for LogAction::ApplyStatus
pub const STATUS_CHILL: u8 = 0;
pub const STATUS_SHRAPNEL: u8 = 1;
pub const STATUS_RUST: u8 = 2;
pub const STATUS_BLEED: u8 = 3;
pub const STATUS_REFLECTION: u8 = 4;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
#[repr(u8)]
pub enum CombatSourceKind {
    Tool = 0,
    Gear = 1,
    Itemset = 2,
    Enemy = 3,
    Boss = 4,
    Status = 5,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub struct CombatSourceRef {
    pub kind: CombatSourceKind,
    pub id: [u8; 16],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq, InitSpace)]
pub struct CombatContribution {
    pub source: CombatSourceRef,
    pub value: i16,
}

/// Status type enum for conditions
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
#[repr(u8)]
pub enum StatusType {
    Chill = 0,
    Shrapnel = 1,
    Rust = 2,
    Bleed = 3,
    Reflection = 4,
}

/// Conditions that must be met for an effect to fire
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace, Default,
)]
#[repr(u8)]
pub enum Condition {
    /// No additional condition required
    #[default]
    None = 0,
    /// Enemy must have the specified status effect
    EnemyHasStatus(StatusType) = 1,
    /// Enemy must have armor > 0
    EnemyHasArmor = 2,
    /// Enemy must have armor <= 0
    EnemyHasNoArmor = 9,
    /// Owner's DIG must be greater than enemy's DIG
    DigGreaterThanEnemyDig = 3,
    /// Owner's SPD must be greater than enemy's SPD
    SpdGreaterThanEnemySpd = 4,
    /// Owner must be Wounded (HP < 50% max)
    OwnerWounded = 5,
    /// Owner must be Exposed (ARM <= 0)
    OwnerExposed = 6,
    /// Enemy must be Wounded (HP < 50% max)
    EnemyWounded = 7,
    /// Owner must have armor > 0
    OwnerHasArmor = 8,
    /// Owner must have armor >= value
    OwnerArmorAtLeast(u8) = 10,
    /// Owner must have the specified status effect
    OwnerHasStatus(StatusType) = 11,
    /// Enemy must have at least N stacks of the specified status
    EnemyHasStatusAtLeast(StatusType, u8) = 12,
    /// Enemy must have no armor and at least N stacks of the specified status
    EnemyHasNoArmorAndStatusAtLeast(StatusType, u8) = 13,
    /// Enemy has the specified status OR has no armor (disjunctive)
    EnemyHasStatusOrNoArmor(StatusType) = 14,
    /// Owner must have at least N gold
    OwnerGoldAtLeast(u16) = 15,
    /// Enemy must have at least N gold
    EnemyGoldAtLeast(u16) = 16,
    /// Owner's DIG must be greater than enemy's Armor
    OwnerDigGreaterThanEnemyArmor = 17,
}

/// A single entry in the combat log.
/// Compact format to minimize data cost (~5 bytes per entry).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CombatLogEntry {
    /// Turn number (1-50)
    pub turn: u8,
    /// true = player action, false = enemy action
    pub is_player: bool,
    /// The action type
    pub action: LogAction,
    /// Primary value (damage, healing, stacks, etc.)
    pub value: i16,
    /// Extra data (status_id for ApplyStatus, SPD bonus for Attack, 0 otherwise)
    pub extra: u8,
    pub source: Option<CombatSourceRef>,
    pub contributions: Vec<CombatContribution>,
}

impl CombatLogEntry {
    pub fn new(turn: u8, is_player: bool, action: LogAction, value: i16, extra: u8) -> Self {
        Self {
            turn,
            is_player,
            action,
            value,
            extra,
            source: None,
            contributions: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: CombatSourceRef) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_contributions(mut self, contributions: Vec<CombatContribution>) -> Self {
        self.contributions = contributions;
        self
    }

    pub fn attack(turn: u8, is_player: bool, damage: i16) -> Self {
        Self::new(turn, is_player, LogAction::Attack, damage, 0)
    }

    pub fn attack_with_extra(turn: u8, is_player: bool, damage: i16, extra: u8) -> Self {
        Self::new(turn, is_player, LogAction::Attack, damage, extra)
    }

    pub fn heal(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::Heal, amount, 0)
    }

    pub fn apply_status(turn: u8, is_player: bool, status_id: u8, stacks: i16) -> Self {
        Self::new(turn, is_player, LogAction::ApplyStatus, stacks, status_id)
    }

    pub fn status_damage(turn: u8, is_player: bool, damage: i16, status_id: u8) -> Self {
        Self::new(turn, is_player, LogAction::StatusDamage, damage, status_id)
    }

    pub fn armor_change(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::ArmorChange, amount, 0)
    }

    pub fn atk_change(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::AtkChange, amount, 0)
    }

    pub fn spd_change(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::SpdChange, amount, 0)
    }

    pub fn non_weapon_damage(turn: u8, is_player: bool, damage: i16) -> Self {
        Self::new(turn, is_player, LogAction::NonWeaponDamage, damage, 0)
    }

    pub fn shrapnel_retaliation(turn: u8, is_player: bool, damage: i16) -> Self {
        Self::new(turn, is_player, LogAction::ShrapnelRetaliation, damage, 0)
    }

    pub fn gold_stolen(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::GoldStolen, amount, 0)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct StatusEffects {
    pub chill: u8,
    pub shrapnel: u8,
    pub rust: u8,
    pub bleed: u8,
    pub reflection: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct CombatantInput {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub dig: i16,
    pub strikes: u8,
    pub attack_source: Option<CombatSourceRef>,
    #[max_len(16)]
    pub atk_contributions: Vec<CombatContribution>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum TriggerType {
    BattleStart,
    FirstTurn,
    /// Triggers on Turn 1 only if this combatant acts first (higher SPD or enemy on tie)
    FirstTurnIfFaster,
    /// Triggers on Turn 1 only if this combatant acts second (lower SPD)
    FirstTurnIfSlower,
    TurnStart,
    EveryOtherTurn,
    BeforeStrike,
    OnHit,
    Exposed,
    Wounded,
    Countdown {
        turns: u8,
    },
    /// Triggers after combat ends when player wins (processed outside combat system)
    Victory,
    /// Triggers when this combatant takes damage
    OnStruck,
    /// Triggers on a specific turn number
    TurnN {
        turn: u8,
    },
    /// Triggers on the first hit of every other turn (turn 2, 4, 6...)
    EveryOtherTurnFirstHit,
    /// Triggers at the end of each turn
    TurnEnd,
    /// Triggers when enemy takes bleed damage (processed during status phase)
    OnEnemyBleedDamage,
    /// Triggers when rust is applied to enemy
    OnApplyRust,
    /// Triggers when owner deals non-weapon damage
    OnDealNonWeaponDamage,
    /// Triggers when owner gains shrapnel
    OnGainShrapnel,
    /// Triggers when owner successfully converts Gold to Armor
    OnGoldArmorConverted,
    /// Triggers at start of each day (processed outside combat system)
    DayStart,
    /// Triggers once when owner first becomes wounded (HP drops below 50%)
    FirstTimeWounded,
    /// Triggers once when owner first becomes exposed (ARM <= 0)
    FirstTimeExposed,
    /// Triggers once when owner first gains Shrapnel this battle
    FirstTimeGainShrapnel,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum EffectType {
    DealDamage,
    DealNonWeaponDamage,
    Heal,
    GainArmor,
    GainAtk,
    /// ATK gained from gear sources. Used for multi-strike scaling rules.
    GainGearAtk,
    GainSpd,
    GainDig,
    GainGold,
    /// Increase gold gains from all sources by the given percentage.
    AmplifyGoldGain,
    ApplyBomb,
    ApplyChill,
    ApplyShrapnel,
    ApplyRust,
    ApplyBleed,
    RemoveArmor,
    RemoveOwnArmor,
    GainStrikes,
    StealGold,
    GoldToArmor,
    ApplyReflection,
    /// Permanent max HP bonus (e.g., Work Vest's +HP).
    /// Only processed outside combat for max_hp calculation.
    /// Does NOT heal during combat - use Heal for that.
    MaxHp,
    /// Reduce enemy's SPD stat
    ReduceEnemySpd,
    /// Deal non-weapon damage to self (for bomb self-damage)
    DealSelfNonWeaponDamage,
    /// Gain armor equal to floor(gold/10), capped at value
    GoldToArmorScaled,
    /// Consume 1 gold to gain armor (value = armor gained per gold)
    ConsumeGoldForArmor,
    /// Prevent death once per battle, heal for value
    PreventDeath,
    /// Set armor piercing for this battle (strikes ignore value armor)
    SetArmorPiercing,
    /// Convert starting armor to max HP (capped at value)
    ArmorToMaxHp,
    /// Reduce countdown of all bomb items by value
    ReduceAllCountdowns,
    /// Amplify all non-weapon damage by value
    AmplifyNonWeaponDamage,
    /// Apply +damage to the next non-weapon damage instance only.
    EmpowerNextNonWeaponDamage,
    /// Assign the bonus added to the first non-weapon damage each turn.
    DoubleDetonationFirst,
    /// Assign the bonus added to the second non-weapon damage each turn.
    DoubleDetonationSecond,
    /// Store damage each turn (released on Exposed trigger)
    StoreDamage,
    /// Apply +damage to the next bomb trigger only.
    EmpowerNextBombDamage,
    /// Reduce self-damage on the next bomb trigger only.
    ReduceNextBombSelfDamage,
    /// For Pneumatic Drill: strikes beyond the 2nd use half gear ATK bonus.
    HalfGearAtkAfterSecondStrike,
    /// Immune to self-inflicted blast damage
    BlastImmunity,
    /// Double the next bomb trigger effect
    DoubleBombTrigger,
    /// Double OnHit effects (once per turn)
    DoubleOnHitEffects,
    /// Trigger all equipped shard effects
    TriggerAllShards,
    /// Override shard cadence so `EveryOtherTurnFirstHit` effects can trigger every turn.
    ShardsEveryTurn,
    /// Keep up to `value` shrapnel stacks at end of turn.
    PreserveShrapnel,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct ItemEffect {
    pub trigger: TriggerType,
    pub once_per_turn: bool,
    pub effect_type: EffectType,
    pub value: i16,
    /// Optional condition that must be met for the effect to fire
    pub condition: Condition,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct AnnotatedItemEffect {
    pub effect: ItemEffect,
    pub source: Option<CombatSourceRef>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ResolutionType {
    PlayerDefeated,
    EnemyDefeated,
    PvpTiePlayerWin,
    PvpTieEnemyWin,
    SuddenDeathPlayerWin,
    SuddenDeathEnemyWin,
    FailsafePlayerWin,
    FailsafeEnemyWin,
}

/// Per-combatant state during combat. Replaces the flat `player_*`/`enemy_*`
/// fields that were previously duplicated on `CombatState`.
#[derive(Default)]
pub(crate) struct Combatant {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub dig: i16,
    pub strikes: u8,
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
    pub double_bomb_trigger: bool,
    pub pending_self_non_weapon_bonus: i16,
    pub preserve_shrapnel_cap: u8,
    pub shards_every_turn: bool,
    pub attack_source: Option<CombatSourceRef>,
    pub attack_base_value: i16,
    pub atk_contributions: Vec<CombatContribution>,
    pub status: StatusEffects,
    /// Bitmask for first-time event flags (WOUNDED, EXPOSED, GAINED_SHRAPNEL).
    pub first_time_flags: u8,
}

impl Combatant {
    pub const WOUNDED: u8 = 1;
    pub const EXPOSED: u8 = 2;
    pub const GAINED_SHRAPNEL: u8 = 4;
    pub const PHASE_ONE_TRIGGERED: u8 = 8;
    pub const PHASE_TWO_TRIGGERED: u8 = 16;
    pub const PHASE_THREE_TRIGGERED: u8 = 32;
    pub const REFLECTION_DEPLETED: u8 = 64;

    pub fn has_flag(&self, flag: u8) -> bool {
        self.first_time_flags & flag != 0
    }

    pub fn set_flag(&mut self, flag: u8) {
        self.first_time_flags |= flag;
    }

    pub fn to_stats(&self) -> crate::triggers::CombatantStats {
        crate::triggers::CombatantStats {
            hp: self.hp,
            max_hp: self.max_hp,
            atk: self.atk,
            arm: self.arm,
            spd: self.spd,
            dig: self.dig,
            armor_piercing: self.armor_piercing,
            stored_damage: self.stored_damage,
            gear_atk_bonus: self.gear_atk_bonus,
            half_gear_atk_after_second_strike: self.half_gear_atk_after_second_strike,
            next_bomb_damage_bonus: self.next_bomb_damage_bonus,
            next_bomb_self_damage_reduction: self.next_bomb_self_damage_reduction,
            active_bomb_self_damage_reduction: self.active_bomb_self_damage_reduction,
            non_weapon_damage_bonus: self.non_weapon_damage_bonus,
            next_non_weapon_damage_bonus: self.next_non_weapon_damage_bonus,
            gold_gain_bonus: self.gold_gain_bonus,
            non_weapon_hits_this_turn: self.non_weapon_hits_this_turn,
            double_detonation_first: self.double_detonation_first,
            double_detonation_second: self.double_detonation_second,
            double_bomb_trigger: self.double_bomb_trigger,
            pending_self_non_weapon_bonus: self.pending_self_non_weapon_bonus,
            preserve_shrapnel_cap: self.preserve_shrapnel_cap,
            shards_every_turn: self.shards_every_turn,
            attack_source: self.attack_source,
            attack_base_value: self.attack_base_value,
            atk_contributions: self.atk_contributions.clone(),
        }
    }

    pub fn apply_stats(&mut self, stats: &crate::triggers::CombatantStats) {
        self.hp = stats.hp;
        self.max_hp = stats.max_hp;
        self.atk = stats.atk;
        self.arm = stats.arm;
        self.spd = stats.spd;
        self.dig = stats.dig;
        self.armor_piercing = stats.armor_piercing;
        self.stored_damage = stats.stored_damage;
        self.gear_atk_bonus = stats.gear_atk_bonus;
        self.half_gear_atk_after_second_strike = stats.half_gear_atk_after_second_strike;
        self.next_bomb_damage_bonus = stats.next_bomb_damage_bonus;
        self.next_bomb_self_damage_reduction = stats.next_bomb_self_damage_reduction;
        self.active_bomb_self_damage_reduction = stats.active_bomb_self_damage_reduction;
        self.non_weapon_damage_bonus = stats.non_weapon_damage_bonus;
        self.next_non_weapon_damage_bonus = stats.next_non_weapon_damage_bonus;
        self.gold_gain_bonus = stats.gold_gain_bonus;
        self.non_weapon_hits_this_turn = stats.non_weapon_hits_this_turn;
        self.double_detonation_first = stats.double_detonation_first;
        self.double_detonation_second = stats.double_detonation_second;
        self.double_bomb_trigger = stats.double_bomb_trigger;
        self.pending_self_non_weapon_bonus = stats.pending_self_non_weapon_bonus;
        self.preserve_shrapnel_cap = stats.preserve_shrapnel_cap;
        self.shards_every_turn = stats.shards_every_turn;
        self.attack_source = stats.attack_source;
        self.attack_base_value = stats.attack_base_value;
        self.atk_contributions = stats.atk_contributions.clone();
    }
}

pub(crate) struct CombatState {
    pub turn: u8,
    pub player: Combatant,
    pub enemy: Combatant,
    pub sudden_death_bonus: i16,
    pub player_gold: u16,
    pub enemy_gold: u16,
    /// Net gold change during combat (positive = player gains, negative = player loses)
    pub gold_change: i16,
    pub player_acted_this_turn: bool,
    pub enemy_acted_this_turn: bool,
    pub player_preserved_chill: u8,
    pub enemy_preserved_chill: u8,
    pub player_temporary_exposed: bool,
    pub enemy_temporary_exposed: bool,
    pub enemy_boss_id: Option<[u8; 12]>,
}

#[cfg(test)]
mod tests {
    use super::{Combatant, StatusEffects};
    use crate::triggers::CombatantStats;

    #[test]
    fn apply_stats_updates_max_hp() {
        let mut combatant = Combatant {
            hp: 10,
            max_hp: 10,
            atk: 1,
            arm: 1,
            spd: 1,
            dig: 1,
            strikes: 1,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            gold_gain_bonus: 0,
            non_weapon_hits_this_turn: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            attack_source: None,
            atk_contributions: Vec::new(),
            attack_base_value: 0,
            status: StatusEffects::default(),
            first_time_flags: 0,
            double_detonation_first: 0,
            double_detonation_second: 0,
            double_bomb_trigger: false,
            pending_self_non_weapon_bonus: 0,
        };
        let updated_stats = CombatantStats {
            hp: 12,
            max_hp: 12,
            atk: 1,
            arm: 1,
            spd: 1,
            dig: 1,
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

        combatant.apply_stats(&updated_stats);

        assert_eq!(combatant.hp, 12);
        assert_eq!(combatant.max_hp, 12);
    }
}
