pub mod dir_list;
pub mod file_read;
pub mod file_write;

pub use dir_list::DirListSkill;
pub use file_read::FileReadSkill;
pub use file_write::FileWriteSkill;

use std::sync::Arc;

use mauns_filesystem::PathGuard;

use crate::skillset::SkillSet;

/// Build a `SkillSet` containing all built-in skills.
pub fn default_skillset(guard: Arc<PathGuard>, dry_run: bool) -> SkillSet {
    SkillSet::new()
        .with_skill(Arc::new(FileReadSkill::new(Arc::clone(&guard))))
        .with_skill(Arc::new(FileWriteSkill::new(Arc::clone(&guard)).with_dry_run(dry_run)))
        .with_skill(Arc::new(DirListSkill::new(Arc::clone(&guard))))
}
