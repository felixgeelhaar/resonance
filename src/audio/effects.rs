//! Master effects — Schroeder reverb and BPM-synced ping-pong delay.
//!
//! All `process_block` methods are zero-allocation: buffers are pre-allocated
//! in constructors and reused.

// ─── Comb Filter ──────────────────────────────────────────────────────────

struct CombFilter {
    buffer: Vec<f32>,
    write_pos: usize,
    feedback: f32,
}

impl CombFilter {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            write_pos: 0,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.write_pos];
        let output = delayed;
        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        output
    }
}

// ─── Allpass Filter ───────────────────────────────────────────────────────

struct AllpassFilter {
    buffer: Vec<f32>,
    write_pos: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            write_pos: 0,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.write_pos];
        let output = -input + delayed;
        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        output
    }
}

// ─── Reverb ───────────────────────────────────────────────────────────────

/// Schroeder reverb: 4 parallel comb filters feeding 2 series allpass filters.
pub struct Reverb {
    combs: [CombFilter; 4],
    allpasses: [AllpassFilter; 2],
    mix: f32,
}

/// Comb filter delay lengths tuned for 44100 Hz (prime-ish numbers).
const COMB_DELAYS_44100: [usize; 4] = [1557, 1617, 1491, 1422];
/// Allpass delay lengths tuned for 44100 Hz.
const ALLPASS_DELAYS_44100: [usize; 2] = [225, 556];
/// Default comb feedback.
const COMB_FEEDBACK: f32 = 0.84;
/// Default allpass feedback.
const ALLPASS_FEEDBACK: f32 = 0.5;

impl Reverb {
    /// Create a new reverb with delay lines scaled for the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        let scale = |base: usize| -> usize {
            ((base as f64) * (sample_rate as f64) / 44100.0).round() as usize
        };

        Self {
            combs: [
                CombFilter::new(scale(COMB_DELAYS_44100[0]).max(1), COMB_FEEDBACK),
                CombFilter::new(scale(COMB_DELAYS_44100[1]).max(1), COMB_FEEDBACK),
                CombFilter::new(scale(COMB_DELAYS_44100[2]).max(1), COMB_FEEDBACK),
                CombFilter::new(scale(COMB_DELAYS_44100[3]).max(1), COMB_FEEDBACK),
            ],
            allpasses: [
                AllpassFilter::new(scale(ALLPASS_DELAYS_44100[0]).max(1), ALLPASS_FEEDBACK),
                AllpassFilter::new(scale(ALLPASS_DELAYS_44100[1]).max(1), ALLPASS_FEEDBACK),
            ],
            mix: 0.0,
        }
    }

    /// Set wet/dry mix (0.0 = fully dry, 1.0 = fully wet).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Get current mix value.
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Set comb filter feedback (controls decay time). Clamped to 0.0–0.99.
    pub fn set_decay(&mut self, decay: f32) {
        let fb = decay.clamp(0.0, 0.99);
        for comb in &mut self.combs {
            comb.feedback = fb;
        }
    }

    /// Process a stereo-interleaved buffer in-place.
    ///
    /// For each stereo pair: average L+R to mono, run through reverb,
    /// blend wet/dry, write back to L+R.
    pub fn process_block(&mut self, buffer: &mut [f32]) {
        if self.mix <= 0.0 {
            return;
        }

        let dry = 1.0 - self.mix;
        let wet = self.mix;

        for frame in buffer.chunks_exact_mut(2) {
            let mono = (frame[0] + frame[1]) * 0.5;

            // Parallel comb filters, summed
            let mut comb_sum = 0.0f32;
            for comb in &mut self.combs {
                comb_sum += comb.process(mono);
            }
            comb_sum *= 0.25; // Normalize by number of combs

            // Series allpass filters for diffusion
            let mut ap_out = comb_sum;
            for ap in &mut self.allpasses {
                ap_out = ap.process(ap_out);
            }

            frame[0] = frame[0] * dry + ap_out * wet;
            frame[1] = frame[1] * dry + ap_out * wet;
        }
    }
}

