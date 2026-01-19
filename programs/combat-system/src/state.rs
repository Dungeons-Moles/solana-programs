use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct StatusEffects {
    pub chill: u8,
    pub shrapnel: u8,
    pub rust: u8,
    pub bleed: u8,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum TriggerType {
    BattleStart,
    FirstTurn,
    TurnStart,
    EveryOtherTurn,
    OnHit,
    Exposed,
    Wounded,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum EffectType {
    DealDamage,
    DealNonWeaponDamage,
    Heal,
    GainArmor,
    GainAtk,
    GainSpd,
    ApplyChill,
    ApplyShrapnel,
    ApplyRust,
    ApplyBleed,
    RemoveArmor,
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

#[account]
#[derive(InitSpace)]
pub struct CombatState {
    pub game_state: Pubkey,
    pub player: Pubkey,
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
    pub combat_ended: bool,
    pub player_won: bool,
    pub bump: u8,
}
