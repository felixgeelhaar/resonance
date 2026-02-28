//! Performance intents — quantized, immediate actions for live performance.
//!
//! Performance intents are queued and fire on beat boundaries. They cover
//! macro adjustments, layer toggles, section jumps, and tempo changes.

use crate::event::Beat;

/// A performance intent — an action that fires on a beat boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceIntent {
    /// Set a macro to an absolute value.
    SetMacro { name: String, value: f64 },
    /// Adjust a macro by a relative delta.
    AdjustMacro { name: String, delta: f64 },
    /// Toggle a layer on/off.
    ToggleLayer { name: String },
    /// Jump to a named section.
    JumpToSection { name: String },
    /// Change tempo.
    SetTempo(f64),
}

/// Processes and schedules performance intents with beat quantization.
#[derive(Debug, Clone)]
pub struct IntentProcessor {
    pending: Vec<(PerformanceIntent, Beat)>,
    quantize_beats: u32,
}

impl IntentProcessor {
    /// Create a new intent processor.
    ///
    /// `quantize_beats` determines the quantization grid:
    /// - 1 = fire on next beat boundary
    /// - 4 = fire on next bar boundary (in 4/4 time)
    pub fn new(quantize_beats: u32) -> Self {
        Self {
            pending: Vec::new(),
            quantize_beats: quantize_beats.max(1),
        }
    }

    /// Queue an intent to fire at the next quantized boundary after `current_position`.
    pub fn queue(&mut self, intent: PerformanceIntent, current_position: Beat) {
        let fire_at = self.next_quantized_boundary(current_position);
        self.pending.push((intent, fire_at));
    }

    /// Drain all intents that should fire at or before the given position.
    pub fn drain_ready(&mut self, position: Beat) -> Vec<PerformanceIntent> {
        let mut ready = Vec::new();
        self.pending.retain(|(intent, fire_at)| {
            if position >= *fire_at {
                ready.push(intent.clone());
                false
            } else {
                true
            }
        });
        ready
    }

    /// Number of pending intents.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending intents.
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Get the quantization grid size in beats.
    pub fn quantize_beats(&self) -> u32 {
        self.quantize_beats
    }

    /// Set the quantization grid size in beats.
    pub fn set_quantize_beats(&mut self, beats: u32) {
        self.quantize_beats = beats.max(1);
    }

    /// Calculate the next quantized boundary at or after the given position.
    fn next_quantized_boundary(&self, pos: Beat) -> Beat {
        let ticks_per_quantum = self.quantize_beats as u64 * crate::event::beat::TICKS_PER_BEAT;
        let current_ticks = pos.ticks();
        let quantum_number = current_ticks / ticks_per_quantum;
        let quantum_start = quantum_number * ticks_per_quantum;

        if quantum_start == current_ticks {
            // Already on boundary — fire at next one
            Beat::from_ticks(quantum_start + ticks_per_quantum)
        } else {
            Beat::from_ticks((quantum_number + 1) * ticks_per_quantum)
        }
    }
}

impl Default for IntentProcessor {
    fn default() -> Self {
        Self::new(1) // Default: quantize to beats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_and_drain() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(
            PerformanceIntent::SetMacro {
                name: "filter".to_string(),
                value: 0.8,
            },
            Beat::from_beats(0),
        );
        assert_eq!(proc.pending_count(), 1);

        // Before fire time — nothing ready
        let ready = proc.drain_ready(Beat::from_beats_f64(0.5));
        assert!(ready.is_empty());
        assert_eq!(proc.pending_count(), 1);

