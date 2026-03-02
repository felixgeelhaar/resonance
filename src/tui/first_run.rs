//! First-run detection and DSL skeleton generation.
//!
//! On first run, checks if `~/.resonance/` exists. If not, creates it
//! and returns a starter DSL template.

use std::fs;
use std::path::PathBuf;

/// Get the Resonance config directory path.
pub fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".resonance"))
}

/// Check if this is the first run (config dir doesn't exist).
pub fn is_first_run() -> bool {
    config_dir().map_or(true, |p| !p.exists())
}

/// Create the config directory for first-run setup.
pub fn create_config_dir() -> std::io::Result<PathBuf> {
    let dir = config_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine home directory",
        )
    })?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Generate a starter DSL template based on keywords describing the desired feel.
///
/// Keywords are dispatched to curated genre presets. Unrecognized keywords
/// fall through to the house preset (most universally appealing).
pub fn generate_starter(keywords: &str) -> String {
    let lower = keywords.to_lowercase();

    if lower.contains("techno") || lower.contains("dark") {
        return preset_techno();
    }
    if lower.contains("ambient") || lower.contains("chill") {
        return preset_ambient();
    }
    if lower.contains("dnb") || lower.contains("jungle") || lower.contains("drum and bass") {
        return preset_dnb();
    }
    if lower.contains("house") || lower.contains("disco") {
        return preset_house();
    }

    // Default: house preset
    preset_house()
}

/// The default starter template for when no keywords are provided.
/// Delegates to the content module, falls back to built-in preset_house().
pub fn default_starter() -> String {
    crate::content::presets::load_preset("house").unwrap_or_else(preset_house)
}

/// House preset — 124 BPM, classic 4/4 kick, offbeat hats, bass groove, pad.
fn preset_house() -> String {
    r#"tempo 124

macro feel = 0.4
macro space = 0.3

map feel -> cutoff (200.0..6000.0) exp
map space -> reverb_mix (0.0..0.5) linear
map space -> delay_mix (0.0..0.3) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    hat:   [. . x . . . x . . . x . . . x .]
  }
  section main [4 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    snare: [. . . . X . . . . . . . X . . .]
    hat:   [. x . x . x . x . x . x . x . x]
    clap:  [. . . . X . . . . . . . X . . .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C2 . . . . . . . C2 . . . . . Eb2 .]
  }
  section main [4 bars] {
    note: [C2 . . C2 . . Eb2 . F2 . . F2 . . C2 .]
  }
}

track pad {
  poly
  section main [4 bars] {
    note: [C4 . . . . . . . Eb4 . . . . . . .]
  }
}"#
    .to_string()
}

/// Techno preset — 130 BPM, driving kick, minimal snare, industrial feel.
fn preset_techno() -> String {
    r#"tempo 130

macro feel = 0.3
macro space = 0.2
macro drive = 0.4

map feel -> cutoff (100.0..4000.0) exp
map space -> reverb_mix (0.0..0.4) linear
map space -> delay_mix (0.0..0.2) linear
map drive -> drive (0.0..0.8) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    hat:   [x . x . x . x . x . x . x . x .]
  }
  section main [4 bars] {
    kick:  [X . . . X . . . X . . . X . . .]
    snare: [. . . . . . . . X . . . . . . .]
    hat:   [x . x . x . x . x . x . x . x .]
    clap:  [. . . . X . . . . . . . . . . .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C1 . . . . . . . . . . . . . . .]
  }
  section main [4 bars] {
    note: [C1 . . C1 . . . . C1 . . . . . C1 .]
  }
}"#
    .to_string()
}

/// Ambient preset — 85 BPM, poly pad, pluck texture, heavy reverb.
fn preset_ambient() -> String {
    r#"tempo 85

macro feel = 0.6
macro space = 0.7

map feel -> cutoff (300.0..3000.0) linear
map feel -> attack (0.05..0.8) linear
map space -> reverb_mix (0.2..0.8) linear
map space -> delay_mix (0.1..0.5) linear
map space -> delay_feedback (0.3..0.7) linear

track pad {
  poly
  section drift [4 bars] {
    note: [C4 . . . . . . . G3 . . . . . . .]
  }
  section bloom [4 bars] {
    note: [Eb4 . . . . . . . D4 . . . . . . .]
  }
}

track texture {
  pluck
  section drift [4 bars] {
    note: [. . G4 . . . C5 . . . . . . . . .]
  }
  section bloom [4 bars] {
    note: [. . . . Bb4 . . . . . G4 . . . . .]
  }
}"#
    .to_string()
}

