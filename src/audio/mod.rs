//! Audio engine — dedicated thread, lock-free queue, double-buffer swap, master limiter.
//!
//! The audio engine owns the cpal output stream and communicates with it via a
//! lock-free ring buffer. The main thread sends [`AudioCommand`]s to the audio
//! thread, which drains them in its callback and fills the output buffer.

pub mod buffer;
pub mod callback;
pub mod command;
pub mod limiter;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{
    traits::{Producer, Split},
    HeapRb,
};

pub use buffer::DoubleBuffer;
pub use command::AudioCommand;
pub use limiter::Limiter;

use callback::AudioCallback;

/// Ring buffer capacity (number of commands).
const RING_BUFFER_CAPACITY: usize = 1024;

/// Audio engine errors.
#[derive(Debug)]
pub enum AudioError {
    /// No audio output device found.
    NoOutputDevice,
    /// Failed to query device configuration.
    DeviceConfig(String),
    /// Failed to build the audio stream.
    StreamBuild(String),
    /// Failed to start the audio stream.
    StreamPlay(String),
    /// Ring buffer is full — audio thread is not draining fast enough.
    BufferFull,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoOutputDevice => write!(f, "no audio output device found"),
            AudioError::DeviceConfig(e) => write!(f, "device config error: {e}"),
            AudioError::StreamBuild(e) => write!(f, "stream build error: {e}"),
            AudioError::StreamPlay(e) => write!(f, "stream play error: {e}"),
            AudioError::BufferFull => write!(f, "audio command ring buffer is full"),
        }
    }
}

impl std::error::Error for AudioError {}

/// The audio engine. Owns the cpal stream and ring buffer producer.
///
/// Created on the main thread, sends commands to the audio thread via the
/// lock-free ring buffer.
pub struct AudioEngine {
    stream: cpal::Stream,
    producer: ringbuf::HeapProd<AudioCommand>,
    sample_rate: u32,
    channels: u16,
}

impl AudioEngine {
    /// Create and start the audio engine with the default output device.
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;

        let config = device
            .default_output_config()
            .map_err(|e| AudioError::DeviceConfig(e.to_string()))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        Self::build_with_device(&device, sample_rate, channels)
    }

    /// Create the audio engine with a specific sample rate and channel count.
    ///
    /// Uses the default output device but overrides its configuration.
    pub fn with_config(sample_rate: u32, channels: u16) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;

        Self::build_with_device(&device, sample_rate, channels)
    }

    /// Internal builder: sets up ring buffer, callback, and stream.
    fn build_with_device(
        device: &cpal::Device,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, AudioError> {
        let rb = HeapRb::<AudioCommand>::new(RING_BUFFER_CAPACITY);
        let (producer, consumer) = rb.split();

        let mut audio_callback = AudioCallback::new(consumer, channels, sample_rate);

        let stream_config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let err_fn = |err: cpal::StreamError| {
            eprintln!("audio stream error: {err}");
        };

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    audio_callback.process(data);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamBuild(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamPlay(e.to_string()))?;

        Ok(Self {
            stream,
            producer,
            sample_rate,
            channels,
        })
    }

    /// Send pre-rendered audio samples to the audio thread.
    ///
    /// `samples` should contain interleaved channel data (e.g. L, R, L, R, ...).
    pub fn send_samples(&mut self, samples: Vec<f32>) -> Result<(), AudioError> {
        self.producer
            .try_push(AudioCommand::Samples(samples))
            .map_err(|_| AudioError::BufferFull)
    }

    /// Set master volume (clamped to 0.0..=1.0 on the audio thread).
    pub fn set_volume(&mut self, volume: f32) -> Result<(), AudioError> {
        self.producer
            .try_push(AudioCommand::SetVolume(volume))
            .map_err(|_| AudioError::BufferFull)
    }

    /// Stop playback and clear the audio buffer.
    pub fn stop(&mut self) -> Result<(), AudioError> {
        self.producer
            .try_push(AudioCommand::Stop)
            .map_err(|_| AudioError::BufferFull)
    }

    /// Get the sample rate of the audio stream.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of output channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Pause the audio stream.
    pub fn pause(&self) -> Result<(), AudioError> {
        self.stream
            .pause()
            .map_err(|e| AudioError::StreamPlay(e.to_string()))
    }

    /// Resume the audio stream.
    pub fn play(&self) -> Result<(), AudioError> {
        self.stream
            .play()
            .map_err(|e| AudioError::StreamPlay(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires audio device — run manually with `cargo test -- --ignored`
    fn test_audio_engine_creation() {
        let engine = AudioEngine::new();
        assert!(
            engine.is_ok(),
            "AudioEngine::new() failed: {:?}",
            engine.err()
        );
        let engine = engine.unwrap();
        assert!(engine.sample_rate() > 0);
        assert!(engine.channels() > 0);
    }

    #[test]
    #[ignore] // Requires audio device
    fn test_send_samples() {
        let mut engine = AudioEngine::new().expect("no audio device");
        let result = engine.send_samples(vec![0.0; 1024]);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore] // Requires audio device
    fn test_set_volume_and_stop() {
        let mut engine = AudioEngine::new().expect("no audio device");
        assert!(engine.set_volume(0.5).is_ok());
        assert!(engine.stop().is_ok());
    }

    #[test]
    #[ignore] // Requires audio device
    fn test_pause_and_play() {
        let engine = AudioEngine::new().expect("no audio device");
        assert!(engine.pause().is_ok());
        assert!(engine.play().is_ok());
    }

    #[test]
    fn test_audio_error_display() {
        assert_eq!(
            AudioError::NoOutputDevice.to_string(),
            "no audio output device found"
        );
        assert_eq!(
            AudioError::BufferFull.to_string(),
            "audio command ring buffer is full"
        );
        assert_eq!(
            AudioError::DeviceConfig("test".to_string()).to_string(),
            "device config error: test"
        );
    }

    #[test]
    fn test_ring_buffer_capacity() {
        assert_eq!(RING_BUFFER_CAPACITY, 1024);
    }
}
