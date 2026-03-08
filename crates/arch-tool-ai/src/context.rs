use std::path::Path;

/// Context gathered from the target project for AI relevance analysis.
pub struct ProjectContext {
    pub repo_name: String,
    /// Contents of Cargo.toml, or "(not found)".
    pub cargo_toml: String,
    /// Notable existing files (relative paths).
    pub existing_files: Vec<String>,
}

impl ProjectContext {
    /// Build project context from a repository root.
    pub fn from_repo(repo_root: &Path) -> Self {
        let repo_name = repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let cargo_toml = std::fs::read_to_string(repo_root.join("Cargo.toml"))
            .unwrap_or_else(|_| "(not found)".to_string());

        let notable = [
            "AGENTS.md",
            "CLAUDE.md",
            "README.md",
            "hologram.repo.yaml",
            "specs/docs/architecture.md",
        ];
        let existing_files: Vec<String> = notable
            .iter()
            .filter(|f| repo_root.join(f).exists())
            .map(|f| f.to_string())
            .collect();

        Self {
            repo_name,
            cargo_toml,
            existing_files,
        }
    }
}

/// Context shared across per-file AI fill operations.
/// Load once with `FillContext::load(arch_root)` before looping over files.
pub struct FillContext {
    pub adrs_context: String,
    pub example_project: String,
}

impl FillContext {
    pub fn load(arch_root: &Path) -> Self {
        Self {
            adrs_context: glob_read(&arch_root.join("specs/adrs/*.md"), 400),
            example_project: read_file(&arch_root.join("specs/projects/hologram-ai/README.md")),
        }
    }
}

fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|_| "(not found)".into())
}

fn glob_read(pattern: &Path, max_lines: usize) -> String {
    let pat = pattern.to_string_lossy();
    glob::glob(&pat)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|s| s.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}
