//! ADSR envelope generator for synthesizers.

/// Attack-Decay-Sustain-Release envelope.
///
/// All time values are in seconds. Sustain is a level (0.0–1.0).
#[derive(Debug, Clone, Copy)]
pub struct AdsrEnvelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl AdsrEnvelope {
    /// Calculate the amplitude at time `t` for a note of given `duration`.
    ///
    /// - During `[0, attack)`: linear ramp from 0 to 1.
    /// - During `[attack, attack+decay)`: linear ramp from 1 to sustain level.
    /// - During `[attack+decay, duration)`: sustain level.
    /// - During `[duration, duration+release)`: linear ramp from sustain to 0.
    /// - After `duration+release`: 0.
    pub fn amplitude(&self, t: f64, note_duration: f64) -> f64 {
        if t < 0.0 {
            return 0.0;
        }

        if t < self.attack {
            // Attack phase
            if self.attack <= 0.0 {
                1.0
            } else {
                t / self.attack
            }
        } else if t < self.attack + self.decay {
            // Decay phase
            if self.decay <= 0.0 {
                self.sustain
            } else {
                let decay_t = (t - self.attack) / self.decay;
                1.0 - decay_t * (1.0 - self.sustain)
            }
        } else if t < note_duration {
            // Sustain phase
            self.sustain
        } else if t < note_duration + self.release {
            // Release phase
            if self.release <= 0.0 {
                0.0
            } else {
                let release_t = (t - note_duration) / self.release;
                self.sustain * (1.0 - release_t)
            }
        } else {
            0.0
        }
    }

    /// Total sound duration including release tail.
    pub fn total_duration(&self, note_duration: f64) -> f64 {
        note_duration + self.release
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env() -> AdsrEnvelope {
        AdsrEnvelope {
            attack: 0.01,
            decay: 0.05,
            sustain: 0.7,
            release: 0.1,
        }
    }

    #[test]
    fn starts_at_zero() {
        let env = test_env();
        assert!((env.amplitude(0.0, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn reaches_peak_at_attack() {
        let env = test_env();
        assert!((env.amplitude(0.01, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn reaches_sustain_after_decay() {
        let env = test_env();
        let t = 0.01 + 0.05; // attack + decay
        assert!((env.amplitude(t, 1.0) - 0.7).abs() < 1e-10);
    }

    #[test]
    fn sustain_level_holds() {
        let env = test_env();
        assert!((env.amplitude(0.5, 1.0) - 0.7).abs() < 1e-10);
    }

    #[test]
    fn release_starts_at_sustain() {
        let env = test_env();
        let t = 1.0 + 0.001; // just past note_duration
        let amp = env.amplitude(t, 1.0);
        assert!((amp - 0.7).abs() < 0.01);
    }

    #[test]
    fn release_ends_at_zero() {
        let env = test_env();
        let t = 1.0 + 0.1; // note_duration + release
        assert!((env.amplitude(t, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn after_release_is_zero() {
        let env = test_env();
        assert!((env.amplitude(2.0, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn negative_time_is_zero() {
        let env = test_env();
        assert!((env.amplitude(-0.1, 1.0)).abs() < 1e-10);
    }

    #[test]
    fn total_duration() {
        let env = test_env();
        let total = env.total_duration(1.0);
        assert!((total - 1.1).abs() < 1e-10);
    }

    #[test]
    fn zero_attack_instant_peak() {
        let env = AdsrEnvelope {
            attack: 0.0,
            decay: 0.05,
            sustain: 0.7,
            release: 0.1,
        };
        assert!((env.amplitude(0.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn envelope_always_non_negative() {
        let env = test_env();
        for i in 0..2000 {
            let t = i as f64 / 1000.0;
            let amp = env.amplitude(t, 1.0);
            assert!(amp >= 0.0, "amplitude negative at t={t}: {amp}");
        }
    }

    #[test]
    fn envelope_never_exceeds_one() {
        let env = test_env();
        for i in 0..2000 {
            let t = i as f64 / 1000.0;
            let amp = env.amplitude(t, 1.0);
            assert!(amp <= 1.0 + 1e-10, "amplitude > 1 at t={t}: {amp}");
        }
    }
}
