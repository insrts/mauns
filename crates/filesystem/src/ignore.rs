//! `.maunsignore` parser and matcher.
//!
//! Syntax mirrors `.gitignore`:
//!   - Blank lines and lines starting with `#` are ignored.
//!   - A leading `!` negates the pattern (un-ignores).
//!   - `*` matches any sequence of characters except `/`.
//!   - `**` matches any sequence including `/`.
//!   - A trailing `/` forces directory-only matching.
//!   - All other characters are literal.
//!
//! `.maunsignore` overrides everything — it is checked before PathGuard's
//! own hidden-file and blocklist rules inside every I/O operation.

use std::path::Path;

use tracing::debug;

/// A compiled ignore ruleset loaded from a `.maunsignore` file.
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    rules: Vec<IgnoreRule>,
}

#[derive(Debug, Clone)]
struct IgnoreRule {
    pattern: String,
    negated: bool,
    dir_only: bool,
}

impl IgnoreRules {
    /// Load rules from `root/.maunsignore`.  If the file does not exist,
    /// returns an empty ruleset (no paths are ignored).
    pub fn load(root: impl AsRef<Path>) -> Self {
        let path = root.as_ref().join(".maunsignore");
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                debug!(ignore = "load", path = %path.display(), "loaded .maunsignore");
                Self::parse(&text)
            }
            Err(_) => {
                debug!(ignore = "load", "no .maunsignore found; no paths ignored");
                Self::default()
            }
        }
    }

    /// Parse ignore rules from a string (for testing and embedding).
    pub fn parse(text: &str) -> Self {
        let rules = text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }
                let (negated, rest) = if let Some(stripped) = line.strip_prefix('!') {
                    (true, stripped)
                } else {
                    (false, line)
                };
                let (dir_only, pattern) = if rest.ends_with('/') {
                    (true, rest.trim_end_matches('/').to_string())
                } else {
                    (false, rest.to_string())
                };
                if pattern.is_empty() {
                    return None;
                }
                Some(IgnoreRule {
                    pattern,
                    negated,
                    dir_only,
                })
            })
            .collect();
        Self { rules }
    }

    /// Returns `true` if `rel_path` (relative to workspace root) should be
    /// ignored.  Matching is evaluated in declaration order; the last matching
    /// rule wins (same semantics as `.gitignore`).
    pub fn is_ignored(&self, rel_path: impl AsRef<Path>, is_dir: bool) -> bool {
        let rel = rel_path.as_ref();
        let mut ignored = false;

        for rule in &self.rules {
            if rule.dir_only && !is_dir {
                continue;
            }
            if pattern_matches(&rule.pattern, rel) {
                ignored = !rule.negated;
            }
        }

        ignored
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Match `rel_path` against a single glob pattern.
///
/// Matching rules:
/// - If the pattern contains no `/`, match against the final component only.
/// - `*`  matches any char sequence except `/`.
/// - `**` matches any char sequence including `/`.
/// - All other characters match literally.
fn pattern_matches(pattern: &str, rel_path: &Path) -> bool {
    let path_str = rel_path.to_string_lossy();

    // If pattern has no slash, match against each path component individually
    // (last-component semantics, like gitignore).
    if !pattern.contains('/') {
        // Match against the full relative path string too (handles "node_modules").
        if glob_match(pattern, &path_str) {
            return true;
        }
        // Also match against each individual component.
        for component in rel_path.components() {
            if let Some(s) = component.as_os_str().to_str() {
                if glob_match(pattern, s) {
                    return true;
                }
            }
        }
        return false;
    }

    // Pattern contains `/`: match against the full relative path.
    let pat = pattern.trim_start_matches('/');
    glob_match(pat, &path_str)
}

/// Minimal glob matcher supporting `*` and `**`.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_bytes(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_bytes(pat: &[u8], text: &[u8]) -> bool {
    match (pat.first(), text.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(b'*'), _) => {
            // Check for `**`
            if pat.get(1) == Some(&b'*') {
                let rest_pat = &pat[2..];
                // `**` matches zero or more path segments
                for i in 0..=text.len() {
                    if glob_match_bytes(rest_pat.strip_prefix(b"/").unwrap_or(rest_pat), &text[i..])
                    {
                        return true;
                    }
                    if i < text.len() && text[i] == b'/' {
                        // continue scanning
                    }
                }
                return false;
            }
            // Single `*`: match any char except `/`
            let rest_pat = &pat[1..];
            for i in 0..=text.len() {
                if i > 0 && text[i - 1] == b'/' {
                    break;
                }
                if glob_match_bytes(rest_pat, &text[i..]) {
                    return true;
                }
            }
            false
        }
        (Some(p), Some(t)) => {
            if p == t {
                glob_match_bytes(&pat[1..], &text[1..])
            } else {
                false
            }
        }
        (Some(_), None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> IgnoreRules {
        IgnoreRules::parse(text)
    }

    #[test]
    fn ignores_exact_filename() {
        let r = rules("secret.txt\n");
        assert!(r.is_ignored("secret.txt", false));
        assert!(!r.is_ignored("other.txt", false));
    }

    #[test]
    fn ignores_directory_name() {
        let r = rules("node_modules\n");
        assert!(r.is_ignored("node_modules", true));
        assert!(r.is_ignored("frontend/node_modules", true));
    }

    #[test]
    fn wildcard_extension() {
        let r = rules("*.log\n");
        assert!(r.is_ignored("app.log", false));
        assert!(r.is_ignored("logs/app.log", false));
        assert!(!r.is_ignored("app.txt", false));
    }

    #[test]
    fn negation_un_ignores() {
        let r = rules("*.log\n!important.log\n");
        assert!(r.is_ignored("app.log", false));
        assert!(!r.is_ignored("important.log", false));
    }

    #[test]
    fn comments_and_blanks_ignored() {
        let r = rules("# this is a comment\n\nsecret.txt\n");
        assert!(r.is_ignored("secret.txt", false));
    }

    #[test]
    fn dir_only_pattern() {
        let r = rules("build/\n");
        assert!(r.is_ignored("build", true));
        assert!(!r.is_ignored("build", false)); // not a dir
    }

    #[test]
    fn double_star() {
        let r = rules("**/fixtures/**\n");
        assert!(r.is_ignored("tests/fixtures/data.json", false));
    }
}
