//! Musical time representation using integer ticks.
//!
//! Uses 960 PPQN (Pulses Per Quarter Note) to avoid floating-point accumulation
//! errors in musical time tracking. All time operations are integer-based;
//! conversion to sample offsets happens only at the rendering boundary.

use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Ticks per quarter note (beat). 960 is a common PPQN that divides cleanly
/// by 2, 3, 4, 5, 6, 8, 10, 12, 15, 16, 20, 24, 32, etc.
pub const TICKS_PER_BEAT: u64 = 960;

/// Default time signature: 4 beats per bar.
pub const DEFAULT_BEATS_PER_BAR: u32 = 4;

/// Musical time measured in integer ticks at [`TICKS_PER_BEAT`] resolution.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Beat {
    ticks: u64,
}

impl Beat {
    /// Zero time — the very start of the timeline.
    pub const ZERO: Beat = Beat { ticks: 0 };

    /// Create a `Beat` from a raw tick count.
    pub fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Create a `Beat` from whole beats (quarter notes).
    pub fn from_beats(beats: u32) -> Self {
        Self {
            ticks: beats as u64 * TICKS_PER_BEAT,
        }
    }

    /// Create a `Beat` from whole bars using the given beats-per-bar.
    pub fn from_bars(bars: u32) -> Self {
        Self {
            ticks: bars as u64 * DEFAULT_BEATS_PER_BAR as u64 * TICKS_PER_BEAT,
        }
    }

    /// Create a `Beat` from a fractional beat value (e.g. 1.5 = one and a half beats).
    pub fn from_beats_f64(beats: f64) -> Self {
        Self {
            ticks: (beats * TICKS_PER_BEAT as f64).round() as u64,
        }
    }

    /// Return the raw tick count.
    pub fn ticks(self) -> u64 {
        self.ticks
    }

    /// Convert to a floating-point beat value.
    pub fn as_beats_f64(self) -> f64 {
        self.ticks as f64 / TICKS_PER_BEAT as f64
    }

    /// Convert this beat position to a sample offset given BPM and sample rate.
    ///
    /// Formula: `(ticks * 60 * sample_rate) / (TICKS_PER_BEAT * bpm)`
    ///
    /// Uses integer arithmetic where possible to maintain determinism.
    pub fn to_sample_offset(self, bpm: f64, sample_rate: u32) -> u64 {
        let numerator = self.ticks as f64 * 60.0 * sample_rate as f64;
        let denominator = TICKS_PER_BEAT as f64 * bpm;
        (numerator / denominator).round() as u64
    }

    /// Quantize to the nearest beat boundary (round down).
    pub fn quantize_to_beat(self) -> Self {
        Self {
            ticks: (self.ticks / TICKS_PER_BEAT) * TICKS_PER_BEAT,
        }
    }

    /// Quantize to the nearest bar boundary (round down).
    pub fn quantize_to_bar(self, beats_per_bar: u32) -> Self {
        let ticks_per_bar = beats_per_bar as u64 * TICKS_PER_BEAT;
        Self {
            ticks: (self.ticks / ticks_per_bar) * ticks_per_bar,
        }
    }
}

impl Ord for Beat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ticks.cmp(&other.ticks)
    }
}

impl PartialOrd for Beat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for Beat {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            ticks: self.ticks + rhs.ticks,
        }
    }
}

impl Sub for Beat {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            ticks: self.ticks.saturating_sub(rhs.ticks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_zero_ticks() {
        assert_eq!(Beat::ZERO.ticks(), 0);
    }

    #[test]
    fn from_beats_converts_correctly() {
        let b = Beat::from_beats(1);
        assert_eq!(b.ticks(), TICKS_PER_BEAT);

        let b4 = Beat::from_beats(4);
        assert_eq!(b4.ticks(), 4 * TICKS_PER_BEAT);
    }

    #[test]
    fn from_bars_uses_default_time_signature() {
        let bar = Beat::from_bars(1);
        assert_eq!(bar.ticks(), DEFAULT_BEATS_PER_BAR as u64 * TICKS_PER_BEAT);

        let two_bars = Beat::from_bars(2);
        assert_eq!(
            two_bars.ticks(),
            2 * DEFAULT_BEATS_PER_BAR as u64 * TICKS_PER_BEAT
        );
    }

    #[test]
    fn from_ticks_round_trip() {
        let b = Beat::from_ticks(12345);
        assert_eq!(b.ticks(), 12345);
    }

    #[test]
    fn from_beats_f64_fractional() {
        let half = Beat::from_beats_f64(0.5);
        assert_eq!(half.ticks(), TICKS_PER_BEAT / 2);

        let one_and_half = Beat::from_beats_f64(1.5);
        assert_eq!(one_and_half.ticks(), TICKS_PER_BEAT + TICKS_PER_BEAT / 2);
    }

    #[test]
    fn as_beats_f64_conversion() {
        let b = Beat::from_beats(3);
        assert!((b.as_beats_f64() - 3.0).abs() < f64::EPSILON);

        let half = Beat::from_ticks(TICKS_PER_BEAT / 2);
        assert!((half.as_beats_f64() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn addition() {
        let a = Beat::from_beats(1);
        let b = Beat::from_beats(2);
        assert_eq!((a + b).ticks(), 3 * TICKS_PER_BEAT);
    }

    #[test]
    fn subtraction_saturates() {
        let a = Beat::from_beats(1);
        let b = Beat::from_beats(3);
        assert_eq!((a - b).ticks(), 0); // saturating
    }

    #[test]
    fn ordering() {
        let a = Beat::from_beats(1);
        let b = Beat::from_beats(2);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, Beat::from_beats(1));
    }

    #[test]
    fn to_sample_offset_at_120_bpm() {
        // At 120 BPM, 44100 Hz: one beat = 0.5 seconds = 22050 samples
        let one_beat = Beat::from_beats(1);
        let samples = one_beat.to_sample_offset(120.0, 44100);
        assert_eq!(samples, 22050);
    }

    #[test]
    fn to_sample_offset_at_60_bpm() {
        // At 60 BPM, 44100 Hz: one beat = 1.0 second = 44100 samples
        let one_beat = Beat::from_beats(1);
        let samples = one_beat.to_sample_offset(60.0, 44100);
        assert_eq!(samples, 44100);
    }

    #[test]
    fn quantize_to_beat_rounds_down() {
        let mid = Beat::from_ticks(TICKS_PER_BEAT + TICKS_PER_BEAT / 2); // 1.5 beats
        let quantized = mid.quantize_to_beat();
        assert_eq!(quantized.ticks(), TICKS_PER_BEAT); // rounds to 1 beat
    }

    #[test]
    fn quantize_to_bar_rounds_down() {
        // 5.5 beats in 4/4 time → bar 1 start (beat 4)
        let pos = Beat::from_ticks(5 * TICKS_PER_BEAT + TICKS_PER_BEAT / 2);
        let quantized = pos.quantize_to_bar(4);
        assert_eq!(quantized.ticks(), 4 * TICKS_PER_BEAT);
    }

    #[test]
    fn determinism_across_many_conversions() {
        let beat = Beat::from_beats_f64(3.75);
        let expected = beat.to_sample_offset(128.0, 48000);
        for _ in 0..1000 {
            assert_eq!(
                Beat::from_beats_f64(3.75).to_sample_offset(128.0, 48000),
                expected
            );
        }
    }
}
