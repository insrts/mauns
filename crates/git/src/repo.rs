//! Repository lifecycle — init, open, branch creation.

use std::path::{Path, PathBuf};

use git2::{Repository, Signature};
use mauns_core::error::{MaunsError, Result};
use tracing::{debug, info};

use crate::safety;

/// Thin wrapper around a `git2::Repository`.
pub struct GitRepo {
    inner: Repository,
    root: PathBuf,
}

impl GitRepo {
    /// Open the repository at `path`.  If no git repository exists there,
    /// initialise one.
    pub fn open_or_init(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let inner = match Repository::open(path) {
            Ok(r) => {
                debug!(git = "repo", path = %path.display(), "opened existing repository");
                r
            }
            Err(_) => {
                info!(git = "repo", path = %path.display(), "no repository found; initialising");
                Repository::init(path).map_err(|e| {
                    MaunsError::Git(format!(
                        "failed to init repository at '{}': {e}",
                        path.display()
                    ))
                })?
            }
        };

        Ok(Self {
            inner,
            root: path.to_path_buf(),
        })
    }

    /// Return the absolute root of the repository working directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the name of the current HEAD branch, or an error if HEAD is
    /// detached or the repository has no commits yet.
    pub fn current_branch(&self) -> Result<String> {
        let head = self
            .inner
            .head()
            .map_err(|e| MaunsError::Git(format!("cannot resolve HEAD: {e}")))?;

        head.shorthand()
            .map(|s| s.to_string())
            .ok_or_else(|| MaunsError::Git("HEAD has no shorthand name (detached?)".to_string()))
    }

    /// Create a new branch off the current HEAD and check it out.
    ///
    /// The branch name is validated by the safety layer before creation.
    pub fn create_and_checkout(&mut self, branch: &str) -> Result<()> {
        safety::assert_not_protected(branch)?;

        let head_commit = {
            let head = self.inner.head().map_err(|e| {
                MaunsError::Git(format!("cannot resolve HEAD for branch creation: {e}"))
            })?;
            let oid = head
                .target()
                .ok_or_else(|| MaunsError::Git("HEAD is not a direct reference".to_string()))?;
            self.inner
                .find_commit(oid)
                .map_err(|e| MaunsError::Git(format!("cannot find HEAD commit: {e}")))?
        };

        let git_branch = self
            .inner
            .branch(branch, &head_commit, false)
            .map_err(|e| MaunsError::Git(format!("failed to create branch '{branch}': {e}")))?;

        let refname = git_branch
            .get()
            .name()
            .ok_or_else(|| MaunsError::Git("branch reference has no name".to_string()))?
            .to_string();

        self.inner
            .set_head(&refname)
            .map_err(|e| MaunsError::Git(format!("failed to set HEAD to '{branch}': {e}")))?;

        self.inner
            .checkout_head(Some(git2::build::CheckoutBuilder::new().safe()))
            .map_err(|e| MaunsError::Git(format!("checkout of '{branch}' failed: {e}")))?;

        info!(git = "branch", name = %branch, "branch created and checked out");
        Ok(())
    }

    /// Expose a reference to the inner `git2::Repository` for staging/committing.
    pub fn inner(&self) -> &Repository {
        &self.inner
    }

    /// Build a git `Signature` for Mauns commits.
    pub(crate) fn signature() -> Result<Signature<'static>> {
        Signature::now("mauns", "mauns@localhost")
            .map_err(|e| MaunsError::Git(format!("failed to build git signature: {e}")))
    }
}
