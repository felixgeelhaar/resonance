//! Audio callback — runs on the cpal audio thread.
//!
//! Drains commands from the ring buffer, fills the output with samples,
//! applies volume and the master limiter.

use ringbuf::traits::Consumer;
use ringbuf::HeapCons;

use super::command::AudioCommand;
use super::limiter::Limiter;

/// Threshold (in samples) at which consumed samples are compacted.
/// When `read_pos` exceeds this, we shift remaining data to the front.
const COMPACT_THRESHOLD: usize = 8192;

/// State that lives on the audio thread. Accessed only from the cpal callback.
pub struct AudioCallback {
    consumer: HeapCons<AudioCommand>,
    playback_buffer: Vec<f32>,
    read_pos: usize,
    volume: f32,
    limiter: Limiter,
    channels: u16,
    sample_rate: u32,
}

impl AudioCallback {
    /// Create a new audio callback with the given ring buffer consumer.
    pub fn new(consumer: HeapCons<AudioCommand>, channels: u16, sample_rate: u32) -> Self {
        Self {
            consumer,
            playback_buffer: Vec::with_capacity(sample_rate as usize * channels as usize),
            read_pos: 0,
            volume: 1.0,
            limiter: Limiter::default(),
            channels,
            sample_rate,
        }
    }

    /// Called by cpal for each audio frame. Fills `output` with samples.
    pub fn process(&mut self, output: &mut [f32]) {
        // 1. Drain all pending commands from the ring buffer.
        while let Some(cmd) = self.consumer.try_pop() {
            match cmd {
                AudioCommand::Samples(data) => {
                    self.playback_buffer.extend_from_slice(&data);
                }
                AudioCommand::SetVolume(v) => {
                    self.volume = v.clamp(0.0, 1.0);
                }
                AudioCommand::Stop => {
                    self.playback_buffer.clear();
                    self.read_pos = 0;
                }
            }
        }

        // 2. Fill output buffer from playback buffer, applying volume.
        let available = self.playback_buffer.len() - self.read_pos;
        let copy_len = output.len().min(available);

        for (out, &src) in output[..copy_len]
            .iter_mut()
            .zip(&self.playback_buffer[self.read_pos..self.read_pos + copy_len])
        {
            *out = src * self.volume;
        }
        self.read_pos += copy_len;

        // Fill remainder with silence on underrun.
        for sample in output[copy_len..].iter_mut() {
            *sample = 0.0;
        }

        // 3. Apply master limiter.
        self.limiter.process_block(output);

        // 4. Compact playback buffer when enough has been consumed.
        if self.read_pos >= COMPACT_THRESHOLD {
            self.playback_buffer.drain(..self.read_pos);
            self.read_pos = 0;
        }
    }

    /// Returns the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the channel count.
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::{
        traits::{Producer, Split},
        HeapRb,
    };

    /// Helper: create a callback and its producer for testing.
    fn setup(capacity: usize) -> (ringbuf::HeapProd<AudioCommand>, AudioCallback) {
        let rb = HeapRb::<AudioCommand>::new(capacity);
        let (prod, cons) = rb.split();
        let callback = AudioCallback::new(cons, 2, 44100);
        (prod, callback)
    }

    #[test]
    fn test_callback_silence_on_empty() {
        let (_prod, mut callback) = setup(16);
        let mut output = vec![999.0f32; 64];
        callback.process(&mut output);

        for &sample in &output {
            assert_eq!(sample, 0.0);
        }
    }

    #[test]
    fn test_callback_plays_samples() {
        let (mut prod, mut callback) = setup(16);
        let samples = vec![0.1, 0.2, 0.3, 0.4];

        prod.try_push(AudioCommand::Samples(samples.clone()))
            .unwrap();

        let mut output = vec![0.0f32; 4];
        callback.process(&mut output);

        for (out, expected) in output.iter().zip(samples.iter()) {
            assert!(
                (out - expected).abs() < 1e-6,
                "expected {expected}, got {out}"
            );
        }
    }

