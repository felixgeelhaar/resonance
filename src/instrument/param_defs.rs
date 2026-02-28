//! Well-known parameter ID constants and defaults for instrument parameters.
//!
//! These constants define the canonical `ParamId` values that instruments
//! read from `Event.params`, populated by macro mappings.

use crate::event::types::ParamId;

/// Bass synth: low-pass filter cutoff in Hz (default: 800.0).
pub fn cutoff() -> ParamId {
    ParamId("cutoff".to_string())
}

/// Bass/poly synth: oscillator detune in cents (default: 7.0 bass, 12.0 poly).
pub fn detune() -> ParamId {
    ParamId("detune".to_string())
}

/// Poly synth: envelope attack time in seconds (default: 0.15).
pub fn attack() -> ParamId {
    ParamId("attack".to_string())
}

/// Poly synth: envelope release time in seconds (default: 0.4).
pub fn release() -> ParamId {
    ParamId("release".to_string())
}

/// Pluck synth: damping factor 0.0–1.0 (default: 0.996).
pub fn damping() -> ParamId {
    ParamId("damping".to_string())
}

/// Pluck synth: brightness factor 0.0–1.0 (default: 1.0).
pub fn brightness() -> ParamId {
    ParamId("brightness".to_string())
}

/// Drive/distortion amount (default: 0.0).
pub fn drive() -> ParamId {
    ParamId("drive".to_string())
}

/// Reverb mix 0.0–1.0 (default: 0.0).
pub fn reverb_mix() -> ParamId {
    ParamId("reverb_mix".to_string())
}

/// Delay mix 0.0–1.0 (default: 0.0).
pub fn delay_mix() -> ParamId {
    ParamId("delay_mix".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_constants_are_consistent() {
        // Same call produces equal ParamId
        assert_eq!(cutoff(), cutoff());
        assert_eq!(detune(), detune());
        assert_eq!(attack(), attack());
        assert_eq!(release(), release());
        assert_eq!(damping(), damping());
        assert_eq!(brightness(), brightness());
    }

    #[test]
    fn param_constants_are_distinct() {
        let all = vec![
            cutoff(),
            detune(),
            attack(),
            release(),
            damping(),
            brightness(),
            drive(),
            reverb_mix(),
            delay_mix(),
        ];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "params at {} and {} should differ", i, j);
            }
        }
    }

    #[test]
    fn param_id_string_values() {
        assert_eq!(cutoff().0, "cutoff");
        assert_eq!(detune().0, "detune");
        assert_eq!(attack().0, "attack");
        assert_eq!(release().0, "release");
        assert_eq!(damping().0, "damping");
        assert_eq!(brightness().0, "brightness");
        assert_eq!(drive().0, "drive");
        assert_eq!(reverb_mix().0, "reverb_mix");
        assert_eq!(delay_mix().0, "delay_mix");
    }
}
