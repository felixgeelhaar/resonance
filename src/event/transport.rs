//! Transport state — play/stop control and drift-free musical time advancement.
//!
//! The transport tracks the current playback position in ticks and advances
//! by sample frames. A fractional tick remainder accumulates to prevent drift
//! over long playback sessions.

use super::beat::{Beat, TICKS_PER_BEAT};

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Stopped,
    Playing,
}

/// Musical transport: tracks position, BPM, and audio format.
#[derive(Debug)]
pub struct Transport {
    bpm: f64,
    sample_rate: u32,
    channels: u16,
    state: PlayState,
    position_ticks: u64,
    /// Fractional tick accumulator for drift-free advancement.
    tick_remainder: f64,
}

impl Transport {
    /// Create a new transport in the stopped state at position zero.
    pub fn new(bpm: f64, sample_rate: u32, channels: u16) -> Self {
        Self {
            bpm,
            sample_rate,
            channels,
            state: PlayState::Stopped,
            position_ticks: 0,
            tick_remainder: 0.0,
        }
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.state = PlayState::Playing;
    }

    /// Stop playback.
    pub fn stop(&mut self) {
        self.state = PlayState::Stopped;
    }

    /// Reset position to zero without changing play state.
    pub fn reset(&mut self) {
        self.position_ticks = 0;
        self.tick_remainder = 0.0;
    }

    /// Current play state.
    pub fn state(&self) -> PlayState {
        self.state
    }

    /// Current position as a [`Beat`].
    pub fn position(&self) -> Beat {
        Beat::from_ticks(self.position_ticks)
    }

    /// Current BPM.
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Set a new BPM. Takes effect on the next `advance_by_frames` call.
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
    }

    /// Sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channel count.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Advance the transport by `num_frames` audio frames.
    ///
    /// Returns the `(from, to)` beat range covered by this advancement.
    /// The range is `[from, to)` — inclusive start, exclusive end.
    ///
    /// If the transport is stopped, returns `None`.
    pub fn advance_by_frames(&mut self, num_frames: u32) -> Option<(Beat, Beat)> {
        if self.state == PlayState::Stopped {
            return None;
        }

        let from = Beat::from_ticks(self.position_ticks);

        // How many ticks correspond to num_frames at current BPM?
        // ticks = (frames / sample_rate) * (bpm / 60) * TICKS_PER_BEAT
        let ticks_f64 = (num_frames as f64 / self.sample_rate as f64)
            * (self.bpm / 60.0)
            * TICKS_PER_BEAT as f64;

        let total = self.tick_remainder + ticks_f64;
        let whole_ticks = total.floor() as u64;
        self.tick_remainder = total - whole_ticks as f64;
        self.position_ticks += whole_ticks;

        let to = Beat::from_ticks(self.position_ticks);
        Some((from, to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let t = Transport::new(120.0, 44100, 2);
        assert_eq!(t.state(), PlayState::Stopped);
        assert_eq!(t.position(), Beat::ZERO);
        assert!((t.bpm() - 120.0).abs() < f64::EPSILON);
        assert_eq!(t.sample_rate(), 44100);
        assert_eq!(t.channels(), 2);
    }

    #[test]
    fn play_and_stop() {
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        assert_eq!(t.state(), PlayState::Playing);
        t.stop();
        assert_eq!(t.state(), PlayState::Stopped);
    }

    #[test]
    fn reset_position() {
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        t.advance_by_frames(22050);
        assert!(t.position().ticks() > 0);
        t.reset();
        assert_eq!(t.position(), Beat::ZERO);
    }

    #[test]
    fn advance_returns_none_when_stopped() {
        let mut t = Transport::new(120.0, 44100, 2);
        assert!(t.advance_by_frames(1024).is_none());
    }

    #[test]
    fn advance_one_beat_at_120_bpm() {
        // 120 BPM, 44100 Hz → 1 beat = 0.5s = 22050 frames
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        let (from, to) = t.advance_by_frames(22050).unwrap();
        assert_eq!(from, Beat::ZERO);
        assert_eq!(to.ticks(), TICKS_PER_BEAT);
    }

    #[test]
    fn advance_accumulates_correctly() {
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        // Advance in two halves
        t.advance_by_frames(11025);
        let (_, to) = t.advance_by_frames(11025).unwrap();
        assert_eq!(to.ticks(), TICKS_PER_BEAT);
    }

    #[test]
    fn advance_range_is_contiguous() {
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        let (from1, to1) = t.advance_by_frames(1024).unwrap();
        let (from2, _to2) = t.advance_by_frames(1024).unwrap();
        assert_eq!(from1, Beat::ZERO);
        assert_eq!(to1, from2);
    }

    #[test]
    fn fractional_tick_drift_test() {
        // Advance by a prime-ish frame count many times and verify
        // we don't drift from the expected position.
        let mut t = Transport::new(120.0, 44100, 2);
        t.play();
        let frames_per_advance: u32 = 997; // prime
        let advances = 10_000;
        for _ in 0..advances {
            t.advance_by_frames(frames_per_advance);
        }
        // Total frames = 997 * 10000 = 9_970_000
        // Total seconds = 9_970_000 / 44100 ≈ 226.0771...
        // Total beats = 226.0771... * (120/60) = 452.1541...
        // Total ticks = 452.1541... * 960 ≈ 434067.9...
        let expected_ticks = ((frames_per_advance as f64 * advances as f64 / 44100.0)
            * (120.0 / 60.0)
            * TICKS_PER_BEAT as f64)
            .floor() as u64;
        // Allow ±1 tick tolerance for rounding
        let actual = t.position().ticks();
        assert!(
            (actual as i64 - expected_ticks as i64).unsigned_abs() <= 1,
            "drift detected: expected ~{expected_ticks}, got {actual}"
        );
    }

    #[test]
    fn set_bpm_takes_effect() {
        let mut t = Transport::new(120.0, 44100, 2);
        t.set_bpm(60.0);
        t.play();
        // At 60 BPM, 44100 frames = 1 beat
        let (_, to) = t.advance_by_frames(44100).unwrap();
        assert_eq!(to.ticks(), TICKS_PER_BEAT);
    }

    #[test]
    fn determinism() {
        let run = || {
            let mut t = Transport::new(133.0, 48000, 2);
            t.play();
            for _ in 0..1000 {
                t.advance_by_frames(512);
            }
            t.position().ticks()
        };
        let first = run();
        for _ in 0..10 {
            assert_eq!(run(), first);
        }
    }
}
