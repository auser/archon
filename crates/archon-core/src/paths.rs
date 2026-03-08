use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::repo_meta::RepoMeta;

/// The standard repo metadata filename.
pub const REPO_META_FILENAME: &str = "hologram.repo.yaml";

/// Discover hologram.repo.yaml by walking up from `start` directory.
/// Returns the repo root directory and parsed metadata.
pub fn find_repo_meta(start: &Path) -> Result<Option<(PathBuf, RepoMeta)>> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(REPO_META_FILENAME);
        if candidate.is_file() {
            let contents = std::fs::read_to_string(&candidate)
                .with_context(|| format!("reading {}", candidate.display()))?;
            let meta: RepoMeta = serde_yaml::from_str(&contents)
                .with_context(|| format!("parsing {}", candidate.display()))?;
            return Ok(Some((current, meta)));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

/// Resolve the architecture repo root from an explicit path, env var, or sibling detection.
pub fn resolve_arch_root(explicit: Option<&str>) -> Result<Option<PathBuf>> {
    // 1. Explicit --arch-root flag
    if let Some(path) = explicit {
        let p = PathBuf::from(path);
        if p.is_dir() {
            return Ok(Some(p));
        }
        anyhow::bail!("arch-root path does not exist: {path}");
    }

    // 2. ARCHON_ROOT env var
    if let Ok(path) = std::env::var("ARCHON_ROOT") {
        let p = PathBuf::from(&path);
        if p.is_dir() {
            return Ok(Some(p));
        }
    }

    // 3. Try sibling directory detection (../hologram-architecture)
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(parent) = cwd.parent() {
            let sibling = parent.join("hologram-architecture");
            if sibling.is_dir() {
                return Ok(Some(sibling));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn find_repo_meta_in_current_dir() {
        let dir = tempdir().unwrap();
        let meta_path = dir.path().join(REPO_META_FILENAME);
        std::fs::write(
            &meta_path,
            r#"
name: test-repo
role: tool
repo_type: rust-binary
standards_version: "2026.03"
"#,
        )
        .unwrap();

        let result = find_repo_meta(dir.path()).unwrap();
        assert!(result.is_some());
        let (root, meta) = result.unwrap();
        assert_eq!(root, dir.path());
        assert_eq!(meta.name, "test-repo");
    }

    #[test]
    fn find_repo_meta_walks_up() {
        let dir = tempdir().unwrap();
        let meta_path = dir.path().join(REPO_META_FILENAME);
        std::fs::write(
            &meta_path,
            r#"
name: parent-repo
role: core
repo_type: rust-workspace
standards_version: "2026.03"
"#,
        )
        .unwrap();

        let subdir = dir.path().join("crates").join("sub-crate");
        std::fs::create_dir_all(&subdir).unwrap();

        let result = find_repo_meta(&subdir).unwrap();
        assert!(result.is_some());
        let (root, meta) = result.unwrap();
        assert_eq!(root, dir.path());
        assert_eq!(meta.name, "parent-repo");
    }

    #[test]
    fn find_repo_meta_missing() {
        let dir = tempdir().unwrap();
        let result = find_repo_meta(dir.path()).unwrap();
        assert!(result.is_none());
    }
}
