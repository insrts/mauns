//! Project type and language detection.
//!
//! Scans the workspace root for known marker files to identify the
//! project type and primary language.  Results are stored in `RunContext`.

use std::path::Path;

/// Detected programming language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Unknown,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust       => write!(f, "Rust"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Python     => write!(f, "Python"),
            Language::Go         => write!(f, "Go"),
            Language::Unknown    => write!(f, "unknown"),
        }
    }
}

/// Detected project type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    CargoWorkspace,
    CargoCrate,
    NodePackage,
    PythonPackage,
    GoModule,
    Unknown,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::CargoWorkspace => write!(f, "Cargo workspace"),
            ProjectType::CargoCrate     => write!(f, "Cargo crate"),
            ProjectType::NodePackage    => write!(f, "Node.js package"),
            ProjectType::PythonPackage  => write!(f, "Python package"),
            ProjectType::GoModule       => write!(f, "Go module"),
            ProjectType::Unknown        => write!(f, "unknown"),
        }
    }
}

/// Detected project information.
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub language:     Language,
    pub project_type: ProjectType,
    /// Non-empty hint string injected into agent prompts.
    pub context_hint: String,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            language:     Language::Unknown,
            project_type: ProjectType::Unknown,
            context_hint: String::new(),
        }
    }
}

/// Detect the project type and language by scanning `root` for marker files.
pub fn detect(root: impl AsRef<Path>) -> ProjectInfo {
    let root = root.as_ref();

    // Rust — check workspace first (has [workspace] in Cargo.toml)
    if root.join("Cargo.toml").exists() {
        let proj = if is_cargo_workspace(root) {
            ProjectType::CargoWorkspace
        } else {
            ProjectType::CargoCrate
        };
        return ProjectInfo {
            language:     Language::Rust,
            project_type: proj.clone(),
            context_hint: format!(
                "This is a {proj}. Use cargo conventions: src/lib.rs for libraries, \
                 src/main.rs for binaries. Respect the existing module structure."
            ),
        };
    }

    // TypeScript (check before JS — tsconfig is more specific)
    if root.join("tsconfig.json").exists() || root.join("package.json").exists() {
        let lang = if root.join("tsconfig.json").exists() {
            Language::TypeScript
        } else {
            Language::JavaScript
        };
        return ProjectInfo {
            language: lang.clone(),
            project_type: ProjectType::NodePackage,
            context_hint: format!(
                "This is a {lang} Node.js project. Respect package.json scripts \
                 and existing import conventions."
            ),
        };
    }

    // Python
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        return ProjectInfo {
            language:     Language::Python,
            project_type: ProjectType::PythonPackage,
            context_hint: "This is a Python project. Follow PEP 8 conventions and \
                           respect the existing package structure."
                .to_string(),
        };
    }

    // Go
    if root.join("go.mod").exists() {
        return ProjectInfo {
            language:     Language::Go,
            project_type: ProjectType::GoModule,
            context_hint: "This is a Go module. Follow idiomatic Go conventions \
                           and respect the module path in go.mod."
                .to_string(),
        };
    }

    ProjectInfo::default()
}

fn is_cargo_workspace(root: &Path) -> bool {
    let cargo_toml = match std::fs::read_to_string(root.join("Cargo.toml")) {
        Ok(s)  => s,
        Err(_) => return false,
    };
    cargo_toml.contains("[workspace]")
}