// ─── Delay ────────────────────────────────────────────────────────────────

/// BPM-synced ping-pong stereo delay.
pub struct Delay {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
    sample_rate: u32,
    time_beats: f64,
}

/// Maximum delay buffer size: 2 seconds at any sample rate.
const MAX_DELAY_SECONDS: f64 = 2.0;

impl Delay {
    /// Create a new delay for the given sample rate.
    ///
    /// Defaults: time_beats=0.5 (eighth note), feedback=0.4, mix=0.0.
    pub fn new(sample_rate: u32) -> Self {
        let max_samples = (sample_rate as f64 * MAX_DELAY_SECONDS) as usize;
        Self {
            buffer_l: vec![0.0; max_samples],
            buffer_r: vec![0.0; max_samples],
            write_pos: 0,
            delay_samples: Self::calc_delay_samples(sample_rate, 120.0, 0.5),
            feedback: 0.4,
            mix: 0.0,
            sample_rate,
            time_beats: 0.5,
        }
    }

    fn calc_delay_samples(sample_rate: u32, bpm: f64, time_beats: f64) -> usize {
        let seconds = time_beats * 60.0 / bpm;
        let max = (sample_rate as f64 * MAX_DELAY_SECONDS) as usize;
        ((seconds * sample_rate as f64).round() as usize)
            .min(max)
            .max(1)
    }

    /// Update BPM, recalculating delay time.
    pub fn set_bpm(&mut self, bpm: f64) {
        self.delay_samples = Self::calc_delay_samples(self.sample_rate, bpm, self.time_beats);
    }

    /// Set wet/dry mix (0.0 = fully dry, 1.0 = fully wet).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Get current mix value.
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Set feedback amount (0.0–0.95).
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }

    /// Get current feedback value.
    pub fn feedback(&self) -> f32 {
        self.feedback
    }

    /// Set delay time in beats and recalculate samples at current BPM.
    pub fn set_time_beats(&mut self, time_beats: f64, bpm: f64) {
        self.time_beats = time_beats;
        self.delay_samples = Self::calc_delay_samples(self.sample_rate, bpm, time_beats);
    }

    /// Get delay length in samples.
    pub fn delay_samples(&self) -> usize {
        self.delay_samples
    }

    /// Process a stereo-interleaved buffer in-place (ping-pong delay).
    ///
    /// Left channel feeds right delay, right channel feeds left delay.
    pub fn process_block(&mut self, buffer: &mut [f32]) {
        if self.mix <= 0.0 {
            return;
        }

        let buf_len = self.buffer_l.len();
        let dry = 1.0 - self.mix;
        let wet = self.mix;

        for frame in buffer.chunks_exact_mut(2) {
            let read_pos = (self.write_pos + buf_len - self.delay_samples) % buf_len;

            let delayed_l = self.buffer_l[read_pos];
            let delayed_r = self.buffer_r[read_pos];

            // Ping-pong: left input → right delay, right input → left delay
            self.buffer_l[self.write_pos] = frame[1] + delayed_l * self.feedback;
            self.buffer_r[self.write_pos] = frame[0] + delayed_r * self.feedback;

            frame[0] = frame[0] * dry + delayed_l * wet;
            frame[1] = frame[1] * dry + delayed_r * wet;

            self.write_pos = (self.write_pos + 1) % buf_len;
        }
    }
}

// ─── MasterEffects ────────────────────────────────────────────────────────

/// Aggregation of all master effects: reverb then delay.
pub struct MasterEffects {
    pub reverb: Reverb,
    pub delay: Delay,
}

