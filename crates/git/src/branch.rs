//! Branch name construction and push helpers.

use mauns_core::error::{MaunsError, Result};
use tracing::info;

use crate::{repo::GitRepo, safety};

/// Push `branch` to the `origin` remote using the supplied GitHub token for
/// HTTPS authentication.
///
/// This function only operates on branches that pass the safety check.
pub fn push_branch(repo: &GitRepo, branch: &str, token: &str) -> Result<()> {
    safety::assert_not_protected(branch)?;

    // Locate the origin remote.
    let mut remote = repo
        .inner()
        .find_remote("origin")
        .map_err(|e| MaunsError::Git(format!("cannot find 'origin' remote: {e}")))?;

    // Build authenticating callbacks using the token as the password.
    // GitHub accepts any non-empty username with a PAT.
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        git2::Cred::userpass_plaintext("x-access-token", token)
    });

    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote
        .push(&[&refspec], Some(&mut push_opts))
        .map_err(|e| MaunsError::Git(format!("push of '{branch}' failed: {e}")))?;

    info!(git = "push", branch = %branch, "branch pushed to origin");
    Ok(())
}
