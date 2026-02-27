//! Synthetic drum sound generators.
//!
//! Each generator produces a mono f32 buffer at the given sample rate.
//! Noise-based generators use a seeded `ChaCha8Rng` for determinism.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::sample::SampleData;
use super::SampleBank;

/// Generate a synthetic kick drum (~250ms).
///
/// Sine wave with exponential pitch sweep from 150 Hz down to 50 Hz,
/// combined with exponential amplitude decay.
pub fn generate_kick(sample_rate: u32) -> Vec<f32> {
    let duration_secs = 0.25;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let mut output = Vec::with_capacity(num_samples);
    let mut phase = 0.0_f64;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let norm = t / duration_secs;

        // Pitch sweep: 150 Hz → 50 Hz, exponential decay
        let freq = 50.0 + 100.0 * (-norm * 8.0).exp();

        // Amplitude envelope: fast exponential decay
        let amp = (-norm * 10.0).exp();

        phase += freq / sample_rate as f64;
        let sample = (phase * 2.0 * std::f64::consts::PI).sin() * amp;
        output.push(sample as f32);
    }

    output
}

/// Generate a synthetic snare drum (~200ms).
///
/// Sine body at 180 Hz with its own decay, plus white noise with independent
/// faster decay, mixed together.
pub fn generate_snare(sample_rate: u32, seed: u64) -> Vec<f32> {
    let duration_secs = 0.2;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = Vec::with_capacity(num_samples);
    let mut phase = 0.0_f64;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let norm = t / duration_secs;

        // Sine body
        let body_amp = (-norm * 15.0).exp();
        phase += 180.0 / sample_rate as f64;
        let body = (phase * 2.0 * std::f64::consts::PI).sin() * body_amp;

        // Noise component
        let noise_amp = (-norm * 12.0).exp();
        let noise: f64 = rng.gen_range(-1.0..1.0) * noise_amp;

        output.push((body * 0.5 + noise * 0.5) as f32);
    }

    output
}

/// Generate a synthetic hi-hat (~80ms).
///
/// High-frequency white noise with very fast exponential decay.
pub fn generate_hihat(sample_rate: u32, seed: u64) -> Vec<f32> {
    let duration_secs = 0.08;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = Vec::with_capacity(num_samples);

    // Simple one-pole high-pass filter state
    let mut prev_input = 0.0_f64;
    let mut prev_output = 0.0_f64;
    let cutoff = 0.85; // high-pass coefficient

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let norm = t / duration_secs;

        let amp = (-norm * 20.0).exp();
        let noise: f64 = rng.gen_range(-1.0..1.0);

        // One-pole high-pass: y[n] = alpha * (y[n-1] + x[n] - x[n-1])
        let filtered = cutoff * (prev_output + noise - prev_input);
        prev_input = noise;
        prev_output = filtered;

        output.push((filtered * amp) as f32);
    }

    output
}

/// Generate a synthetic clap (~150ms).
///
/// Three staggered noise micro-bursts followed by a bandpassed decay tail.
pub fn generate_clap(sample_rate: u32, seed: u64) -> Vec<f32> {
    let duration_secs = 0.15;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut output = vec![0.0f32; num_samples];

    // Three micro-bursts at 0ms, 15ms, 30ms — each ~10ms long
    let burst_offsets = [0.0, 0.015, 0.030];
    let burst_len_secs = 0.01;

    for &offset in &burst_offsets {
        let start = (offset * sample_rate as f64) as usize;
        let end = ((offset + burst_len_secs) * sample_rate as f64) as usize;
        for (i, sample) in output
            .iter_mut()
            .enumerate()
            .take(end.min(num_samples))
            .skip(start)
        {
            let local_t = (i - start) as f64 / (burst_len_secs * sample_rate as f64);
            let env = (-local_t * 15.0).exp();
            let noise: f64 = rng.gen_range(-1.0..1.0);
            *sample += (noise * env * 0.7) as f32;
        }
    }

    // Decay tail from ~40ms onwards
    let tail_start = (0.04 * sample_rate as f64) as usize;
    let mut bp_state = 0.0_f64;
    let bp_freq = 1200.0;
    let bp_q = 0.5;

    for (i, sample) in output
        .iter_mut()
        .enumerate()
        .take(num_samples)
        .skip(tail_start)
    {
        let t = (i - tail_start) as f64 / sample_rate as f64;
        let tail_amp = (-t * 18.0).exp();
        let noise: f64 = rng.gen_range(-1.0..1.0);

        // Simple resonant bandpass approximation
        bp_state += (noise - bp_state) * (bp_freq * bp_q / sample_rate as f64);
        *sample += (bp_state * tail_amp * 0.5) as f32;
    }

    output
}

