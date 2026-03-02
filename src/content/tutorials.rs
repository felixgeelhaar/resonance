//! Tutorial loading — YAML-based lesson packs with built-in fallback.

use serde::Deserialize;

/// A pack of tutorial lessons.
#[derive(Debug, Clone, Deserialize)]
pub struct TutorialPack {
    pub name: String,
    pub lessons: Vec<TutorialLesson>,
}

/// A single tutorial lesson.
#[derive(Debug, Clone, Deserialize)]
pub struct TutorialLesson {
    pub id: String,
    pub title: String,
    pub explanation: Vec<String>,
    pub code: String,
    #[serde(default)]
    pub hints: Vec<String>,
    pub next: Option<String>,
}

// Built-in tutorial YAML compiled into the binary.
const BUILTIN_TUTORIAL: &str = include_str!("../../assets/tutorials/basics.yaml");

/// Load the built-in tutorial pack.
pub fn builtin_tutorial() -> TutorialPack {
    serde_yaml::from_str(BUILTIN_TUTORIAL).expect("built-in tutorial YAML must be valid")
}

/// Load a tutorial pack from a YAML file path.
pub fn load_tutorial_pack(path: &std::path::Path) -> Result<TutorialPack, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_yaml::from_str(&content).map_err(|e| format!("failed to parse tutorial YAML: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tutorial_loads() {
        let pack = builtin_tutorial();
        assert_eq!(pack.name, "First Sounds");
        assert!(!pack.lessons.is_empty());
    }

    #[test]
    fn builtin_has_expected_lessons() {
        let pack = builtin_tutorial();
        assert!(pack.lessons.len() >= 5, "should have at least 5 lessons");
        assert_eq!(pack.lessons[0].id, "first-beat");
    }

    #[test]
    fn lesson_code_compiles() {
        use crate::dsl::Compiler;
        let pack = builtin_tutorial();
        for lesson in &pack.lessons {
            let source = lesson.code.trim();
            if !source.is_empty() {
                assert!(
                    Compiler::compile(source).is_ok(),
                    "lesson '{}' code should compile: {}",
                    lesson.id,
                    Compiler::compile(source).unwrap_err()
                );
            }
        }
    }

    #[test]
    fn lesson_chain_is_valid() {
        let pack = builtin_tutorial();
        let ids: Vec<&str> = pack.lessons.iter().map(|l| l.id.as_str()).collect();
        for lesson in &pack.lessons {
            if let Some(ref next) = lesson.next {
                assert!(
                    ids.contains(&next.as_str()),
                    "lesson '{}' points to unknown next '{next}'",
                    lesson.id
                );
            }
        }
        // Last lesson should have next: null
        assert!(
            pack.lessons.last().unwrap().next.is_none(),
            "last lesson should have next: null"
        );
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result = load_tutorial_pack(std::path::Path::new("/nonexistent/tutorial.yaml"));
        assert!(result.is_err());
    }
}
