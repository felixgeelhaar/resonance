//! Preset loading — scans user directory and falls back to built-in presets.
//!
//! Preset files use optional YAML frontmatter (`---` delimited) followed by DSL source.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Metadata about an available preset.
#[derive(Debug, Clone)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub genre: String,
    pub file_path: Option<PathBuf>,
}

// Built-in presets compiled into the binary.
const BUILTIN_HOUSE: &str = include_str!("../../assets/presets/house.dsl");
const BUILTIN_TECHNO: &str = include_str!("../../assets/presets/techno.dsl");
const BUILTIN_AMBIENT: &str = include_str!("../../assets/presets/ambient.dsl");
const BUILTIN_DNB: &str = include_str!("../../assets/presets/dnb.dsl");
const BUILTIN_EMPTY: &str = include_str!("../../assets/presets/empty.dsl");

/// Parse optional YAML frontmatter from a preset file.
/// Returns (frontmatter_map, dsl_source_body).
fn parse_frontmatter(content: &str) -> (HashMap<String, String>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (HashMap::new(), content);
    }

    // Find the closing ---
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml_block = &after_first[..end];
        let body_start = end + 4; // skip \n---
        let body = after_first[body_start..].trim_start_matches('\n');

        let mut map = HashMap::new();
        for line in yaml_block.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let k = key.trim().to_string();
                let v = value.trim().trim_matches('"').to_string();
                if !k.is_empty() {
                    map.insert(k, v);
                }
            }
        }
        (map, body)
    } else {
        (HashMap::new(), content)
    }
}

/// Extract just the DSL source body from a preset file (strips frontmatter).
pub fn extract_source(content: &str) -> &str {
    let (_, body) = parse_frontmatter(content);
    body
}

/// Build a PresetInfo from frontmatter and optional file path.
fn info_from_content(content: &str, file_path: Option<PathBuf>) -> PresetInfo {
    let (meta, _) = parse_frontmatter(content);
    PresetInfo {
        name: meta
            .get("name")
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        description: meta.get("description").cloned().unwrap_or_else(String::new),
        genre: meta
            .get("genre")
            .cloned()
            .unwrap_or_else(|| "none".to_string()),
        file_path,
    }
}

/// Get the user presets directory (~/.resonance/presets/).
fn user_presets_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".resonance").join("presets"))
}

/// List all available presets (user directory + built-in).
/// User presets come first, built-in presets after.
pub fn list_presets() -> Vec<PresetInfo> {
    let mut presets = Vec::new();

    // Scan user directory
    if let Some(dir) = user_presets_dir() {
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "dsl") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            presets.push(info_from_content(&content, Some(path)));
                        }
                    }
                }
            }
        }
    }

    // Pack presets
    let pack_manager = super::packs::PackManager::default_manager();
    for path in pack_manager.preset_files() {
        if let Ok(content) = fs::read_to_string(&path) {
            presets.push(info_from_content(&content, Some(path)));
        }
    }

    // Built-in presets
    for content in [
        BUILTIN_HOUSE,
        BUILTIN_TECHNO,
        BUILTIN_AMBIENT,
        BUILTIN_DNB,
        BUILTIN_EMPTY,
    ] {
        presets.push(info_from_content(content, None));
    }

    presets
}

/// Load a preset by name. Checks user directory first, then built-in.
/// Returns the DSL source body (frontmatter stripped).
pub fn load_preset(name: &str) -> Option<String> {
    let lower = name.to_lowercase();

    // Check user directory first
    if let Some(dir) = user_presets_dir() {
        let file_path = dir.join(format!("{lower}.dsl"));
        if let Ok(content) = fs::read_to_string(&file_path) {
            return Some(extract_source(&content).to_string());
        }
    }

    // Check pack presets
    let pack_manager = super::packs::PackManager::default_manager();
    for path in pack_manager.preset_files() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if stem.to_lowercase() == lower {
                if let Ok(content) = fs::read_to_string(&path) {
                    return Some(extract_source(&content).to_string());
                }
            }
        }
    }

    // Built-in fallback
    let builtin = match lower.as_str() {
        "house" => Some(BUILTIN_HOUSE),
        "techno" => Some(BUILTIN_TECHNO),
        "ambient" => Some(BUILTIN_AMBIENT),
        "dnb" | "drum and bass" | "drum_and_bass" => Some(BUILTIN_DNB),
        "empty" => Some(BUILTIN_EMPTY),
        _ => None,
    };

    builtin.map(|content| extract_source(content).to_string())
}

/// Load the default starter preset (house).
pub fn default_preset() -> String {
    load_preset("house").expect("built-in house preset must exist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_have_frontmatter() {
        for (name, content) in [
            ("house", BUILTIN_HOUSE),
            ("techno", BUILTIN_TECHNO),
            ("ambient", BUILTIN_AMBIENT),
            ("dnb", BUILTIN_DNB),
            ("empty", BUILTIN_EMPTY),
        ] {
            let (meta, _) = parse_frontmatter(content);
            assert!(
                meta.contains_key("name"),
                "{name} preset should have name in frontmatter"
            );
        }
    }

    #[test]
    fn extract_source_strips_frontmatter() {
        let content = "---\nname: Test\n---\ntempo 120\n";
        let body = extract_source(content);
        assert!(body.starts_with("tempo"));
        assert!(!body.contains("---"));
    }

    #[test]
    fn extract_source_passthrough_no_frontmatter() {
        let content = "tempo 120\ntrack drums { }";
        let body = extract_source(content);
        assert_eq!(body, content);
    }

    #[test]
    fn builtin_presets_compile() {
        use crate::dsl::Compiler;
        for (name, content) in [
            ("house", BUILTIN_HOUSE),
            ("techno", BUILTIN_TECHNO),
            ("ambient", BUILTIN_AMBIENT),
            ("dnb", BUILTIN_DNB),
        ] {
            let source = extract_source(content);
            assert!(
                Compiler::compile(source).is_ok(),
                "{name} built-in preset should compile: {}",
                Compiler::compile(source).unwrap_err()
            );
        }
    }

    #[test]
    fn load_preset_by_name() {
        let source = load_preset("house");
        assert!(source.is_some());
        assert!(source.unwrap().contains("tempo"));
    }

    #[test]
    fn load_preset_unknown_returns_none() {
        assert!(load_preset("nonexistent_genre_xyz").is_none());
    }

    #[test]
    fn list_presets_includes_builtins() {
        let presets = list_presets();
        assert!(
            presets.len() >= 5,
            "should have at least 5 built-in presets"
        );
        let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"House"));
        assert!(names.contains(&"Techno"));
    }

    #[test]
    fn default_preset_works() {
        let source = default_preset();
        assert!(source.contains("tempo"));
    }

    #[test]
    fn frontmatter_parsing_handles_quoted_values() {
        let content = "---\nname: \"My Preset\"\ndescription: cool stuff\n---\ntempo 100\n";
        let (meta, body) = parse_frontmatter(content);
        assert_eq!(meta.get("name").unwrap(), "My Preset");
        assert_eq!(meta.get("description").unwrap(), "cool stuff");
        assert!(body.starts_with("tempo"));
    }
}
