//! State Variable Filter (SVF) — LP/HP/BP with cutoff and resonance.

use std::f64::consts::PI;

/// Filter mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
}

/// State Variable Filter with LP/HP/BP modes, cutoff (Hz) and resonance (Q).
pub struct SvfFilter {
    low: f64,
    band: f64,
    mode: FilterMode,
    cutoff: f64,
    resonance: f64,
    sample_rate: f64,
}

impl SvfFilter {
    /// Create a new SVF filter.
    pub fn new(mode: FilterMode, cutoff: f64, resonance: f64, sample_rate: f64) -> Self {
        Self {
            low: 0.0,
            band: 0.0,
            mode,
            cutoff: cutoff.clamp(20.0, 20000.0),
            resonance: resonance.clamp(0.5, 20.0),
            sample_rate,
        }
    }

    /// Set cutoff frequency in Hz.
    pub fn set_cutoff(&mut self, cutoff: f64) {
        self.cutoff = cutoff.clamp(20.0, 20000.0);
    }

    /// Set resonance (Q factor).
    pub fn set_resonance(&mut self, resonance: f64) {
        self.resonance = resonance.clamp(0.5, 20.0);
    }

    /// Process a single sample through the filter.
    pub fn process(&mut self, input: f64) -> f64 {
        // Clamp f to prevent instability at high cutoff/sample_rate ratios
        let f = (2.0 * (PI * self.cutoff / self.sample_rate).sin()).min(0.99);
        let q = 1.0 / self.resonance;

        let high = input - self.low - q * self.band;
        self.band += f * high;
        self.low += f * self.band;

        // Clamp state to prevent runaway
        self.low = self.low.clamp(-10.0, 10.0);
        self.band = self.band.clamp(-10.0, 10.0);

        match self.mode {
            FilterMode::LowPass => self.low,
            FilterMode::HighPass => high,
            FilterMode::BandPass => self.band,
        }
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpass_passes_dc() {
        let mut f = SvfFilter::new(FilterMode::LowPass, 1000.0, 0.707, 44100.0);
        // Feed constant signal — LP should pass it
        for _ in 0..1000 {
            f.process(1.0);
        }
        let out = f.process(1.0);
        assert!((out - 1.0).abs() < 0.01, "LP should pass DC: {out}");
    }

    #[test]
    fn highpass_blocks_dc() {
        let mut f = SvfFilter::new(FilterMode::HighPass, 1000.0, 0.707, 44100.0);
        for _ in 0..1000 {
            f.process(1.0);
        }
        let out = f.process(1.0);
        assert!(out.abs() < 0.01, "HP should block DC: {out}");
    }

    #[test]
    fn bandpass_blocks_dc() {
        let mut f = SvfFilter::new(FilterMode::BandPass, 1000.0, 0.707, 44100.0);
        for _ in 0..1000 {
            f.process(1.0);
        }
        let out = f.process(1.0);
        assert!(out.abs() < 0.01, "BP should block DC: {out}");
    }

    #[test]
    fn stability_at_extremes() {
        // Very high cutoff
        let mut f = SvfFilter::new(FilterMode::LowPass, 20000.0, 20.0, 44100.0);
        for i in 0..10000 {
            let input = (i as f64 * 0.1).sin();
            let out = f.process(input);
            assert!(out.is_finite(), "output not finite at sample {i}");
        }
        // Very low cutoff
        let mut f2 = SvfFilter::new(FilterMode::LowPass, 20.0, 0.5, 44100.0);
        for i in 0..10000 {
            let input = (i as f64 * 0.1).sin();
            let out = f2.process(input);
            assert!(out.is_finite(), "output not finite at sample {i}");
        }
    }

    #[test]
    fn determinism() {
        let run = || {
            let mut f = SvfFilter::new(FilterMode::LowPass, 500.0, 2.0, 44100.0);
            (0..100)
                .map(|i| f.process((i as f64 * 0.3).sin()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn reset_clears_state() {
        let mut f = SvfFilter::new(FilterMode::LowPass, 1000.0, 1.0, 44100.0);
        for i in 0..100 {
            f.process((i as f64 * 0.5).sin());
        }
        f.reset();
        assert!((f.low).abs() < 1e-10);
        assert!((f.band).abs() < 1e-10);
    }
}
