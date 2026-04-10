//! Git safety enforcement.
//!
//! These rules are enforced in Rust and cannot be overridden by AGENTS.md
//! content or any LLM output.

use mauns_core::error::{MaunsError, Result};

/// Branch names that are unconditionally protected from direct commits.
const PROTECTED_BRANCHES: &[&str] = &[
    "main",
    "master",
    "develop",
    "release",
    "production",
    "staging",
];

/// File names that are never committed even if they pass the path guard.
const COMMIT_BLOCKLIST: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.staging",
    ".env.development",
    ".npmrc",
    ".netrc",
];

/// Assert that `branch` is not a protected branch.
/// Returns `Err` if the branch is protected; Ok otherwise.
pub fn assert_not_protected(branch: &str) -> Result<()> {
    let lower = branch.to_lowercase();
    for protected in PROTECTED_BRANCHES {
        if lower == *protected || lower.starts_with(&format!("{protected}/")) {
            return Err(MaunsError::Git(format!(
                "branch '{branch}' is protected and cannot be committed to directly; \
                 mauns always creates a dedicated branch"
            )));
        }
    }
    Ok(())
}

/// Assert that `filename` (the final component of a path) is not in the
/// commit blocklist.
pub fn assert_not_blocked(filename: &str) -> Result<()> {
    let lower = filename.to_lowercase();
    for blocked in COMMIT_BLOCKLIST {
        if lower == *blocked {
            return Err(MaunsError::Git(format!(
                "file '{filename}' is on the commit blocklist and will never be committed"
            )));
        }
    }
    // Hidden files that are not explicitly allowed are also blocked.
    if lower.starts_with('.') && !is_explicitly_allowed(&lower) {
        return Err(MaunsError::Git(format!(
            "hidden file '{filename}' is blocked from commits; \
             add it to the explicit allow list to permit it"
        )));
    }
    Ok(())
}

fn is_explicitly_allowed(name: &str) -> bool {
    matches!(
        name,
        ".gitignore" | ".gitattributes" | ".editorconfig" | ".rustfmt.toml" | ".clippy.toml"
    )
}

/// Build the branch name from a task description and a UTC timestamp.
/// Format: mauns/<slug>-<timestamp>
/// The slug is the first 40 characters of the task, lowercased,
/// with non-alphanumeric characters replaced by hyphens.
pub fn branch_name(task: &str, ts: chrono::DateTime<chrono::Utc>) -> String {
    let slug: String = task
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(40)
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    // Collapse consecutive hyphens.
    let mut prev_hyphen = false;
    let slug: String = slug
        .chars()
        .filter(|&c| {
            if c == '-' {
                if prev_hyphen {
                    return false;
                }
                prev_hyphen = true;
            } else {
                prev_hyphen = false;
            }
            true
        })
        .collect();

    format!("mauns/{}-{}", slug, ts.format("%Y%m%dT%H%M%SZ"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_branch_is_blocked() {
        assert!(assert_not_protected("main").is_err());
        assert!(assert_not_protected("master").is_err());
        assert!(assert_not_protected("production").is_err());
    }

    #[test]
    fn mauns_branch_is_permitted() {
        assert!(assert_not_protected("mauns/fix-thing-20240101T000000Z").is_ok());
    }

    #[test]
    fn env_file_is_blocked() {
        assert!(assert_not_blocked(".env").is_err());
        assert!(assert_not_blocked(".env.local").is_err());
    }

    #[test]
    fn gitignore_is_allowed() {
        assert!(assert_not_blocked(".gitignore").is_ok());
    }

    #[test]
    fn branch_name_format() {
        use chrono::TimeZone;
        let ts = chrono::Utc
            .with_ymd_and_hms(2024, 1, 15, 10, 30, 0)
            .unwrap();
        let name = branch_name("Fix the login bug", ts);
        assert!(name.starts_with("mauns/"));
        assert!(name.contains("20240115T103000Z"));
    }
}
