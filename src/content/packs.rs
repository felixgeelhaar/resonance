//! Instrument packs — bundles of kits, plugins, and presets.
//!
//! A pack is a directory under `~/.resonance/packs/<name>/` containing:
//! - `manifest.yaml` — pack metadata and contents listing
//! - `samples/<kit_name>/` — WAV sample directories (usable as kits)
//! - `plugins/<plugin_name>/` — plugin directories (each with `plugin.yaml`)
//! - `presets/<name>.dsl` — preset DSL files

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Error type for pack operations.
#[derive(Debug)]
pub enum PackError {
    Io(std::io::Error),
    InvalidManifest(String),
    NotFound(String),
    AlreadyExists(String),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Io(e) => write!(f, "I/O error: {e}"),
            PackError::InvalidManifest(e) => write!(f, "invalid manifest: {e}"),
            PackError::NotFound(name) => write!(f, "pack not found: {name}"),
            PackError::AlreadyExists(name) => write!(f, "pack already exists: {name}"),
        }
    }
}

impl From<std::io::Error> for PackError {
    fn from(e: std::io::Error) -> Self {
        PackError::Io(e)
    }
}

/// Pack manifest — describes the contents of an instrument pack.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackManifest {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genre: Option<String>,
    /// Kit name → list of sample filenames (informational).
    pub kits: Option<HashMap<String, Vec<String>>>,
    /// Plugin directory names included in this pack.
    pub plugins: Option<Vec<String>>,
    /// Preset `.dsl` filenames included in this pack.
    pub presets: Option<Vec<String>>,
}

/// Manages instrument packs in a base directory.
pub struct PackManager {
    base_dir: PathBuf,
}

