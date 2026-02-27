//! Sample data type — WAV loading, mono conversion, and linear-interpolation resampling.

use std::io::{Read, Seek};

/// Errors that can occur when loading or converting samples.
#[derive(Debug)]
pub enum SampleError {
    /// WAV decoding or I/O error.
    Wav(hound::Error),
    /// The WAV file contains no samples.
    Empty,
    /// Unsupported bit depth or format.
    UnsupportedFormat(String),
}

impl std::fmt::Display for SampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleError::Wav(e) => write!(f, "WAV error: {e}"),
            SampleError::Empty => write!(f, "WAV file contains no samples"),
            SampleError::UnsupportedFormat(s) => write!(f, "unsupported format: {s}"),
        }
    }
}

impl std::error::Error for SampleError {}

impl From<hound::Error> for SampleError {
    fn from(e: hound::Error) -> Self {
        SampleError::Wav(e)
    }
}

/// A mono audio sample buffer at a known sample rate.
#[derive(Debug, Clone)]
pub struct SampleData {
    samples: Vec<f32>,
    sample_rate: u32,
}

impl SampleData {
    /// Create from raw mono f32 samples.
    pub fn from_mono(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
        }
    }

    /// Load a WAV file from a reader, converting to mono f32 at `target_sample_rate`.
    ///
    /// Supports 16-bit integer and 32-bit float WAV formats. Multi-channel files
    /// are mixed down to mono by averaging channels. If the source sample rate
    /// differs from `target_sample_rate`, linear interpolation resampling is applied.
    pub fn from_wav<R: Read + Seek>(
        reader: R,
        target_sample_rate: u32,
    ) -> Result<Self, SampleError> {
        let wav = hound::WavReader::new(reader)?;
        let spec = wav.spec();
        let channels = spec.channels as usize;
        let source_rate = spec.sample_rate;

        let raw_samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_val = (1u32 << (bits - 1)) as f32;
                wav.into_samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max_val))
                    .collect::<Result<Vec<f32>, _>>()?
            }
            hound::SampleFormat::Float => {
                wav.into_samples::<f32>().collect::<Result<Vec<f32>, _>>()?
            }
        };

        if raw_samples.is_empty() {
            return Err(SampleError::Empty);
        }

        // Mix down to mono by averaging channels.
        let mono: Vec<f32> = raw_samples
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect();

        // Resample if rates differ.
        let resampled = if source_rate == target_sample_rate {
            mono
        } else {
            resample_linear(&mono, source_rate, target_sample_rate)
        };

        Ok(Self {
            samples: resampled,
            sample_rate: target_sample_rate,
        })
    }

    /// The mono sample buffer.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the sample buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Linear-interpolation resampling from `source_rate` to `target_rate`.
fn resample_linear(input: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    if input.len() == 1 {
        return vec![input[0]];
    }

    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = ((input.len() as f64 / ratio).ceil()) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;

        let sample = if idx + 1 < input.len() {
            input[idx] * (1.0 - frac) + input[idx + 1] * frac
        } else {
            input[idx.min(input.len() - 1)]
        };
        output.push(sample);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn from_mono_stores_samples() {
        let data = vec![0.1, 0.2, 0.3];
        let sd = SampleData::from_mono(data.clone(), 44100);
        assert_eq!(sd.samples(), &data[..]);
        assert_eq!(sd.sample_rate(), 44100);
    }

    #[test]
    fn len_and_is_empty() {
        let empty = SampleData::from_mono(vec![], 44100);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());

        let nonempty = SampleData::from_mono(vec![1.0], 44100);
        assert_eq!(nonempty.len(), 1);
        assert!(!nonempty.is_empty());
    }

    /// Helper: write a mono 16-bit WAV to an in-memory buffer.
    fn write_wav_16bit(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut buf, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
        buf.into_inner()
    }

    /// Helper: write a mono 32-bit float WAV to an in-memory buffer.
    fn write_wav_f32(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::new(&mut buf, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
        buf.into_inner()
    }

    #[test]
    fn from_wav_mono_16bit() {
        let wav_data = write_wav_16bit(&[0, 16384, -16384], 44100, 1);
        let sd = SampleData::from_wav(Cursor::new(wav_data), 44100).unwrap();
        assert_eq!(sd.len(), 3);
        assert_eq!(sd.sample_rate(), 44100);
        // 16384 / 32768 = 0.5
        assert!((sd.samples()[0]).abs() < 1e-6);
        assert!((sd.samples()[1] - 0.5).abs() < 1e-3);
        assert!((sd.samples()[2] + 0.5).abs() < 1e-3);
    }

    #[test]
    fn from_wav_mono_f32() {
        let wav_data = write_wav_f32(&[0.0, 0.5, -0.5, 1.0], 44100, 1);
        let sd = SampleData::from_wav(Cursor::new(wav_data), 44100).unwrap();
        assert_eq!(sd.len(), 4);
        assert!((sd.samples()[1] - 0.5).abs() < 1e-6);
        assert!((sd.samples()[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn from_wav_stereo_to_mono() {
        // Stereo: L=0.8, R=0.2 → mono = 0.5
        let wav_data = write_wav_f32(&[0.8, 0.2, -0.4, -0.6], 44100, 2);
        let sd = SampleData::from_wav(Cursor::new(wav_data), 44100).unwrap();
        assert_eq!(sd.len(), 2);
        assert!((sd.samples()[0] - 0.5).abs() < 1e-6);
        assert!((sd.samples()[1] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn resample_identity() {
        // Same rate → no change.
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let output = resample_linear(&input, 44100, 44100);
        assert_eq!(output.len(), input.len());
        for (a, b) in output.iter().zip(input.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn resample_double_rate() {
        // 22050 → 44100: doubles the length (approximately).
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let output = resample_linear(&input, 22050, 44100);
        // Output should be roughly twice as long.
        assert!(output.len() >= 190 && output.len() <= 210);
        // First sample preserved.
        assert!((output[0] - input[0]).abs() < 1e-6);
    }

    #[test]
    fn resample_half_rate() {
        // 44100 → 22050: halves the length (approximately).
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let output = resample_linear(&input, 44100, 22050);
        assert!(output.len() >= 45 && output.len() <= 55);
    }

    #[test]
    fn from_wav_with_resample() {
        // Write at 22050 Hz, load at 44100 Hz.
        let samples: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0).sin()).collect();
        let wav_data = write_wav_f32(&samples, 22050, 1);
        let sd = SampleData::from_wav(Cursor::new(wav_data), 44100).unwrap();
        // Should be roughly doubled in length.
        assert!(sd.len() >= 190 && sd.len() <= 210);
        assert_eq!(sd.sample_rate(), 44100);
    }

    #[test]
    fn resample_empty() {
        let output = resample_linear(&[], 44100, 22050);
        assert!(output.is_empty());
    }

    #[test]
    fn resample_single_sample() {
        let output = resample_linear(&[0.5], 44100, 22050);
        assert_eq!(output.len(), 1);
        assert!((output[0] - 0.5).abs() < 1e-6);
    }
}
