//! Quantized transition manager — ensures state changes land on bar boundaries.

use crate::event::beat::{Beat, DEFAULT_BEATS_PER_BAR, TICKS_PER_BEAT};

/// Manages quantized transitions, ensuring changes align to bar boundaries.
#[derive(Debug, Clone)]
pub struct QuantizedTransitionManager {
    beats_per_bar: u32,
}

impl QuantizedTransitionManager {
    /// Create a new manager with the given time signature.
    pub fn new(beats_per_bar: u32) -> Self {
        Self { beats_per_bar }
    }

    /// Calculate the next bar boundary at or after the given position.
    pub fn next_bar_boundary(&self, current: Beat) -> Beat {
        let ticks_per_bar = self.beats_per_bar as u64 * TICKS_PER_BEAT;
        let current_ticks = current.ticks();
        let bar_number = current_ticks / ticks_per_bar;
        let bar_start = bar_number * ticks_per_bar;

        if bar_start == current_ticks {
            // Already on a bar boundary — return the next one
            Beat::from_ticks(bar_start + ticks_per_bar)
        } else {
            Beat::from_ticks((bar_number + 1) * ticks_per_bar)
        }
    }

    /// Check if a position is exactly on a bar boundary.
    pub fn is_on_bar_boundary(&self, pos: Beat) -> bool {
        let ticks_per_bar = self.beats_per_bar as u64 * TICKS_PER_BEAT;
        pos.ticks() % ticks_per_bar == 0
    }

    /// Get the bar number for a given position (0-indexed).
    pub fn bar_number(&self, pos: Beat) -> u64 {
        let ticks_per_bar = self.beats_per_bar as u64 * TICKS_PER_BEAT;
        pos.ticks() / ticks_per_bar
    }
}

impl Default for QuantizedTransitionManager {
    fn default() -> Self {
        Self::new(DEFAULT_BEATS_PER_BAR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_bar_from_zero() {
        let mgr = QuantizedTransitionManager::default();
        // At beat 0 (bar boundary), next bar is bar 1
        let next = mgr.next_bar_boundary(Beat::ZERO);
        assert_eq!(next, Beat::from_bars(1));
    }

    #[test]
    fn next_bar_from_mid_bar() {
        let mgr = QuantizedTransitionManager::default();
        // At beat 2 (mid bar 0), next bar boundary is bar 1
        let next = mgr.next_bar_boundary(Beat::from_beats(2));
        assert_eq!(next, Beat::from_bars(1));
    }

    #[test]
    fn next_bar_from_bar_boundary() {
        let mgr = QuantizedTransitionManager::default();
        // At bar 1 start (beat 4), next is bar 2 (beat 8)
        let next = mgr.next_bar_boundary(Beat::from_bars(1));
        assert_eq!(next, Beat::from_bars(2));
    }

    #[test]
    fn next_bar_from_just_before_boundary() {
        let mgr = QuantizedTransitionManager::default();
        // Just before bar 1 (beat 3.99...)
        let pos = Beat::from_ticks(4 * TICKS_PER_BEAT - 1);
        let next = mgr.next_bar_boundary(pos);
        assert_eq!(next, Beat::from_bars(1));
    }

    #[test]
    fn is_on_bar_boundary_true() {
        let mgr = QuantizedTransitionManager::default();
        assert!(mgr.is_on_bar_boundary(Beat::ZERO));
        assert!(mgr.is_on_bar_boundary(Beat::from_bars(1)));
        assert!(mgr.is_on_bar_boundary(Beat::from_bars(5)));
    }

    #[test]
    fn is_on_bar_boundary_false() {
        let mgr = QuantizedTransitionManager::default();
        assert!(!mgr.is_on_bar_boundary(Beat::from_beats(1)));
        assert!(!mgr.is_on_bar_boundary(Beat::from_beats(3)));
    }

    #[test]
    fn bar_number_calculation() {
        let mgr = QuantizedTransitionManager::default();
        assert_eq!(mgr.bar_number(Beat::ZERO), 0);
        assert_eq!(mgr.bar_number(Beat::from_beats(3)), 0);
        assert_eq!(mgr.bar_number(Beat::from_bars(1)), 1);
        assert_eq!(mgr.bar_number(Beat::from_bars(3)), 3);
    }

    #[test]
    fn custom_time_signature() {
        let mgr = QuantizedTransitionManager::new(3); // 3/4 time
        let next = mgr.next_bar_boundary(Beat::from_beats(1));
        assert_eq!(next, Beat::from_beats(3)); // 3 beats per bar
    }
}
