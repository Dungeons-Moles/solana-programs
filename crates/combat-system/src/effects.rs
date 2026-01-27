use crate::state::StatusEffects;

pub fn apply_chill_to_strikes(base_strikes: u8, chill_stacks: u8) -> u8 {
    let reduced = base_strikes.saturating_sub(chill_stacks);
    reduced.max(1)
}

pub fn process_shrapnel_retaliation(shrapnel_stacks: u8, attacker_hp: i16) -> i16 {
    if shrapnel_stacks == 0 {
        return attacker_hp;
    }

    let damage = i16::from(shrapnel_stacks);
    attacker_hp.checked_sub(damage).unwrap_or(i16::MIN)
}

pub fn process_rust_decay(rust_stacks: u8, current_arm: i16) -> i16 {
    if rust_stacks == 0 {
        return current_arm.max(0);
    }

    let decay = i16::from(rust_stacks);
    let reduced = current_arm.checked_sub(decay).unwrap_or(i16::MIN);
    reduced.max(0)
}

pub fn process_bleed_damage(bleed_stacks: u8, current_hp: i16) -> i16 {
    if bleed_stacks == 0 {
        return current_hp;
    }

    let damage = i16::from(bleed_stacks);
    current_hp.checked_sub(damage).unwrap_or(i16::MIN)
}

pub fn decay_status_effects(status: &mut StatusEffects) {
    status.chill = status.chill.saturating_sub(1);
    status.bleed = status.bleed.saturating_sub(1);
    status.shrapnel = 0;
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
    fn test_shrapnel_retaliation_and_clear() {
        let hp = process_shrapnel_retaliation(3, 10);
        assert_eq!(hp, 7);

        let mut status = StatusEffects {
            shrapnel: 4,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.shrapnel, 0);
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
        let hp = process_bleed_damage(2, 10);
        assert_eq!(hp, 8);

        let mut status = StatusEffects {
            bleed: 3,
            ..StatusEffects::default()
        };
        decay_status_effects(&mut status);
        assert_eq!(status.bleed, 2);
    }
}