        // At fire time (beat 1)
        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 1);
        assert_eq!(proc.pending_count(), 0);
    }

    #[test]
    fn quantization_to_beats() {
        let mut proc = IntentProcessor::new(1);
        // Queue at beat 0.5 → should fire at beat 1
        proc.queue(
            PerformanceIntent::SetTempo(140.0),
            Beat::from_beats_f64(0.5),
        );

        // At beat 0.9 — not ready
        assert!(proc.drain_ready(Beat::from_beats_f64(0.9)).is_empty());

        // At beat 1 — ready
        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], PerformanceIntent::SetTempo(140.0));
    }

    #[test]
    fn quantization_to_bars() {
        let mut proc = IntentProcessor::new(4); // 4-beat quantization
        proc.queue(
            PerformanceIntent::JumpToSection {
                name: "chorus".to_string(),
            },
            Beat::from_beats(2),
        );

        // Should fire at beat 4 (next bar boundary)
        assert!(proc.drain_ready(Beat::from_beats(3)).is_empty());
        let ready = proc.drain_ready(Beat::from_beats(4));
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn multiple_intents_drain_together() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(
            PerformanceIntent::SetMacro {
                name: "a".to_string(),
                value: 0.5,
            },
            Beat::ZERO,
        );
        proc.queue(
            PerformanceIntent::SetMacro {
                name: "b".to_string(),
                value: 0.3,
            },
            Beat::ZERO,
        );
        assert_eq!(proc.pending_count(), 2);

        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 2);
        assert_eq!(proc.pending_count(), 0);
    }

    #[test]
    fn intents_drain_in_order() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(
            PerformanceIntent::SetMacro {
                name: "first".to_string(),
                value: 0.1,
            },
            Beat::ZERO,
        );
        proc.queue(
            PerformanceIntent::AdjustMacro {
                name: "second".to_string(),
                delta: 0.05,
            },
            Beat::ZERO,
        );

        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 2);
        match &ready[0] {
            PerformanceIntent::SetMacro { name, .. } => assert_eq!(name, "first"),
            _ => panic!("expected SetMacro"),
        }
    }

    #[test]
    fn staggered_intents() {
        let mut proc = IntentProcessor::new(1);
        // Intent at beat 0 → fires at beat 1
        proc.queue(PerformanceIntent::SetTempo(120.0), Beat::ZERO);
        // Intent at beat 1 → fires at beat 2
        proc.queue(PerformanceIntent::SetTempo(140.0), Beat::from_beats(1));

        // At beat 1: only first fires
        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], PerformanceIntent::SetTempo(120.0));

        // At beat 2: second fires
        let ready = proc.drain_ready(Beat::from_beats(2));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], PerformanceIntent::SetTempo(140.0));
    }

    #[test]
    fn clear_removes_all_pending() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(PerformanceIntent::SetTempo(120.0), Beat::ZERO);
        proc.queue(PerformanceIntent::SetTempo(140.0), Beat::ZERO);
        assert_eq!(proc.pending_count(), 2);

        proc.clear();
        assert_eq!(proc.pending_count(), 0);
    }

    #[test]
    fn toggle_layer_intent() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(
            PerformanceIntent::ToggleLayer {
                name: "reverb".to_string(),
            },
            Beat::from_beats(3),
        );

        let ready = proc.drain_ready(Beat::from_beats(4));
        assert_eq!(ready.len(), 1);
        assert_eq!(
            ready[0],
            PerformanceIntent::ToggleLayer {
                name: "reverb".to_string()
            }
        );
    }

    #[test]
    fn adjust_macro_intent() {
        let mut proc = IntentProcessor::new(1);
        proc.queue(
            PerformanceIntent::AdjustMacro {
                name: "filter".to_string(),
                delta: -0.1,
            },
            Beat::ZERO,
        );

        let ready = proc.drain_ready(Beat::from_beats(1));
        assert_eq!(ready.len(), 1);
        assert_eq!(
            ready[0],
            PerformanceIntent::AdjustMacro {
                name: "filter".to_string(),
                delta: -0.1,
            }
        );
    }

    #[test]
    fn on_boundary_fires_at_next() {
        let mut proc = IntentProcessor::new(1);
        // Queue exactly at beat 1 boundary → should fire at beat 2
        proc.queue(PerformanceIntent::SetTempo(130.0), Beat::from_beats(1));

        assert!(proc.drain_ready(Beat::from_beats(1)).is_empty());
        let ready = proc.drain_ready(Beat::from_beats(2));
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn set_quantize_beats() {
        let mut proc = IntentProcessor::new(1);
        assert_eq!(proc.quantize_beats(), 1);
        proc.set_quantize_beats(4);
        assert_eq!(proc.quantize_beats(), 4);
    }

    #[test]
    fn default_quantizes_to_beats() {
        let proc = IntentProcessor::default();
        assert_eq!(proc.quantize_beats(), 1);
    }
}