/// Build a default drum kit with kick, snare, hi-hat, and clap.
///
/// All samples are generated synthetically at `sample_rate`. The `seed`
/// controls noise-based randomness for deterministic output.
pub fn build_default_kit(sample_rate: u32, seed: u64) -> SampleBank {
    let mut bank = SampleBank::new();

    bank.insert(
        "kick",
        SampleData::from_mono(generate_kick(sample_rate), sample_rate),
    );
    bank.insert(
        "snare",
        SampleData::from_mono(generate_snare(sample_rate, seed), sample_rate),
    );
    bank.insert(
        "hat",
        SampleData::from_mono(
            generate_hihat(sample_rate, seed.wrapping_add(1)),
            sample_rate,
        ),
    );
    bank.insert(
        "clap",
        SampleData::from_mono(
            generate_clap(sample_rate, seed.wrapping_add(2)),
            sample_rate,
        ),
    );

    bank
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 44100;
    const SEED: u64 = 42;

    #[test]
    fn kick_not_silent() {
        let kick = generate_kick(SR);
        assert!(!kick.is_empty());
        assert!(kick.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn kick_approximate_length() {
        let kick = generate_kick(SR);
        let expected = (SR as f64 * 0.25) as usize;
        assert_eq!(kick.len(), expected);
    }

    #[test]
    fn kick_starts_loud_ends_quiet() {
        let kick = generate_kick(SR);
        let first_quarter = &kick[..kick.len() / 4];
        let last_quarter = &kick[kick.len() * 3 / 4..];
        let first_rms: f32 =
            (first_quarter.iter().map(|s| s * s).sum::<f32>() / first_quarter.len() as f32).sqrt();
        let last_rms: f32 =
            (last_quarter.iter().map(|s| s * s).sum::<f32>() / last_quarter.len() as f32).sqrt();
        assert!(first_rms > last_rms * 2.0);
    }

    #[test]
    fn kick_peak_within_bounds() {
        let kick = generate_kick(SR);
        for &s in &kick {
            assert!(s >= -1.0 && s <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn snare_not_silent() {
        let snare = generate_snare(SR, SEED);
        assert!(!snare.is_empty());
        assert!(snare.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn snare_approximate_length() {
        let snare = generate_snare(SR, SEED);
        let expected = (SR as f64 * 0.2) as usize;
        assert_eq!(snare.len(), expected);
    }

    #[test]
    fn snare_starts_loud_ends_quiet() {
        let snare = generate_snare(SR, SEED);
        let first_quarter = &snare[..snare.len() / 4];
        let last_quarter = &snare[snare.len() * 3 / 4..];
        let first_rms: f32 =
            (first_quarter.iter().map(|s| s * s).sum::<f32>() / first_quarter.len() as f32).sqrt();
        let last_rms: f32 =
            (last_quarter.iter().map(|s| s * s).sum::<f32>() / last_quarter.len() as f32).sqrt();
        assert!(first_rms > last_rms * 2.0);
    }

    #[test]
    fn snare_peak_within_bounds() {
        let snare = generate_snare(SR, SEED);
        for &s in &snare {
            assert!(s >= -1.0 && s <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn hihat_not_silent() {
        let hat = generate_hihat(SR, SEED);
        assert!(!hat.is_empty());
        assert!(hat.iter().any(|&s| s.abs() > 0.001));
    }

    #[test]
    fn hihat_approximate_length() {
        let hat = generate_hihat(SR, SEED);
        let expected = (SR as f64 * 0.08) as usize;
        assert_eq!(hat.len(), expected);
    }

    #[test]
    fn hihat_peak_within_bounds() {
        let hat = generate_hihat(SR, SEED);
        for &s in &hat {
            assert!(s >= -1.0 && s <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn clap_not_silent() {
        let clap = generate_clap(SR, SEED);
        assert!(!clap.is_empty());
        assert!(clap.iter().any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn clap_approximate_length() {
        let clap = generate_clap(SR, SEED);
        let expected = (SR as f64 * 0.15) as usize;
        assert_eq!(clap.len(), expected);
    }

    #[test]
    fn clap_peak_within_bounds() {
        let clap = generate_clap(SR, SEED);
        for &s in &clap {
            assert!(s >= -1.0 && s <= 1.0, "sample out of bounds: {s}");
        }
    }

    #[test]
    fn determinism_same_seed() {
        let a = generate_snare(SR, SEED);
        let b = generate_snare(SR, SEED);
        assert_eq!(a, b, "same seed must produce identical output");
    }

    #[test]
    fn different_seeds_differ() {
        let a = generate_snare(SR, 1);
        let b = generate_snare(SR, 2);
        assert_ne!(a, b, "different seeds should produce different output");
    }

    #[test]
    fn build_default_kit_has_all_samples() {
        let bank = build_default_kit(SR, SEED);
        assert_eq!(bank.len(), 4);
        assert!(bank.get("kick").is_some());
        assert!(bank.get("snare").is_some());
        assert!(bank.get("hat").is_some());
        assert!(bank.get("clap").is_some());
    }

    #[test]
    fn build_default_kit_deterministic() {
        let a = build_default_kit(SR, SEED);
        let b = build_default_kit(SR, SEED);
        assert_eq!(
            a.get("kick").unwrap().samples(),
            b.get("kick").unwrap().samples()
        );
        assert_eq!(
            a.get("snare").unwrap().samples(),
            b.get("snare").unwrap().samples()
        );
        assert_eq!(
            a.get("hat").unwrap().samples(),
            b.get("hat").unwrap().samples()
        );
        assert_eq!(
            a.get("clap").unwrap().samples(),
            b.get("clap").unwrap().samples()
        );
    }
}
