//! Intent processor — performance intents (quantized) vs structural intents (diff-based).
//!
//! Phase 1 implements only performance intents (macro-only).
//! Structural intents (diff-based code changes) are planned for Phase 2.

pub mod performance;

pub use performance::{IntentProcessor, PerformanceIntent};