/// Drum & Bass preset — 170 BPM, breakbeat pattern, fast bass.
fn preset_dnb() -> String {
    r#"tempo 170

macro feel = 0.5
macro space = 0.3

map feel -> cutoff (200.0..8000.0) exp
map space -> reverb_mix (0.0..0.4) linear
map space -> delay_mix (0.0..0.25) linear

track drums {
  kit: default
  section intro [2 bars] {
    kick:  [X . . . . . X . . . X . . . . .]
    snare: [. . . . X . . . . . . . X . . .]
    hat:   [x x x x x x x x x x x x x x x x]
  }
  section main [4 bars] {
    kick:  [X . . . . . X . . . X . . . . .]
    snare: [. . . . X . . . . . . X X . . .]
    hat:   [x x x x x x x x x x x x x x x x]
    clap:  [. . . . X . . . . . . . . . X .]
  }
}

track bass {
  bass
  section intro [2 bars] {
    note: [C2 . . . . . . . Eb2 . . . . . . .]
  }
  section main [4 bars] {
    note: [C2 . C2 . . . Eb2 . F2 . . . C2 . . .]
  }
}"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::Compiler;

    // --- Compilation tests ---

    #[test]
    fn house_preset_compiles() {
        let src = preset_house();
        assert!(
            Compiler::compile(&src).is_ok(),
            "house preset should compile"
        );
    }

    #[test]
    fn techno_preset_compiles() {
        let src = preset_techno();
        assert!(
            Compiler::compile(&src).is_ok(),
            "techno preset should compile"
        );
    }

    #[test]
    fn ambient_preset_compiles() {
        let src = preset_ambient();
        assert!(
            Compiler::compile(&src).is_ok(),
            "ambient preset should compile"
        );
    }

    #[test]
    fn dnb_preset_compiles() {
        let src = preset_dnb();
        assert!(Compiler::compile(&src).is_ok(), "dnb preset should compile");
    }

    #[test]
    fn default_starter_compiles() {
        let src = default_starter();
        assert!(Compiler::compile(&src).is_ok());
    }

    // --- Content tests ---

    #[test]
    fn presets_have_macros() {
        for (name, src) in [
            ("house", preset_house()),
            ("techno", preset_techno()),
            ("ambient", preset_ambient()),
            ("dnb", preset_dnb()),
        ] {
            assert!(src.contains("macro "), "{name} should have macros");
        }
    }

    #[test]
    fn presets_have_mappings() {
        for (name, src) in [
            ("house", preset_house()),
            ("techno", preset_techno()),
            ("ambient", preset_ambient()),
            ("dnb", preset_dnb()),
        ] {
            assert!(src.contains("map "), "{name} should have mappings");
        }
    }

    #[test]
    fn presets_are_multi_track() {
        for (name, src) in [
            ("house", preset_house()),
            ("techno", preset_techno()),
            ("ambient", preset_ambient()),
            ("dnb", preset_dnb()),
        ] {
            let track_count = src.matches("track ").count();
            assert!(
                track_count >= 2,
                "{name} should have >= 2 tracks, has {track_count}"
            );
        }
    }

    #[test]
    fn presets_are_multi_section() {
        for (name, src) in [
            ("house", preset_house()),
            ("techno", preset_techno()),
            ("ambient", preset_ambient()),
            ("dnb", preset_dnb()),
        ] {
            let section_count = src.matches("section ").count();
            assert!(
                section_count >= 2,
                "{name} should have >= 2 sections, has {section_count}"
            );
        }
    }

    #[test]
    fn presets_correct_tempos() {
        assert!(preset_house().contains("tempo 124"));
        assert!(preset_techno().contains("tempo 130"));
        assert!(preset_ambient().contains("tempo 85"));
        assert!(preset_dnb().contains("tempo 170"));
    }

    // --- Dispatch tests ---

    #[test]
    fn keywords_dispatch_to_correct_preset() {
        assert!(generate_starter("house").contains("tempo 124"));
        assert!(generate_starter("disco").contains("tempo 124"));
        assert!(generate_starter("techno").contains("tempo 130"));
        assert!(generate_starter("dark").contains("tempo 130"));
        assert!(generate_starter("ambient").contains("tempo 85"));
        assert!(generate_starter("chill").contains("tempo 85"));
        assert!(generate_starter("dnb").contains("tempo 170"));
        assert!(generate_starter("jungle").contains("tempo 170"));
    }

    #[test]
    fn unknown_keywords_default_to_house() {
        assert!(generate_starter("").contains("tempo 124"));
        assert!(generate_starter("random words").contains("tempo 124"));
    }
}