impl MasterEffects {
    /// Create master effects for the given sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            reverb: Reverb::new(sample_rate),
            delay: Delay::new(sample_rate),
        }
    }

    /// Process a stereo-interleaved buffer: reverb first, then delay.
    pub fn process_block(&mut self, buffer: &mut [f32]) {
        self.reverb.process_block(buffer);
        self.delay.process_block(buffer);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- CombFilter tests ---

    #[test]
    fn comb_filter_output_not_silent() {
        let mut comb = CombFilter::new(10, 0.8);
        // Feed some input
        let mut any_nonzero = false;
        for i in 0..20 {
            let out = comb.process(if i == 0 { 1.0 } else { 0.0 });
            if out.abs() > 1e-10 {
                any_nonzero = true;
            }
        }
        assert!(any_nonzero, "comb should produce non-silent output");
    }

    #[test]
    fn comb_filter_feedback_affects_decay() {
        // High feedback should sustain longer
        let mut high_fb = CombFilter::new(10, 0.9);
        let mut low_fb = CombFilter::new(10, 0.1);

        // Impulse
        high_fb.process(1.0);
        low_fb.process(1.0);

        // Run past two full delay cycles so feedback has a chance to accumulate
        for _ in 1..25 {
            high_fb.process(0.0);
            low_fb.process(0.0);
        }

        // Collect energy over several more samples
        let mut high_energy = 0.0f32;
        let mut low_energy = 0.0f32;
        for _ in 0..10 {
            high_energy += high_fb.process(0.0).abs();
            low_energy += low_fb.process(0.0).abs();
        }
        assert!(
            high_energy > low_energy,
            "high feedback {high_energy} should > low {low_energy}"
        );
    }

    #[test]
    fn comb_filter_deterministic() {
        let mut a = CombFilter::new(10, 0.8);
        let mut b = CombFilter::new(10, 0.8);

        for i in 0..30 {
            let input = if i < 5 { (i as f32) * 0.2 } else { 0.0 };
            let out_a = a.process(input);
            let out_b = b.process(input);
            assert!(
                (out_a - out_b).abs() < f32::EPSILON,
                "comb should be deterministic"
            );
        }
    }

    // --- AllpassFilter tests ---

    #[test]
    fn allpass_output_not_silent() {
        let mut ap = AllpassFilter::new(10, 0.5);
        let mut any_nonzero = false;
        for i in 0..20 {
            let out = ap.process(if i == 0 { 1.0 } else { 0.0 });
            if out.abs() > 1e-10 {
                any_nonzero = true;
            }
        }
        assert!(any_nonzero, "allpass should produce non-silent output");
    }

    #[test]
    fn allpass_preserves_energy() {
        let mut ap = AllpassFilter::new(10, 0.5);
        let mut input_energy = 0.0f64;
        let mut output_energy = 0.0f64;

        // Feed a short impulse and collect output over time
        for i in 0..100 {
            let input = if i == 0 { 1.0f32 } else { 0.0 };
            input_energy += (input as f64) * (input as f64);
            let out = ap.process(input);
            output_energy += (out as f64) * (out as f64);
        }

        // Allpass should roughly preserve energy (within tolerance)
        // Due to finite observation window, ratio may exceed 1.0
        let ratio = output_energy / input_energy;
        assert!(
            ratio > 0.3 && ratio < 3.0,
            "allpass energy ratio {ratio} should be roughly bounded"
        );
    }

    #[test]
    fn allpass_deterministic() {
        let mut a = AllpassFilter::new(10, 0.5);
        let mut b = AllpassFilter::new(10, 0.5);

        for i in 0..30 {
            let input = if i < 5 { (i as f32) * 0.2 } else { 0.0 };
            let out_a = a.process(input);
            let out_b = b.process(input);
            assert!(
                (out_a - out_b).abs() < f32::EPSILON,
                "allpass should be deterministic"
            );
        }
    }

    // --- Reverb tests ---

    #[test]
    fn reverb_silence_to_silence() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(0.5);
        let mut buffer = vec![0.0f32; 128];
        reverb.process_block(&mut buffer);
        for &s in &buffer {
            assert!(s.abs() < 1e-10, "reverb on silence should be silent");
        }
    }

    #[test]
    fn reverb_mix_zero_passthrough() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(0.0);

        let original: Vec<f32> = (0..64).map(|i| (i as f32) * 0.01).collect();
        let mut buffer = original.clone();
        reverb.process_block(&mut buffer);

        for (a, b) in buffer.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-10, "mix=0 should pass through unchanged");
        }
    }

    #[test]
    fn reverb_mix_one_fully_wet() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(1.0);

        // Feed an impulse
        let mut buffer = vec![0.0f32; 256];
        buffer[0] = 0.8;
        buffer[1] = 0.8;
        let original = buffer.clone();

        reverb.process_block(&mut buffer);

        // Output should differ from input (reverb processing)
        let any_different = buffer
            .iter()
            .zip(original.iter())
            .any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(any_different, "mix=1 should produce different output");
    }

    #[test]
    fn reverb_set_mix_clamps() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(2.0);
        assert!((reverb.mix() - 1.0).abs() < f32::EPSILON);
        reverb.set_mix(-1.0);
        assert!(reverb.mix().abs() < f32::EPSILON);
    }

    #[test]
    fn reverb_process_block_output() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(0.3);

        let mut buffer = vec![0.0f32; 4096];
        // Put an impulse at the start
        buffer[0] = 0.5;
        buffer[1] = 0.5;

        reverb.process_block(&mut buffer);

        // Should have tail energy later in the buffer
        let tail_energy: f32 = buffer[2000..].iter().map(|s| s * s).sum();
        assert!(
            tail_energy > 1e-10,
            "reverb should produce a tail: {tail_energy}"
        );
    }

    #[test]
    fn reverb_stereo_handling() {
        let mut reverb = Reverb::new(44100);
        reverb.set_mix(0.5);

        let mut buffer = vec![0.0f32; 64];
        buffer[0] = 0.8; // L
        buffer[1] = 0.2; // R

        reverb.process_block(&mut buffer);

        // Both channels should be affected (reverb mixes to mono internally)
        let l_energy: f32 = buffer.iter().step_by(2).map(|s| s * s).sum();
        let r_energy: f32 = buffer.iter().skip(1).step_by(2).map(|s| s * s).sum();
        assert!(l_energy > 0.0);
        assert!(r_energy > 0.0);
    }

    #[test]
    fn reverb_sample_rate_scaling() {
        let r_44100 = Reverb::new(44100);
        let r_48000 = Reverb::new(48000);

        // Delay lines should be scaled
        assert_ne!(r_44100.combs[0].buffer.len(), r_48000.combs[0].buffer.len());
    }

    // --- Delay tests ---

    #[test]
    fn delay_silence_to_silence() {
        let mut delay = Delay::new(44100);
        delay.set_mix(0.5);
        let mut buffer = vec![0.0f32; 128];
        delay.process_block(&mut buffer);
        for &s in &buffer {
            assert!(s.abs() < 1e-10, "delay on silence should be silent");
        }
    }

    #[test]
    fn delay_mix_zero_passthrough() {
        let mut delay = Delay::new(44100);
        delay.set_mix(0.0);

        let original: Vec<f32> = (0..64).map(|i| (i as f32) * 0.01).collect();
        let mut buffer = original.clone();
        delay.process_block(&mut buffer);

        for (a, b) in buffer.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-10, "mix=0 should pass through unchanged");
        }
    }

    #[test]
    fn delay_feedback_zero_single_echo() {
        let mut delay = Delay::new(44100);
        delay.set_mix(0.5);
        delay.set_feedback(0.0);
        delay.delay_samples = 10; // Short delay for test

        // Feed an impulse and collect output
        let mut buffer = vec![0.0f32; 100];
        buffer[0] = 1.0; // L
        buffer[1] = 1.0; // R

        delay.process_block(&mut buffer);

        // Should see one echo at delay_samples offset, then silence
        let echo_pos = 10 * 2; // stereo frames
        let has_echo = buffer[echo_pos..echo_pos + 2]
            .iter()
            .any(|s| s.abs() > 0.01);
        assert!(has_echo, "should have echo at delay offset");

        // Well past the echo, energy should die out (no feedback)
        let late_energy: f32 = buffer[60..].iter().map(|s| s * s).sum();
        assert!(
            late_energy < 0.1,
            "no feedback means energy dies: {late_energy}"
        );
    }

    #[test]
    fn delay_set_bpm_recalculates() {
        let mut delay = Delay::new(44100);
        let original = delay.delay_samples();

        delay.set_bpm(60.0);
        let at_60 = delay.delay_samples();

        delay.set_bpm(180.0);
        let at_180 = delay.delay_samples();

        assert_ne!(original, at_60);
        assert!(at_60 > at_180, "slower BPM = longer delay");
    }

    #[test]
    fn delay_ping_pong_lr() {
        let mut delay = Delay::new(44100);
        delay.set_mix(1.0);
        delay.set_feedback(0.0);
        delay.delay_samples = 5;

        // Feed only into L channel
        let mut buffer = vec![0.0f32; 40];
        buffer[0] = 1.0; // L
        buffer[1] = 0.0; // R

        delay.process_block(&mut buffer);

        // Ping-pong: L input should appear in R delay
        let r_has_echo = buffer.iter().skip(1).step_by(2).any(|s| s.abs() > 0.01);
        assert!(r_has_echo, "L input should echo to R in ping-pong");
    }

    #[test]
    fn delay_set_mix_clamps() {
        let mut delay = Delay::new(44100);
        delay.set_mix(2.0);
        assert!((delay.mix() - 1.0).abs() < f32::EPSILON);
        delay.set_mix(-1.0);
        assert!(delay.mix().abs() < f32::EPSILON);
    }

    #[test]
    fn delay_deterministic() {
        let mut a = Delay::new(44100);
        let mut b = Delay::new(44100);
        a.set_mix(0.5);
        b.set_mix(0.5);

        let input: Vec<f32> = (0..64).map(|i| ((i as f32) * 0.1).sin()).collect();
        let mut buf_a = input.clone();
        let mut buf_b = input.clone();

        a.process_block(&mut buf_a);
        b.process_block(&mut buf_b);

        for (a, b) in buf_a.iter().zip(buf_b.iter()) {
            assert!(
                (a - b).abs() < f32::EPSILON,
                "delay should be deterministic"
            );
        }
    }

    // --- MasterEffects tests ---

    #[test]
    fn master_effects_combined_processing() {
        let mut fx = MasterEffects::new(44100);
        fx.reverb.set_mix(0.3);
        fx.delay.set_mix(0.2);

        let mut buffer = vec![0.0f32; 256];
        buffer[0] = 0.8;
        buffer[1] = 0.8;
        let original = buffer.clone();

        fx.process_block(&mut buffer);

        let any_different = buffer
            .iter()
            .zip(original.iter())
            .any(|(a, b)| (a - b).abs() > 1e-6);
        assert!(any_different, "effects should change the signal");
    }

    #[test]
    fn master_effects_both_applied() {
        // Reverb adds tail, delay adds echoes — test that both contribute
        let mut fx = MasterEffects::new(44100);
        fx.reverb.set_mix(0.5);
        fx.delay.set_mix(0.0);

        let mut buf_reverb = vec![0.0f32; 4096];
        buf_reverb[0] = 0.5;
        buf_reverb[1] = 0.5;
        fx.process_block(&mut buf_reverb);

        // Reset and test delay only — use fast BPM so delay fits in buffer
        let mut fx2 = MasterEffects::new(44100);
        fx2.reverb.set_mix(0.0);
        fx2.delay.set_mix(0.5);
        fx2.delay.set_bpm(300.0); // Short delay at high BPM

        let mut buf_delay = vec![0.0f32; 44100]; // 0.5s buffer
        buf_delay[0] = 0.5;
        buf_delay[1] = 0.5;
        fx2.process_block(&mut buf_delay);

        // Both should have non-trivial energy in the tail
        let reverb_tail: f32 = buf_reverb[2000..].iter().map(|s| s * s).sum();
        let delay_tail: f32 = buf_delay[2000..].iter().map(|s| s * s).sum();

        assert!(reverb_tail > 1e-10, "reverb should have tail");
        assert!(delay_tail > 1e-10, "delay should have tail");
    }
}
