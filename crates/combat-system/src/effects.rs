use crate::state::StatusEffects;

pub fn chill_damage_bonus(chill_stacks: u8) -> i16 {
    i16::from(chill_stacks.min(3))
}

pub fn apply_chill_to_strikes(base_strikes: u8, chill_stacks: u8) -> u8 {
    let reduced = base_strikes.saturating_sub(chill_stacks);
    reduced.max(1)
}

pub fn process_shrapnel_retaliation(
    strike_atk: i16,
    shrapnel_reflect_bonus: i16,
    attacker_chill: u8,
    attacker_hp: i16,
    attacker_arm: i16,
) -> (i16, i16) {
    if strike_atk <= 0 && shrapnel_reflect_bonus <= 0 {
        return (attacker_hp, attacker_arm);
    }

    let raw = strike_atk
        .max(0)
        .saturating_add(shrapnel_reflect_bonus.max(0))
        .saturating_add(chill_damage_bonus(attacker_chill));

    // Shrapnel retaliates with 50% of the damage (rounded down, minimum 1)
    let damage = (raw / 2).max(1);

    // Damage goes through armor first, then HP
    let arm_damage = damage.min(attacker_arm.max(0));
    let new_arm = attacker_arm.saturating_sub(arm_damage);
    let hp_damage = damage.saturating_sub(arm_damage);
    let new_hp = attacker_hp.checked_sub(hp_damage).unwrap_or(i16::MIN);

    (new_hp, new_arm)
}

pub fn process_rust_decay(rust_stacks: u8, current_arm: i16) -> i16 {
    if rust_stacks == 0 {
        return current_arm.max(0);
    }

    let decay = i16::from(rust_stacks);
    let reduced = current_arm.checked_sub(decay).unwrap_or(i16::MIN);
    reduced.max(0)
}

pub fn process_bleed_damage(bleed_stacks: u8, chill_stacks: u8, current_hp: i16) -> i16 {
    if bleed_stacks == 0 {
        return current_hp;
    }

    let damage = i16::from(bleed_stacks).saturating_add(chill_damage_bonus(chill_stacks));
    current_hp.checked_sub(damage).unwrap_or(i16::MIN)
}

pub fn decay_status_effects(status: &mut StatusEffects) {
    status.chill = status.chill.saturating_sub(1);
    status.bleed = status.bleed.saturating_sub(1);
    // Reflection does not decay - it consumes stacks when triggered
}

pub fn decay_status_effects_preserving_late_chill(
    status: &mut StatusEffects,
    preserved_late_chill: &mut u8,
) {
    let preserved = (*preserved_late_chill).min(status.chill);
    let decayable = status.chill.saturating_sub(preserved);
    status.chill = preserved.saturating_add(decayable.saturating_sub(1));
    *preserved_late_chill = 0;

    status.bleed = status.bleed.saturating_sub(1);
    // Reflection does not decay - it consumes stacks when triggered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chill_reduces_strikes_and_decays() {
        assert_eq!(apply_chill_to_strikes(3, 2), 1);
        assert_eq!(apply_chill_to_strikes(1, 5), 1);

        let mut status = StatusEffects {
            chill: 2,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.chill, 1);
    }

    #[test]
    fn test_late_applied_chill_is_preserved_through_turn_end() {
        let mut status = StatusEffects {
            chill: 1,
            ..StatusEffects::default()
        };
        let mut preserved = 1;
        decay_status_effects_preserving_late_chill(&mut status, &mut preserved);
        assert_eq!(status.chill, 1);
        assert_eq!(preserved, 0);

        let mut stacked = StatusEffects {
            chill: 3,
            ..StatusEffects::default()
        };
        let mut stacked_preserved = 1;
        decay_status_effects_preserving_late_chill(&mut stacked, &mut stacked_preserved);
        assert_eq!(stacked.chill, 2);
    }

    #[test]
    fn test_shrapnel_retaliation_persists() {
        // No armor: raw=3, 50% = 1, straight to HP
        let (hp, arm) = process_shrapnel_retaliation(3, 0, 0, 10, 0);
        assert_eq!(hp, 9);
        assert_eq!(arm, 0);

        // With armor: raw=3, 50% = 1, armor absorbs all
        let (hp_armored, arm_after) = process_shrapnel_retaliation(3, 0, 0, 10, 5);
        assert_eq!(hp_armored, 10);
        assert_eq!(arm_after, 4);

        // Higher damage with armor: raw=3+1+3(chill)=7, 50%=3, armor absorbs all 3
        let (hp_partial, arm_partial) = process_shrapnel_retaliation(3, 1, 4, 10, 3);
        assert_eq!(arm_partial, 0);
        assert_eq!(hp_partial, 10);

        // Minimum 1 damage even with low raw: raw=1, 50%=max(0,1)=1
        let (hp_min, arm_min) = process_shrapnel_retaliation(1, 0, 0, 10, 0);
        assert_eq!(hp_min, 9);
        assert_eq!(arm_min, 0);

        let mut status = StatusEffects {
            shrapnel: 4,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.shrapnel, 4);
    }

    #[test]
    fn test_rust_decay_and_persistence() {
        assert_eq!(process_rust_decay(2, 5), 3);
        assert_eq!(process_rust_decay(10, 4), 0);

        let mut status = StatusEffects {
            rust: 2,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.rust, 2);
    }

    #[test]
    fn test_bleed_damage_and_decay() {
        let hp = process_bleed_damage(2, 0, 10);
        assert_eq!(hp, 8);

        let mut status = StatusEffects {
            bleed: 3,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.bleed, 2);
    }

    #[test]
    fn test_bleed_damage_includes_chill_bonus_capped_at_three() {
        let hp = process_bleed_damage(2, 0, 10);
        assert_eq!(hp, 8);

        let hp_with_chill = process_bleed_damage(2, 4, 10);
        assert_eq!(hp_with_chill, 5);
    }
}
