//! Staging and committing filesystem changes.
//!
//! The ONLY input to commit operations is the `FileChange` list produced by
//! the filesystem tracker.  LLM output is never used to decide what is
//! staged or committed.

use std::path::Path;

use mauns_core::{
    error::{MaunsError, Result},
    types::{FileChange, FileOperation},
};
use tracing::{info, warn};

use crate::{repo::GitRepo, safety};

/// Stage and commit all `applied` changes from `change_log`.
///
/// Returns the OID of the created commit as a hex string.
///
/// Safety invariants (enforced here, not delegated to callers):
/// - Only `applied = true` changes are staged.
/// - Every file name passes `safety::assert_not_blocked`.
/// - The current branch is checked against `safety::assert_not_protected`.
pub fn stage_and_commit(
    repo: &GitRepo,
    change_log: &[FileChange],
    message: &str,
) -> Result<String> {
    // Enforce branch safety.
    let branch = repo.current_branch()?;
    safety::assert_not_protected(&branch)?;

    let inner = repo.inner();
    let mut index = inner
        .index()
        .map_err(|e| MaunsError::Git(format!("cannot open git index: {e}")))?;

    let mut staged = 0usize;

    for change in change_log {
        if !change.applied {
            warn!(git = "stage", path = %change.path, "skipping unapplied change");
            continue;
        }

        // Extract the file name component for blocklist checking.
        let filename = Path::new(&change.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&change.path);

        safety::assert_not_blocked(filename)?;

        match change.operation {
            FileOperation::Create | FileOperation::Edit => {
                index
                    .add_path(Path::new(&change.path))
                    .map_err(|e| MaunsError::Git(format!(
                        "failed to stage '{}': {e}", change.path
                    )))?;
                info!(git = "stage", path = %change.path, op = "add");
            }
            FileOperation::Delete => {
                index
                    .remove_path(Path::new(&change.path))
                    .map_err(|e| MaunsError::Git(format!(
                        "failed to remove '{}' from index: {e}", change.path
                    )))?;
                info!(git = "stage", path = %change.path, op = "remove");
            }
        }

        staged += 1;
    }

    if staged == 0 {
        return Err(MaunsError::Git(
            "no applied changes to commit; aborting".to_string(),
        ));
    }

    // Write the index to a tree.
    let tree_oid = index
        .write_tree()
        .map_err(|e| MaunsError::Git(format!("failed to write index tree: {e}")))?;

    index
        .write()
        .map_err(|e| MaunsError::Git(format!("failed to write index to disk: {e}")))?;

    let tree = inner
        .find_tree(tree_oid)
        .map_err(|e| MaunsError::Git(format!("failed to find tree object: {e}")))?;

    let sig = GitRepo::signature()?;

    // Resolve the parent commit (may not exist on an empty repo).
    let parents: Vec<git2::Commit<'_>> = match inner.head() {
        Ok(head) => {
            let oid = head.target().ok_or_else(|| {
                MaunsError::Git("HEAD has no target OID".to_string())
            })?;
            let c = inner.find_commit(oid).map_err(|e| {
                MaunsError::Git(format!("cannot find parent commit: {e}"))
            })?;
            vec![c]
        }
        Err(_) => vec![], // initial commit has no parent
    };

    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

    let commit_oid = inner
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .map_err(|e| MaunsError::Git(format!("commit failed: {e}")))?;

    let hex = commit_oid.to_string();
    info!(git = "commit", id = %hex, branch = %branch, "committed {} change(s)", staged);
    Ok(hex)
}
