use anchor_lang::prelude::*;

// =============================================================================
// Combat Log Types - For turn-by-turn visualization
// =============================================================================

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

/// A single entry in the combat log.
/// Compact format to minimize data cost (~5 bytes per entry).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct CombatLogEntry {
    /// Turn number (1-50)
    pub turn: u8,
    /// true = player action, false = enemy action
    pub is_player: bool,
    /// The action type
    pub action: LogAction,
    /// Primary value (damage, healing, stacks, etc.)
    pub value: i16,
    /// Extra data (status_id for ApplyStatus, 0 otherwise)
    pub extra: u8,
}

impl CombatLogEntry {
    pub fn new(turn: u8, is_player: bool, action: LogAction, value: i16, extra: u8) -> Self {
        Self {
            turn,
            is_player,
            action,
            value,
            extra,
        }
    }

    pub fn attack(turn: u8, is_player: bool, damage: i16) -> Self {
        Self::new(turn, is_player, LogAction::Attack, damage, 0)
    }

    pub fn heal(turn: u8, is_player: bool, amount: i16) -> Self {
        Self::new(turn, is_player, LogAction::Heal, amount, 0)
    }

    pub fn apply_status(turn: u8, is_player: bool, status_id: u8, stacks: i16) -> Self {
        Self::new(turn, is_player, LogAction::ApplyStatus, stacks, status_id)
    }

    pub fn status_damage(turn: u8, is_player: bool, damage: i16) -> Self {
        Self::new(turn, is_player, LogAction::StatusDamage, damage, 0)
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

// =============================================================================
// Combat State Types
// =============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct StatusEffects {
    pub chill: u8,
    pub shrapnel: u8,
    pub rust: u8,
    pub bleed: u8,
    pub reflection: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct CombatantInput {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub dig: i16,
    pub strikes: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum TriggerType {
    BattleStart,
    FirstTurn,
    TurnStart,
    EveryOtherTurn,
    OnHit,
    Exposed,
    Wounded,
    Countdown { turns: u8 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum EffectType {
    DealDamage,
    DealNonWeaponDamage,
    Heal,
    GainArmor,
    GainAtk,
    GainSpd,
    GainDig,
    GainGold,
    ApplyBomb,
    ApplyChill,
    ApplyShrapnel,
    ApplyRust,
    ApplyBleed,
    RemoveArmor,
    GainStrikes,
    StealGold,
    GoldToArmor,
    ApplyReflection,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct ItemEffect {
    pub trigger: TriggerType,
    pub once_per_turn: bool,
    pub effect_type: EffectType,
    pub value: i16,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ResolutionType {
    PlayerDefeated,
    EnemyDefeated,
    SuddenDeathPlayerWin,
    SuddenDeathEnemyWin,
    FailsafePlayerWin,
    FailsafeEnemyWin,
}

pub(crate) struct CombatState {
    pub turn: u8,
    pub player_hp: i16,
    pub player_max_hp: u16,
    pub player_atk: i16,
    pub player_arm: i16,
    pub player_spd: i16,
    pub player_strikes: u8,
    pub player_status: StatusEffects,
    pub enemy_hp: i16,
    pub enemy_max_hp: u16,
    pub enemy_atk: i16,
    pub enemy_arm: i16,
    pub enemy_spd: i16,
    pub enemy_strikes: u8,
    pub enemy_status: StatusEffects,
    pub sudden_death_bonus: i16,
    /// Net gold change during combat (positive = player gains, negative = player loses)
    pub gold_change: i16,
}

impl CombatState {
    /// Get player stats as CombatantStats
    pub fn player_stats(&self) -> crate::triggers::CombatantStats {
        crate::triggers::CombatantStats {
            hp: self.player_hp,
            max_hp: self.player_max_hp,
            atk: self.player_atk,
            arm: self.player_arm,
            spd: self.player_spd,
        }
    }

    /// Get enemy stats as CombatantStats
    pub fn enemy_stats(&self) -> crate::triggers::CombatantStats {
        crate::triggers::CombatantStats {
            hp: self.enemy_hp,
            max_hp: self.enemy_max_hp,
            atk: self.enemy_atk,
            arm: self.enemy_arm,
            spd: self.enemy_spd,
        }
    }

    /// Update player stats from CombatantStats
    pub fn set_player_stats(&mut self, stats: &crate::triggers::CombatantStats) {
        self.player_hp = stats.hp;
        self.player_atk = stats.atk;
        self.player_arm = stats.arm;
        self.player_spd = stats.spd;
    }

    /// Update enemy stats from CombatantStats
    pub fn set_enemy_stats(&mut self, stats: &crate::triggers::CombatantStats) {
        self.enemy_hp = stats.hp;
        self.enemy_atk = stats.atk;
        self.enemy_arm = stats.arm;
        self.enemy_spd = stats.spd;
    }
}
