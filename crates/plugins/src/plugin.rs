//! Plugin trait.

use mauns_core::error::Result;
use mauns_skills::registry::SkillRegistry;

/// A plugin may only contribute additional skills to the registry.
///
/// Invariants that CANNOT be violated by any plugin implementation:
/// - Plugins cannot override core safety rules.
/// - Plugins cannot access API tokens or environment secrets.
/// - Plugins cannot execute shell commands.
/// - Plugins cannot replace already-registered skills.
pub trait Plugin: Send + Sync {
    /// Unique machine-readable name.
    fn name(&self) -> &str;

    /// Register the plugin's skills into the provided registry.
    /// Any attempt to register a skill whose name conflicts with an existing
    /// one will return `Err(MaunsError::Skill { .. })` from the registry.
    fn register(&self, registry: &mut SkillRegistry) -> Result<()>;
}
