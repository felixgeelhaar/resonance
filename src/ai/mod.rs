//! AI module — natural language command parsing and optional LLM integration.

pub mod config;
#[cfg(feature = "llm")]
pub mod llm;
pub mod nl_parser;
