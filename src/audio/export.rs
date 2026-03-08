//! Offline WAV export — renders a CompiledSong to a WAV file.

use std::path::PathBuf;

use crate::dsl::compile::CompiledSong;
use crate::event::EventScheduler;
use crate::instrument::InstrumentRouter;
use crate::plugin::registry::PluginRegistry;

/// Configuration for WAV export.
pub struct ExportConfig {
    pub output_path: PathBuf,
    /// Number of bars to render. None = full song.
    pub bars: Option<u32>,
    /// Whether to apply master effects (reverb/delay).
    pub include_effects: bool,
}

/// Errors that can occur during export.
#[derive(Debug)]
pub enum ExportError {
    Io(std::io::Error),
    Audio(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "I/O error: {e}"),
            ExportError::Audio(s) => write!(f, "audio error: {s}"),
        }
    }
}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        ExportError::Io(e)
    }
}

impl From<hound::Error> for ExportError {
    fn from(e: hound::Error) -> Self {
        ExportError::Audio(e.to_string())
    }
}

/// Export a compiled song to a WAV file.
///
/// Returns the total number of samples written.
pub fn export_wav(
    song: &CompiledSong,
    config: ExportConfig,
    seed: u64,
    sample_rate: u32,
    plugin_registry: &PluginRegistry,
) -> Result<u64, ExportError> {
    let channels: u16 = 2;
    let block_size: u32 = 1024;

    // Determine total bars
    let total_bars = config.bars.unwrap_or_else(|| {
        // Calculate from events
        if song.events.is_empty() {
            4
        } else {
            let max_beat = song
                .events
                .iter()
                .map(|e| e.time.as_beats_f64() + e.duration.as_beats_f64())
                .fold(0.0f64, f64::max);
            ((max_beat / 4.0).ceil() as u32).max(1)
        }
    });

    let total_beats = total_bars as f64 * 4.0;
    let total_seconds = total_beats * 60.0 / song.tempo;
    let total_frames = (total_seconds * sample_rate as f64) as u64;

    // Build scheduler
    let mut scheduler = EventScheduler::new(song.tempo, sample_rate, channels, block_size, seed);
    scheduler.timeline_mut().insert_batch(song.events.clone());
    scheduler.play();

    // Build router
    let router = InstrumentRouter::from_track_defs_with_kits(
        &song.track_defs,
        sample_rate,
        seed,
        plugin_registry,
    );
    let mut render_fn = router.into_render_fn();

    // Build master effects (with audible defaults when enabled)
    let mut effects = if config.include_effects {
        let mut fx = crate::audio::effects::MasterEffects::new(sample_rate);
        fx.reverb.set_mix(0.3);
        fx.delay.set_mix(0.2);
        Some(fx)
    } else {
        None
    };

    // Create WAV writer
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(&config.output_path, spec)?;

    let mut samples_written: u64 = 0;
    let blocks_needed = total_frames.div_ceil(block_size as u64);

    for _ in 0..blocks_needed {
        let block = match scheduler.render_block(&mut render_fn) {
            Some(b) => b,
            None => break,
        };

        let processed = if let Some(ref mut fx) = effects {
            let mut buf = block;
            fx.process_block(&mut buf);
            buf
        } else {
            block
        };

        for &sample in &processed {
            // Soft limit
            let limited = sample.clamp(-1.0, 1.0);
            writer.write_sample(limited)?;
            samples_written += 1;
        }
    }

    writer.finalize()?;
    Ok(samples_written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::Compiler;

    #[test]
    fn export_creates_valid_wav() {
        let song = Compiler::compile(
            r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . X .]
    }
}
"#,
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wav");
        let config = ExportConfig {
            output_path: path.clone(),
            bars: Some(1),
            include_effects: false,
        };
        let registry = PluginRegistry::default();
        let count = export_wav(&song, config, 42, 44100, &registry).unwrap();
        assert!(count > 0);
        assert!(path.exists());

        // Verify it's a valid WAV
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 44100);
    }

    #[test]
    fn determinism() {
        let song = Compiler::compile(
            r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . . .]
    }
}
"#,
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let registry = PluginRegistry::default();

        let path1 = dir.path().join("a.wav");
        export_wav(
            &song,
            ExportConfig {
                output_path: path1.clone(),
                bars: Some(1),
                include_effects: false,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        let path2 = dir.path().join("b.wav");
        export_wav(
            &song,
            ExportConfig {
                output_path: path2.clone(),
                bars: Some(1),
                include_effects: false,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        let data1 = std::fs::read(&path1).unwrap();
        let data2 = std::fs::read(&path2).unwrap();
        assert_eq!(data1, data2, "same seed should produce identical WAV");
    }

    #[test]
    fn bars_limit() {
        let song = Compiler::compile(
            r#"
tempo 120
track drums {
    kit: default
    section main [4 bars] {
        kick: [X . . . X . . . X . . . X . . .]
    }
}
"#,
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let registry = PluginRegistry::default();

        let short_path = dir.path().join("short.wav");
        let short_count = export_wav(
            &song,
            ExportConfig {
                output_path: short_path,
                bars: Some(1),
                include_effects: false,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        let long_path = dir.path().join("long.wav");
        let long_count = export_wav(
            &song,
            ExportConfig {
                output_path: long_path,
                bars: Some(4),
                include_effects: false,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        assert!(
            long_count > short_count,
            "4 bars should be longer than 1 bar"
        );
    }

    #[test]
    fn effects_change_output() {
        let song = Compiler::compile(
            r#"
tempo 120
track drums {
    kit: default
    section main [1 bars] {
        kick: [X . X .]
    }
}
"#,
        )
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let registry = PluginRegistry::default();

        let dry_path = dir.path().join("dry.wav");
        export_wav(
            &song,
            ExportConfig {
                output_path: dry_path.clone(),
                bars: Some(1),
                include_effects: false,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        let wet_path = dir.path().join("wet.wav");
        export_wav(
            &song,
            ExportConfig {
                output_path: wet_path.clone(),
                bars: Some(1),
                include_effects: true,
            },
            42,
            44100,
            &registry,
        )
        .unwrap();

        let dry = std::fs::read(&dry_path).unwrap();
        let wet = std::fs::read(&wet_path).unwrap();
        // Effects should change something (reverb tail makes file larger or different)
        // At minimum they shouldn't be identical
        assert_ne!(dry, wet, "effects should change output");
    }
}