impl PackManager {
    /// Create a pack manager for a specific directory.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a pack manager for the default location (`~/.resonance/packs/`).
    pub fn default_manager() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".resonance")
            .join("packs");
        Self { base_dir }
    }

    /// List all installed packs with their manifests.
    pub fn list(&self) -> Vec<(String, PackManifest)> {
        let mut packs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("manifest.yaml");
                    if let Ok(content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_yaml::from_str::<PackManifest>(&content) {
                            let dir_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            packs.push((dir_name, manifest));
                        }
                    }
                }
            }
        }

        packs
    }

    /// Install a pack from a source directory (copies it into the packs directory).
    pub fn install(&self, source_dir: &Path) -> Result<(), PackError> {
        // Read manifest from source
        let manifest_path = source_dir.join("manifest.yaml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|_| PackError::InvalidManifest("manifest.yaml not found".to_string()))?;
        let manifest: PackManifest = serde_yaml::from_str(&content)
            .map_err(|e| PackError::InvalidManifest(e.to_string()))?;

        let dest_dir = self.base_dir.join(&manifest.name);
        if dest_dir.exists() {
            return Err(PackError::AlreadyExists(manifest.name));
        }

        // Create destination and copy contents
        fs::create_dir_all(&dest_dir)?;
        copy_dir_recursive(source_dir, &dest_dir)?;

        Ok(())
    }

    /// Remove an installed pack by name.
    pub fn remove(&self, name: &str) -> Result<(), PackError> {
        let pack_dir = self.base_dir.join(name);
        if !pack_dir.exists() {
            return Err(PackError::NotFound(name.to_string()));
        }
        fs::remove_dir_all(&pack_dir)?;
        Ok(())
    }

    /// Get info about a specific pack.
    pub fn info(&self, name: &str) -> Option<PackManifest> {
        let manifest_path = self.base_dir.join(name).join("manifest.yaml");
        let content = fs::read_to_string(&manifest_path).ok()?;
        serde_yaml::from_str(&content).ok()
    }

    /// Get all kit directories from all installed packs.
    /// Returns (kit_name, path_to_samples_dir) pairs.
    pub fn kit_dirs(&self) -> Vec<(String, PathBuf)> {
        let mut dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let pack_path = entry.path();
                if pack_path.is_dir() {
                    let samples_dir = pack_path.join("samples");
                    if samples_dir.is_dir() {
                        if let Ok(kit_entries) = fs::read_dir(&samples_dir) {
                            for kit_entry in kit_entries.flatten() {
                                let kit_path = kit_entry.path();
                                if kit_path.is_dir() {
                                    if let Some(name) =
                                        kit_path.file_name().and_then(|n| n.to_str())
                                    {
                                        dirs.push((name.to_string(), kit_path));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        dirs
    }

    /// Get all preset files from all installed packs.
    pub fn preset_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let pack_path = entry.path();
                if pack_path.is_dir() {
                    let presets_dir = pack_path.join("presets");
                    if presets_dir.is_dir() {
                        if let Ok(preset_entries) = fs::read_dir(&presets_dir) {
                            for preset_entry in preset_entries.flatten() {
                                let path = preset_entry.path();
                                if path.extension().is_some_and(|ext| ext == "dsl") {
                                    files.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Get all plugin directories from all installed packs.
    pub fn plugin_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let pack_path = entry.path();
                if pack_path.is_dir() {
                    let plugins_dir = pack_path.join("plugins");
                    if plugins_dir.is_dir() {
                        if let Ok(plugin_entries) = fs::read_dir(&plugins_dir) {
                            for plugin_entry in plugin_entries.flatten() {
                                let plugin_path = plugin_entry.path();
                                if plugin_path.is_dir() && plugin_path.join("plugin.yaml").exists()
                                {
                                    dirs.push(plugin_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        dirs
    }
}

/// Recursively copy a directory's contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), PackError> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Resolve a kit name by checking installed packs.
/// Returns the path to the kit's sample directory if found.
pub fn resolve_kit_from_packs(name: &str) -> Option<PathBuf> {
    let manager = PackManager::default_manager();
    for (kit_name, kit_path) in manager.kit_dirs() {
        if kit_name == name {
            return Some(kit_path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_pack(base: &Path, name: &str) -> PathBuf {
        let pack_dir = base.join(name);
        fs::create_dir_all(&pack_dir).unwrap();
        let manifest = format!(
            r#"name: "{name}"
version: "1.0.0"
author: "Test Author"
description: "A test pack"
genre: "electronic"
kits:
  808:
    - kick.wav
    - snare.wav
plugins:
  - warm_pad
presets:
  - groove.dsl
"#
        );
        fs::write(pack_dir.join("manifest.yaml"), manifest).unwrap();
        pack_dir
    }

    fn create_pack_with_samples(base: &Path, name: &str) -> PathBuf {
        let pack_dir = create_test_pack(base, name);

        // Create samples directory with a kit
        let kit_dir = pack_dir.join("samples").join("808");
        fs::create_dir_all(&kit_dir).unwrap();

        // Write a minimal WAV file
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let wav_path = kit_dir.join("kick.wav");
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        writer.write_sample(0.5f32).unwrap();
        writer.finalize().unwrap();

        pack_dir
    }

    fn create_pack_with_presets(base: &Path, name: &str) -> PathBuf {
        let pack_dir = create_test_pack(base, name);
        let presets_dir = pack_dir.join("presets");
        fs::create_dir_all(&presets_dir).unwrap();
        fs::write(
            presets_dir.join("groove.dsl"),
            "---\nname: Groove\n---\ntempo 125\n",
        )
        .unwrap();
        pack_dir
    }

    fn create_pack_with_plugins(base: &Path, name: &str) -> PathBuf {
        let pack_dir = create_test_pack(base, name);
        let plugin_dir = pack_dir.join("plugins").join("warm_pad");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("plugin.yaml"),
            r#"name: "warm_pad"
version: "1.0.0"
instrument:
  kind: synth
  waveform: "saw"
"#,
        )
        .unwrap();
        pack_dir
    }

    #[test]
    fn parse_pack_manifest() {
        let yaml = r#"
name: "808_kit"
version: "1.0.0"
author: "Producer"
description: "Classic 808 sounds"
genre: "hip-hop"
kits:
  808:
    - kick.wav
    - snare.wav
    - hat.wav
plugins:
  - warm_pad
presets:
  - groove.dsl
"#;
        let manifest: PackManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "808_kit");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.author.as_deref(), Some("Producer"));
        assert!(manifest.kits.unwrap().contains_key("808"));
        assert_eq!(manifest.plugins.unwrap().len(), 1);
        assert_eq!(manifest.presets.unwrap().len(), 1);
    }

    #[test]
    fn list_empty_dir() {
        let dir = TempDir::new().unwrap();
        let manager = PackManager::new(dir.path().to_path_buf());
        assert!(manager.list().is_empty());
    }

    #[test]
    fn list_with_pack() {
        let dir = TempDir::new().unwrap();
        create_test_pack(dir.path(), "test_pack");
        let manager = PackManager::new(dir.path().to_path_buf());
        let packs = manager.list();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].1.name, "test_pack");
    }

    #[test]
    fn install_and_remove() {
        let dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();

        // Create a source pack
        let manifest = r#"name: "new_pack"
version: "1.0.0"
description: "A new pack"
"#;
        fs::write(source_dir.path().join("manifest.yaml"), manifest).unwrap();
        fs::write(source_dir.path().join("readme.txt"), "Hello").unwrap();

        let manager = PackManager::new(dir.path().to_path_buf());
        manager.install(source_dir.path()).unwrap();

        // Should be listed
        let packs = manager.list();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].1.name, "new_pack");

        // Info should work
        let info = manager.info("new_pack").unwrap();
        assert_eq!(info.name, "new_pack");

        // Remove
        manager.remove("new_pack").unwrap();
        assert!(manager.list().is_empty());
    }

    #[test]
    fn install_already_exists() {
        let dir = TempDir::new().unwrap();
        let source = TempDir::new().unwrap();
        fs::write(
            source.path().join("manifest.yaml"),
            r#"name: "dup"
version: "1.0.0"
"#,
        )
        .unwrap();

        let manager = PackManager::new(dir.path().to_path_buf());
        manager.install(source.path()).unwrap();
        assert!(manager.install(source.path()).is_err());
    }

    #[test]
    fn remove_nonexistent() {
        let dir = TempDir::new().unwrap();
        let manager = PackManager::new(dir.path().to_path_buf());
        assert!(manager.remove("nonexistent").is_err());
    }

    #[test]
    fn kit_dirs_returns_kit_paths() {
        let dir = TempDir::new().unwrap();
        create_pack_with_samples(dir.path(), "sample_pack");
        let manager = PackManager::new(dir.path().to_path_buf());
        let kits = manager.kit_dirs();
        assert_eq!(kits.len(), 1);
        assert_eq!(kits[0].0, "808");
        assert!(kits[0].1.exists());
    }

    #[test]
    fn preset_files_returns_preset_paths() {
        let dir = TempDir::new().unwrap();
        create_pack_with_presets(dir.path(), "preset_pack");
        let manager = PackManager::new(dir.path().to_path_buf());
        let presets = manager.preset_files();
        assert_eq!(presets.len(), 1);
        assert!(presets[0].to_str().unwrap().contains("groove.dsl"));
    }

    #[test]
    fn plugin_dirs_returns_plugin_paths() {
        let dir = TempDir::new().unwrap();
        create_pack_with_plugins(dir.path(), "plugin_pack");
        let manager = PackManager::new(dir.path().to_path_buf());
        let plugins = manager.plugin_dirs();
        assert_eq!(plugins.len(), 1);
        assert!(plugins[0].join("plugin.yaml").exists());
    }
}
