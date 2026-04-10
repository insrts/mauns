//! Plugin system.
//!
//! Plugins extend the skill registry with additional capabilities.
//! They are statically compiled — no dynamic loading, no shell access,
//! no ability to override core safety rules or access tokens.

pub mod plugin;
pub mod registry;

pub use plugin::Plugin;
pub use registry::PluginRegistry;
