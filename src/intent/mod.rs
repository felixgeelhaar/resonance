//! Intent processor — performance intents (quantized) vs structural intents (diff-based).
//!
//! Performance intents fire immediately on beat boundaries (macro-only).
//! Structural intents produce AST diffs requiring user confirmation.

pub mod mode;
pub mod performance;
pub mod structural;

pub use mode::{detect_mode, IntentMode};
pub use performance::{IntentProcessor, PerformanceIntent};
pub use structural::{StructuralIntent, StructuralIntentProcessor, StructuralIntentState};
