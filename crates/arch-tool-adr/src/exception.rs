use std::path::Path;

use anyhow::{Context, Result};

use arch_tool_core::repo_meta::{ExceptionRef, RepoMeta};

/// Add a new exception to a repo's hologram.repo.yaml.
/// Returns the generated exception ID.
pub fn add_exception(
    repo_root: &Path,
    rule: &str,
    reason: &str,
    expires: Option<&str>,
) -> Result<String> {
    let meta_path = repo_root.join("hologram.repo.yaml");
    let contents = std::fs::read_to_string(&meta_path)
        .context("reading hologram.repo.yaml — run `arch-tool init` first")?;

    let mut meta: RepoMeta =
        serde_yaml::from_str(&contents).context("parsing hologram.repo.yaml")?;

    let year = chrono::Utc::now().format("%Y").to_string();
    let next_num = meta.exceptions.len() + 1;
    let id = format!("EXC-{year}-{next_num:03}");

    meta.exceptions.push(ExceptionRef {
        id: id.clone(),
        rule: rule.to_string(),
        expires: expires.map(|s| s.to_string()),
    });

    let yaml = serde_yaml::to_string(&meta).context("serializing hologram.repo.yaml")?;
    std::fs::write(&meta_path, yaml).context("writing hologram.repo.yaml")?;

    // Also store the reason in a sidecar file for documentation
    let exc_dir = repo_root.join(".arch-tool/exceptions");
    std::fs::create_dir_all(&exc_dir)?;
    let exc_file = exc_dir.join(format!("{id}.yaml"));
    let record = format!(
        "id: {id}\nrule: {rule}\nreason: {reason}\nexpires: {expires}\ncreated: {created}\n",
        expires = expires.unwrap_or("never"),
        created = chrono::Utc::now().format("%Y-%m-%d"),
    );
    std::fs::write(&exc_file, record)?;

    Ok(id)
}

/// List all exceptions from a repo's hologram.repo.yaml.
pub fn list_exceptions(repo_root: &Path) -> Result<Vec<ExceptionRef>> {
    let meta_path = repo_root.join("hologram.repo.yaml");
    let contents = std::fs::read_to_string(&meta_path)
        .context("reading hologram.repo.yaml")?;
    let meta: RepoMeta =
        serde_yaml::from_str(&contents).context("parsing hologram.repo.yaml")?;
    Ok(meta.exceptions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo(dir: &Path) {
        let yaml = r#"
name: test-repo
role: tool
repo_type: rust-binary
standards_version: "2026.03"
"#;
        std::fs::write(dir.join("hologram.repo.yaml"), yaml).unwrap();
    }

    #[test]
    fn add_exception_to_repo() {
        let dir = tempdir().unwrap();
        create_test_repo(dir.path());

        let id = add_exception(
            dir.path(),
            "STR-003",
            "Legacy layout, migrating Q2",
            Some("2026-06-01"),
        )
        .unwrap();

        assert!(id.starts_with("EXC-"));

        let exceptions = list_exceptions(dir.path()).unwrap();
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].rule, "STR-003");
        assert_eq!(exceptions[0].expires.as_deref(), Some("2026-06-01"));
    }

    #[test]
    fn list_empty_exceptions() {
        let dir = tempdir().unwrap();
        create_test_repo(dir.path());

        let exceptions = list_exceptions(dir.path()).unwrap();
        assert!(exceptions.is_empty());
    }

    #[test]
    fn add_multiple_exceptions() {
        let dir = tempdir().unwrap();
        create_test_repo(dir.path());

        add_exception(dir.path(), "STR-001", "reason 1", None).unwrap();
        add_exception(dir.path(), "STR-002", "reason 2", Some("2026-12-01")).unwrap();

        let exceptions = list_exceptions(dir.path()).unwrap();
        assert_eq!(exceptions.len(), 2);
    }
}
