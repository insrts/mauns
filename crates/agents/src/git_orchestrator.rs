//! Git + GitHub orchestration layer.
//!
//! Called by the pipeline AFTER the confirmation gate.
//! The LLM has zero input into any operation performed here.

use mauns_core::{
    error::{MaunsError, Result},
    types::{FileChange, GitOutcome, RunContext},
};
use mauns_git::{
    branch::push_branch,
    commit::stage_and_commit,
    repo::GitRepo,
    safety::branch_name,
};
use mauns_github::{
    client::GitHubClient,
    pr::{create_pull_request, default_branch, parse_remote_url, PrRequest},
};
use tracing::{info, warn};

/// Configuration for the git/github step, resolved from CLI flags and config.
#[derive(Debug, Clone)]
pub struct GitConfig {
    pub create_pr:     bool,
    pub commit_prefix: String,
}

impl GitConfig {
    /// Build from config + CLI overrides.
    pub fn new(config_create_pr: bool, no_pr_flag: bool) -> Self {
        // CLI --no-pr always wins over config.
        let create_pr = if no_pr_flag { false } else { config_create_pr };
        Self {
            create_pr,
            commit_prefix: "[mauns] ".to_string(),
        }
    }
}

/// Run the full git workflow:
///   1. Open or init repository.
///   2. Create a safe `mauns/<slug>-<ts>` branch.
///   3. Stage and commit applied changes (source: filesystem tracker only).
///   4. Push to origin if GITHUB_TOKEN is present.
///   5. Create a PR if `create_pr` is true and owner/repo resolved.
///
/// Returns `None` in dry-run mode (nothing written to git).
/// Push/PR failures are non-fatal: the local commit is preserved.
pub async fn run_git_workflow(
    task: &str,
    summary: &str,
    change_log: &[FileChange],
    ctx: &RunContext,
    git_cfg: &GitConfig,
) -> Result<Option<GitOutcome>> {
    if ctx.dry_run {
        warn!(git = "workflow", "dry-run: skipping all git operations");
        return Ok(None);
    }

    let applied: Vec<&FileChange> = change_log.iter().filter(|c| c.applied).collect();
    if applied.is_empty() {
        info!(git = "workflow", "no applied changes; skipping git workflow");
        return Ok(None);
    }

    let cwd = std::env::current_dir().map_err(|e| {
        MaunsError::Git(format!("cannot determine working directory: {e}"))
    })?;

    let mut repo = GitRepo::open_or_init(&cwd)?;

    let ts     = chrono::Utc::now();
    let branch = branch_name(task, ts);

    repo.create_and_checkout(&branch)?;

    let commit_message = format!("{}{}", git_cfg.commit_prefix, task);
    let commit_id = stage_and_commit(&repo, change_log, &commit_message)?;

    info!(git = "workflow", branch = %branch, commit = %commit_id, "commit complete");

    let pr_url = attempt_push_and_pr(
        &repo,
        &branch,
        task,
        summary,
        change_log,
        git_cfg,
    )
    .await
    .unwrap_or_else(|e| {
        warn!(git = "workflow", error = %e, "push/PR step failed; local commit preserved");
        None
    });

    Ok(Some(GitOutcome { branch, commit_id, pr_url }))
}

async fn attempt_push_and_pr(
    repo: &GitRepo,
    branch: &str,
    task: &str,
    summary: &str,
    change_log: &[FileChange],
    git_cfg: &GitConfig,
) -> Result<Option<String>> {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            info!(git = "push", "GITHUB_TOKEN not set; skipping push and PR");
            return Ok(None);
        }
    };

    push_branch(repo, branch, &token)?;

    if !git_cfg.create_pr {
        info!(github = "pr", "create_pr is false; skipping PR creation");
        return Ok(None);
    }

    let remote_url = resolve_origin_url(repo)?;
    let (owner, repo_name) = match parse_remote_url(&remote_url) {
        Ok(pair) => pair,
        Err(e) => {
            warn!(github = "pr", error = %e, "cannot parse remote URL; skipping PR");
            return Ok(None);
        }
    };

    let gh_client = GitHubClient::new(token)?;

    let base = default_branch(&gh_client, &owner, &repo_name)
        .await
        .unwrap_or_else(|_| "main".to_string());

    let pr_req = PrRequest {
        owner,
        repo:        repo_name,
        head_branch: branch.to_string(),
        base_branch: base,
        task:        task.to_string(),
        summary:     summary.to_string(),
        change_log:  change_log.to_vec(),
    };

    let pr = create_pull_request(&gh_client, &pr_req).await?;
    Ok(Some(pr.html_url))
}

fn resolve_origin_url(repo: &GitRepo) -> Result<String> {
    let remote = repo
        .inner()
        .find_remote("origin")
        .map_err(|e| MaunsError::Git(format!("cannot find 'origin' remote: {e}")))?;

    remote
        .url()
        .map(|s| s.to_string())
        .ok_or_else(|| MaunsError::Git("origin remote has no URL".to_string()))
}
