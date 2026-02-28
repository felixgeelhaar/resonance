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
/// Keywords influence tempo, pattern density, and instrument choice.
pub fn generate_starter(keywords: &str) -> String {
    let lower = keywords.to_lowercase();

    let tempo = if lower.contains("fast") || lower.contains("energetic") || lower.contains("dnb") {
        170
    } else if lower.contains("slow") || lower.contains("chill") || lower.contains("ambient") {
        85
    } else if lower.contains("techno") || lower.contains("dark") {
        130
    } else if lower.contains("house") || lower.contains("disco") {
        124
    } else {
        120
    };

    let sparse = lower.contains("minimal") || lower.contains("sparse");
    let kick_pattern = if sparse {
        "X . . . . . . ."
    } else {
        "X . . . X . . ."
    };
    let snare_pattern = if sparse {
        ". . . . X . . ."
    } else {
        ". . X . . . X ."
    };
    let hat_pattern = if lower.contains("ambient") || lower.contains("chill") {
        ". x . . . x . ."
    } else {
        "x x x x x x x x"
    };

    let has_bass = !lower.contains("ambient");

    let mut src = format!(
        r#"tempo {tempo}

track drums {{
  kit: default
  section groove [2 bars] {{
    kick:  [{kick_pattern}]
    snare: [{snare_pattern}]
    hat:   [{hat_pattern}]
  }}
}}"#
    );

    if has_bass {
        src.push_str(
            r#"

track bass {
  bass
  section groove [2 bars] {
    note: [C2 . . C2 . . Eb2 .]
  }
}"#,
        );
    }

    src
}

/// The default starter template for when no keywords are provided.
pub fn default_starter() -> String {
    generate_starter("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_starter_has_tempo() {
        let src = default_starter();
        assert!(src.contains("tempo 120"));
    }

    #[test]
    fn default_starter_has_drums() {
        let src = default_starter();
        assert!(src.contains("track drums"));
        assert!(src.contains("kit: default"));
    }

    #[test]
    fn default_starter_has_bass() {
        let src = default_starter();
        assert!(src.contains("track bass"));
    }

    #[test]
    fn techno_keywords_set_tempo() {
        let src = generate_starter("dark techno");
        assert!(src.contains("tempo 130"));
    }

    #[test]
    fn minimal_keywords_sparse_pattern() {
        let src = generate_starter("minimal techno");
        assert!(src.contains("X . . . . . . ."));
    }

    #[test]
    fn ambient_keywords_no_bass() {
        let src = generate_starter("ambient");
        assert!(!src.contains("track bass"));
    }

    #[test]
    fn ambient_sparse_hats() {
        let src = generate_starter("ambient chill");
        assert!(src.contains(". x . . . x . ."));
    }

    #[test]
    fn starter_compiles() {
        use crate::dsl::Compiler;
        let src = default_starter();
        assert!(Compiler::compile(&src).is_ok());
    }
}
