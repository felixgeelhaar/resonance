//! Oscillator primitives — waveform generation for synthesizers.

use std::f64::consts::PI;

/// Available waveform shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

/// Generate a single sample for the given waveform at the specified phase.
///
/// `phase` is in the range [0.0, 1.0), representing one full cycle.
/// Returns a value in [-1.0, 1.0].
pub fn oscillator(waveform: Waveform, phase: f64) -> f64 {
    match waveform {
        Waveform::Sine => (phase * 2.0 * PI).sin(),
        Waveform::Saw => 2.0 * phase - 1.0,
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Triangle => {
            if phase < 0.25 {
                4.0 * phase
            } else if phase < 0.75 {
                2.0 - 4.0 * phase
            } else {
                4.0 * phase - 4.0
            }
        }
    }
}

/// Convert a MIDI note number to frequency in Hz.
///
/// Standard tuning: A4 (MIDI 69) = 440 Hz.
pub fn midi_to_freq(note: u8) -> f64 {
    440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_at_zero() {
        let v = oscillator(Waveform::Sine, 0.0);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn sine_at_quarter() {
        let v = oscillator(Waveform::Sine, 0.25);
        assert!((v - 1.0).abs() < 1e-10);
    }

    #[test]
    fn saw_at_zero() {
        let v = oscillator(Waveform::Saw, 0.0);
        assert!((v - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn saw_at_one() {
        let v = oscillator(Waveform::Saw, 1.0);
        assert!((v - 1.0).abs() < 1e-10);
    }

    #[test]
    fn saw_midpoint() {
        let v = oscillator(Waveform::Saw, 0.5);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn square_first_half() {
        let v = oscillator(Waveform::Square, 0.25);
        assert!((v - 1.0).abs() < 1e-10);
    }

    #[test]
    fn square_second_half() {
        let v = oscillator(Waveform::Square, 0.75);
        assert!((v - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn triangle_at_zero() {
        let v = oscillator(Waveform::Triangle, 0.0);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn triangle_at_quarter() {
        let v = oscillator(Waveform::Triangle, 0.25);
        assert!((v - 1.0).abs() < 1e-10);
    }

    #[test]
    fn triangle_at_half() {
        let v = oscillator(Waveform::Triangle, 0.5);
        assert!(v.abs() < 1e-10);
    }

    #[test]
    fn triangle_at_three_quarters() {
        let v = oscillator(Waveform::Triangle, 0.75);
        assert!((v - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn all_waveforms_bounded() {
        for wf in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
        ] {
            for i in 0..1000 {
                let phase = i as f64 / 1000.0;
                let v = oscillator(wf, phase);
                assert!(
                    v >= -1.0 && v <= 1.0,
                    "{wf:?} at phase {phase}: {v} out of bounds"
                );
            }
        }
    }

    #[test]
    fn midi_69_is_440() {
        let f = midi_to_freq(69);
        assert!((f - 440.0).abs() < 0.01);
    }

    #[test]
    fn midi_60_is_middle_c() {
        let f = midi_to_freq(60);
        assert!((f - 261.63).abs() < 0.1);
    }

    #[test]
    fn midi_octave_doubles_freq() {
        let f1 = midi_to_freq(60);
        let f2 = midi_to_freq(72);
        assert!((f2 / f1 - 2.0).abs() < 1e-10);
    }

    #[test]
    fn midi_0_very_low() {
        let f = midi_to_freq(0);
        assert!(f > 0.0 && f < 10.0);
    }

    #[test]
    fn midi_127_very_high() {
        let f = midi_to_freq(127);
        assert!(f > 10000.0);
    }
}
