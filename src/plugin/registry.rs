//! Plugin registry — scans directories and instantiates config-based instruments.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::instrument::Instrument;

use super::config::{InstrumentKind, PluginManifest};
use super::instrument::ConfigInstrument;

/// Registry of discovered plugins.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, (PluginManifest, PathBuf)>,
}

impl PluginRegistry {
    /// Scan a directory for plugins. Each subdirectory containing `plugin.yaml` is a plugin.
    pub fn scan(base_dir: &Path) -> Self {
        let mut plugins = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("plugin.yaml");
                    if manifest_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = serde_yaml::from_str::<PluginManifest>(&content) {
                                plugins.insert(manifest.name.clone(), (manifest, path));
                            }
                        }
                    }
                }
            }
        }

        Self { plugins }
    }

    /// Scan the default plugin directory (`~/.resonance/plugins/`) and pack plugins.
    pub fn scan_default() -> Self {
        let mut registry = if let Some(home) = dirs::home_dir() {
            let plugin_dir = home.join(".resonance").join("plugins");
            Self::scan(&plugin_dir)
        } else {
            Self {
                plugins: HashMap::new(),
            }
        };

        // Also scan pack plugin directories
        let pack_manager = crate::content::packs::PackManager::default_manager();
        let pack_plugin_dirs = pack_manager.plugin_dirs();
        registry.scan_additional(&pack_plugin_dirs);

        registry
    }

    /// Merge plugins from additional directories (e.g., pack plugin dirs).
    pub fn scan_additional(&mut self, dirs: &[PathBuf]) {
        for dir in dirs {
            let manifest_path = dir.join("plugin.yaml");
            if manifest_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = serde_yaml::from_str::<PluginManifest>(&content) {
                        if !self.plugins.contains_key(&manifest.name) {
                            self.plugins
                                .insert(manifest.name.clone(), (manifest, dir.clone()));
                        }
                    }
                }
            }
        }
    }

    /// Look up a plugin by name.
    pub fn get(&self, name: &str) -> Option<&PluginManifest> {
        self.plugins.get(name).map(|(m, _)| m)
    }

    /// Create an instrument instance for a named plugin.
    pub fn create_instrument(&self, name: &str, sample_rate: u32) -> Option<Box<dyn Instrument>> {
        let (manifest, base_dir) = self.plugins.get(name)?;
        let inst_def = manifest.instrument.as_ref()?;

        let instrument: Box<dyn Instrument> = match inst_def.kind {
            InstrumentKind::Sampler => {
                let samples = inst_def.samples.as_ref().cloned().unwrap_or_default();
                Box::new(ConfigInstrument::sampler(
                    manifest.name.clone(),
                    &samples,
                    base_dir,
                    sample_rate,
                ))
            }
            InstrumentKind::Synth => {
                Box::new(ConfigInstrument::synth(manifest.name.clone(), inst_def))
            }
        };

        Some(instrument)
    }

    /// List all discovered plugins.
    pub fn list(&self) -> Vec<&PluginManifest> {
        self.plugins.values().map(|(m, _)| m).collect()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_plugin(dir: &Path, name: &str, yaml: &str) {
        let plugin_dir = dir.join(name);
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("plugin.yaml"), yaml).unwrap();
    }

    #[test]
    fn scan_empty_dir() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::scan(dir.path());
        assert!(registry.is_empty());
        assert!(registry.list().is_empty());
    }

    #[test]
    fn scan_dir_with_synth_plugin() {
        let dir = TempDir::new().unwrap();
        create_test_plugin(
            dir.path(),
            "warm_pad",
            r#"
name: "warm_pad"
version: "1.0.0"
description: "A warm pad synth"
instrument:
  kind: synth
  waveform: "saw"
  envelope:
    attack: 0.5
    decay: 0.3
    sustain: 0.6
    release: 1.0
"#,
        );

        let registry = PluginRegistry::scan(dir.path());
        assert!(!registry.is_empty());
        assert_eq!(registry.list().len(), 1);

        let manifest = registry.get("warm_pad").unwrap();
        assert_eq!(manifest.name, "warm_pad");
        assert_eq!(manifest.description.as_deref(), Some("A warm pad synth"));
    }

    #[test]
    fn create_synth_instrument() {
        let dir = TempDir::new().unwrap();
        create_test_plugin(
            dir.path(),
            "test_synth",
            r#"
name: "test_synth"
version: "1.0.0"
instrument:
  kind: synth
  waveform: "sine"
"#,
        );

        let registry = PluginRegistry::scan(dir.path());
        let inst = registry.create_instrument("test_synth", 44100);
        assert!(inst.is_some());
        assert_eq!(inst.unwrap().name(), "test_synth");
    }

    #[test]
    fn create_instrument_missing_plugin() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::scan(dir.path());
        assert!(registry.create_instrument("nonexistent", 44100).is_none());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let registry = PluginRegistry::scan(Path::new("/nonexistent_dir_xyz"));
        assert!(registry.is_empty());
    }

    #[test]
    fn scan_ignores_invalid_yaml() {
        let dir = TempDir::new().unwrap();
        let plugin_dir = dir.path().join("bad_plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(plugin_dir.join("plugin.yaml"), "{{invalid yaml").unwrap();

        let registry = PluginRegistry::scan(dir.path());
        assert!(registry.is_empty());
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = PluginRegistry::default();
        assert!(registry.is_empty());
    }
}
