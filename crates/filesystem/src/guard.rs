//! Path validation layer — workspace confinement, blocklists, .maunsignore,
//! large-directory blocking, and file-size enforcement.

use std::path::{Component, Path, PathBuf};

use mauns_core::error::{MaunsError, Result};

use crate::ignore::IgnoreRules;

/// File size limit: files larger than this will not be read (default 1 MiB).
pub const DEFAULT_SIZE_LIMIT_BYTES: u64 = 1024 * 1024;

/// Directories that are always skipped regardless of .maunsignore.
const ALWAYS_SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".hg",
    "__pycache__",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "build",
    ".next",
    ".nuxt",
];

/// Files unconditionally blocked (credentials / secrets).
const BLOCKED_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.staging",
    ".env.development",
    ".npmrc",
    ".netrc",
];

/// A validated path confirmed safe to access.
#[derive(Debug, Clone)]
pub struct SafePath(PathBuf);

impl SafePath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl std::fmt::Display for SafePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// Validates and confines all path requests to a workspace root directory.
#[derive(Debug, Clone)]
pub struct PathGuard {
    workspace_root: PathBuf,
    allow_hidden: bool,
    size_limit: u64,
    ignore_rules: IgnoreRules,
}

impl PathGuard {
    /// Create a guard rooted at `workspace_root`.
    /// Loads `.maunsignore` from the root automatically.
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self> {
        let root = workspace_root.as_ref();
        let canonical = std::fs::canonicalize(root).map_err(|e| {
            MaunsError::Filesystem(format!(
                "cannot canonicalize workspace root '{}': {e}",
                root.display()
            ))
        })?;
        let ignore_rules = IgnoreRules::load(&canonical);
        Ok(Self {
            workspace_root: canonical,
            allow_hidden: false,
            size_limit: DEFAULT_SIZE_LIMIT_BYTES,
            ignore_rules,
        })
    }

    pub fn with_allow_hidden(mut self, allow: bool) -> Self {
        self.allow_hidden = allow;
        self
    }

    pub fn with_size_limit(mut self, bytes: u64) -> Self {
        self.size_limit = bytes;
        self
    }

    /// Validate a path for any access (read, write, delete, list).
    ///
    /// Checks in order:
    /// 1. No `..` traversal components.
    /// 2. Resolved path stays inside workspace_root.
    /// 3. No component matches the always-skip-dirs list.
    /// 4. Filename not in the absolute blocklist.
    /// 5. Hidden-file policy (unless allow_hidden).
    /// 6. .maunsignore rules.
    pub fn validate(&self, input: impl AsRef<Path>) -> Result<SafePath> {
        let input = input.as_ref();

        for component in input.components() {
            if component == Component::ParentDir {
                return Err(MaunsError::PathTraversal(format!(
                    "path '{}' contains a parent-directory traversal component",
                    input.display()
                )));
            }
        }

        let candidate = if input.is_absolute() {
            input.to_path_buf()
        } else {
            self.workspace_root.join(input)
        };

        let normalized = normalize_lexically(&candidate);

        if !normalized.starts_with(&self.workspace_root) {
            return Err(MaunsError::OutsideWorkspace {
                path: normalized.display().to_string(),
            });
        }

        // Check every component against always-skip-dirs.
        for component in normalized.components() {
            if let Some(s) = component.as_os_str().to_str() {
                for skipped in ALWAYS_SKIP_DIRS {
                    if s == *skipped {
                        return Err(MaunsError::RestrictedPath(format!(
                            "directory '{s}' is always excluded from workspace operations"
                        )));
                    }
                }
            }
        }

        // Filename-level checks.
        if let Some(name) = normalized.file_name().and_then(|n| n.to_str()) {
            for blocked in BLOCKED_NAMES {
                if name.eq_ignore_ascii_case(blocked) {
                    return Err(MaunsError::RestrictedPath(format!(
                        "access to '{name}' is unconditionally blocked"
                    )));
                }
            }

            if !self.allow_hidden && name.starts_with('.') && !is_allowed_hidden(name) {
                return Err(MaunsError::RestrictedPath(format!(
                    "access to hidden file '{name}' is blocked"
                )));
            }
        }

        // .maunsignore check — compute rel path for matching.
        let rel = normalized
            .strip_prefix(&self.workspace_root)
            .unwrap_or(&normalized);

        let is_dir = normalized.is_dir();
        if self.ignore_rules.is_ignored(rel, is_dir) {
            return Err(MaunsError::RestrictedPath(format!(
                "path '{}' is excluded by .maunsignore",
                rel.display()
            )));
        }

        Ok(SafePath(normalized))
    }

    /// Validate a path for reading and additionally enforce the file-size limit.
    pub fn validate_for_read(&self, input: impl AsRef<Path>) -> Result<SafePath> {
        let safe = self.validate(input)?;

        if safe.as_path().is_file() {
            let meta = std::fs::metadata(safe.as_path()).map_err(|e| {
                MaunsError::Filesystem(format!("cannot stat '{}': {e}", safe.as_path().display()))
            })?;
            if meta.len() > self.size_limit {
                return Err(MaunsError::Filesystem(format!(
                    "file '{}' ({} bytes) exceeds the size limit of {} bytes",
                    safe.as_path().display(),
                    meta.len(),
                    self.size_limit
                )));
            }
        }

        Ok(safe)
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn ignore_rules(&self) -> &IgnoreRules {
        &self.ignore_rules
    }
}

fn is_allowed_hidden(name: &str) -> bool {
    matches!(
        name,
        ".gitignore"
            | ".gitattributes"
            | ".editorconfig"
            | ".rustfmt.toml"
            | ".clippy.toml"
            | ".maunsignore"
    )
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn guard() -> PathGuard {
        PathGuard::new(env::current_dir().unwrap()).unwrap()
    }

    #[test]
    fn blocks_parent_traversal() {
        let err = guard().validate("../outside").unwrap_err();
        assert!(matches!(err, MaunsError::PathTraversal(_)));
    }

    #[test]
    fn blocks_env_file() {
        let err = guard().validate(".env").unwrap_err();
        assert!(matches!(err, MaunsError::RestrictedPath(_)));
    }

    #[test]
    fn blocks_node_modules() {
        let err = guard()
            .validate("node_modules/express/index.js")
            .unwrap_err();
        assert!(matches!(err, MaunsError::RestrictedPath(_)));
    }

    #[test]
    fn blocks_target_dir() {
        let err = guard().validate("target/debug/binary").unwrap_err();
        assert!(matches!(err, MaunsError::RestrictedPath(_)));
    }

    #[test]
    fn allows_maunsignore_itself() {
        // .maunsignore is in the allowed-hidden list
        let g = guard().with_allow_hidden(true);
        let result = g.validate(".maunsignore");
        if let Err(e) = result {
            assert!(!matches!(e, MaunsError::RestrictedPath(_)));
        }
    }

    #[test]
    fn allows_normal_path() {
        let result = guard().validate("src/main.rs");
        match result {
            Ok(p) => assert!(p.as_path().starts_with(guard().workspace_root())),
            Err(e) => panic!("unexpected: {e}"),
        }
    }
}
