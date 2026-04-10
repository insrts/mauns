//! AgentSkills system.
//!
//! Skills are stored as `Vec<Arc<dyn AgentSkill>>`.
//! There is no registry — callers build a `SkillSet` via the builder API.

pub mod builtin;
pub mod skill;
pub mod skillset;

pub use skill::AgentSkill;
pub use skillset::SkillSet;