    #[test]
    fn test_callback_applies_volume() {
        let (mut prod, mut callback) = setup(16);
        let samples = vec![0.4, 0.8, -0.4, -0.8];

        prod.try_push(AudioCommand::SetVolume(0.5)).unwrap();
        prod.try_push(AudioCommand::Samples(samples)).unwrap();

        let mut output = vec![0.0f32; 4];
        callback.process(&mut output);

        let expected = [0.2, 0.4, -0.2, -0.4];
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-6, "expected {exp}, got {out}");
        }
    }

    #[test]
    fn test_callback_stop_clears() {
        let (mut prod, mut callback) = setup(16);

        prod.try_push(AudioCommand::Samples(vec![0.5; 64])).unwrap();
        prod.try_push(AudioCommand::Stop).unwrap();

        let mut output = vec![999.0f32; 32];
        callback.process(&mut output);

        for &sample in &output {
            assert_eq!(sample, 0.0);
        }
    }

    #[test]
    fn test_callback_underrun_fills_silence() {
        let (mut prod, mut callback) = setup(16);

        // Send only 4 samples but request 8.
        prod.try_push(AudioCommand::Samples(vec![0.5, 0.6, 0.7, 0.8]))
            .unwrap();

        let mut output = vec![999.0f32; 8];
        callback.process(&mut output);

        // First 4 should have data.
        assert!((output[0] - 0.5).abs() < 1e-6);
        assert!((output[1] - 0.6).abs() < 1e-6);
        assert!((output[2] - 0.7).abs() < 1e-6);
        assert!((output[3] - 0.8).abs() < 1e-6);

        // Remaining should be silence.
        for &sample in &output[4..] {
            assert_eq!(sample, 0.0);
        }
    }

    #[test]
    fn test_callback_limiter_applied() {
        let (mut prod, mut callback) = setup(16);

        // Send samples that exceed 1.0 — limiter ceiling is 0.95.
        prod.try_push(AudioCommand::Samples(vec![2.0, -2.0, 0.5, -0.5]))
            .unwrap();

        let mut output = vec![0.0f32; 4];
        callback.process(&mut output);

        assert!((output[0] - 0.95).abs() < 1e-6);
        assert!((output[1] - (-0.95)).abs() < 1e-6);
        assert!((output[2] - 0.5).abs() < 1e-6);
        assert!((output[3] - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_callback_multiple_sample_commands() {
        let (mut prod, mut callback) = setup(16);

        prod.try_push(AudioCommand::Samples(vec![0.1, 0.2]))
            .unwrap();
        prod.try_push(AudioCommand::Samples(vec![0.3, 0.4]))
            .unwrap();

        let mut output = vec![0.0f32; 4];
        callback.process(&mut output);

        let expected = [0.1, 0.2, 0.3, 0.4];
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-6);
        }
    }

    #[test]
    fn test_callback_volume_clamps_to_range() {
        let (mut prod, mut callback) = setup(16);

        prod.try_push(AudioCommand::SetVolume(1.5)).unwrap();
        prod.try_push(AudioCommand::Samples(vec![0.8])).unwrap();

        let mut output = vec![0.0f32; 1];
        callback.process(&mut output);

        // Volume clamped to 1.0, so output = 0.8 * 1.0 = 0.8
        assert!((output[0] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_callback_persists_across_calls() {
        let (mut prod, mut callback) = setup(16);

        // Send 8 samples.
        prod.try_push(AudioCommand::Samples(vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8,
        ]))
        .unwrap();

        // Read first 4.
        let mut output1 = vec![0.0f32; 4];
        callback.process(&mut output1);
        assert!((output1[0] - 0.1).abs() < 1e-6);
        assert!((output1[3] - 0.4).abs() < 1e-6);

        // Read next 4.
        let mut output2 = vec![0.0f32; 4];
        callback.process(&mut output2);
        assert!((output2[0] - 0.5).abs() < 1e-6);
        assert!((output2[3] - 0.8).abs() < 1e-6);
    }
}
