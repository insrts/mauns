//! Plugin registry — loads plugins and applies them to the skill registry.

use std::sync::Arc;

use mauns_core::error::{MaunsError, Result};
use mauns_skills::registry::SkillRegistry;
use tracing::{debug, info, warn};

use crate::plugin::Plugin;

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn Plugin>>,
    enabled: bool,
}

impl PluginRegistry {
    pub fn new(enabled: bool) -> Self {
        Self { plugins: Vec::new(), enabled }
    }

    /// Register a plugin. Ignored with a warning when plugins are disabled.
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        if !self.enabled {
            warn!(
                plugins = "registry",
                name = %plugin.name(),
                "plugins are disabled in config; skipping registration"
            );
            return;
        }
        debug!(plugins = "registry", name = %plugin.name(), "registered");
        self.plugins.push(plugin);
    }

    /// Apply all registered plugins to the skill registry.
    ///
    /// Plugin failures are non-fatal: a failed plugin logs an error and
    /// the remaining plugins continue loading.
    pub fn apply(&self, skills: &mut SkillRegistry) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        for plugin in &self.plugins {
            match plugin.register(skills) {
                Ok(()) => {
                    info!(plugins = "apply", name = %plugin.name(), "skills registered");
                }
                Err(e) => {
                    return Err(MaunsError::Plugin {
                        name:    plugin.name().to_string(),
                        message: format!("failed to register skills: {e}"),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mauns_core::error::Result;

    struct NoOpPlugin;

    impl Plugin for NoOpPlugin {
        fn name(&self) -> &str { "no_op" }
        fn register(&self, _: &mut SkillRegistry) -> Result<()> { Ok(()) }
    }

    #[test]
    fn disabled_registry_ignores_plugins() {
        let mut pr = PluginRegistry::new(false);
        pr.register(Arc::new(NoOpPlugin));
        assert_eq!(pr.len(), 0);
    }

    #[test]
    fn enabled_registry_accepts_plugins() {
        let mut pr = PluginRegistry::new(true);
        pr.register(Arc::new(NoOpPlugin));
        assert_eq!(pr.len(), 1);
    }

    #[test]
    fn apply_calls_plugin_register() {
        let mut pr = PluginRegistry::new(true);
        pr.register(Arc::new(NoOpPlugin));
        let mut sr = SkillRegistry::new();
        pr.apply(&mut sr).unwrap();
    }
}
