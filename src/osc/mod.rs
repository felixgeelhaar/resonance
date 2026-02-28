//! OSC (Open Sound Control) support — receive control messages over UDP.

pub mod config;
pub mod listener;
pub mod mapping;

pub use config::OscConfig;
pub use listener::OscListener;
pub use mapping::{apply_osc_message, OscMapping, OscTarget};
