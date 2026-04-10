//! `SkillSet` — builder API with internal O(1) lookup map.
//!
//! The Vec preserves insertion order for the catalogue (used in prompts).
//! The HashMap provides O(1) dispatch during execution.
//! Neither structure is exposed — callers only see the builder API.

use std::{collections::HashMap, sync::Arc};

use mauns_core::error::{MaunsError, Result};
use tracing::debug;

use crate::skill::AgentSkill;

pub struct SkillSet {
    /// Ordered list for deterministic catalogue output.
    ordered: Vec<Arc<dyn AgentSkill>>,
    /// Internal map for O(1) lookup by name. Not exposed externally.
    lookup:  HashMap<String, Arc<dyn AgentSkill>>,
}

impl SkillSet {
    pub fn new() -> Self {
        Self {
            ordered: Vec::new(),
            lookup:  HashMap::new(),
        }
    }

    /// Add a skill. Panics in debug builds if the name is already taken;
    /// in release builds the new skill silently replaces the old one in the
    /// map (the ordered vec retains both for catalogue purposes — duplicates
    /// are harmless in prompts).
    pub fn with_skill(mut self, skill: Arc<dyn AgentSkill>) -> Self {
        let name = skill.name().to_string();
        debug!(skillset = "add", name = %name);
        debug_assert!(
            !self.lookup.contains_key(&name),
            "duplicate skill name '{name}'"
        );
        self.lookup.insert(name, Arc::clone(&skill));
        self.ordered.push(skill);
        self
    }

    /// O(1) skill lookup by name.
    /// Returns `Err(SkillNotFound)` for unknown names — never panics.
    pub fn dispatch(&self, name: &str) -> Result<Arc<dyn AgentSkill>> {
        self.lookup
            .get(name)
            .cloned()
            .ok_or_else(|| MaunsError::SkillNotFound(name.to_string()))
    }

    /// Sorted catalogue of (name, description) pairs for prompt injection.
    pub fn catalogue(&self) -> Vec<(&str, &str)> {
        let mut items: Vec<(&str, &str)> = self
            .ordered
            .iter()
            .map(|s| (s.name(), s.description()))
            .collect();
        items.sort_by_key(|(n, _)| *n);
        items
    }

    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }
}

impl Default for SkillSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mauns_core::types::{SkillInput, SkillOutput};

    struct Dummy(String);

    #[async_trait]
    impl AgentSkill for Dummy {
        fn name(&self) -> &str { &self.0 }
        fn description(&self) -> &str { "dummy" }
        async fn execute(&self, _: SkillInput) -> Result<SkillOutput> {
            Ok(SkillOutput::ok(serde_json::Value::Null))
        }
    }

    #[test]
    fn dispatch_known_skill() {
        let s = SkillSet::new().with_skill(Arc::new(Dummy("ping".into())));
        assert!(s.dispatch("ping").is_ok());
    }

    #[test]
    fn dispatch_unknown_returns_not_found() {
        let s = SkillSet::new();
        let res = s.dispatch("missing");
        assert!(res.is_err());
        let e = res.err().unwrap();
        assert!(matches!(e, MaunsError::SkillNotFound(_)));
    }

    #[test]
    fn catalogue_sorted() {
        let s = SkillSet::new()
            .with_skill(Arc::new(Dummy("zzz".into())))
            .with_skill(Arc::new(Dummy("aaa".into())));
        let cat = s.catalogue();
        assert_eq!(cat[0].0, "aaa");
        assert_eq!(cat[1].0, "zzz");
    }
}
