use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::hasher;
use crate::manifest::{Ownership, SyncManifest};
use crate::sections;
use crate::state::SyncState;

/// Result of syncing a single file.
#[derive(Debug)]
pub struct SyncFileResult {
    pub path: String,
    pub action: SyncAction,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SyncAction {
    Updated,
    Unchanged,
    Skipped,
    Created,
    AiMerged,
    Error(String),
}

/// AI merge callback — provided by the CLI when an AI backend is available.
/// Takes (filename, arch_content, local_content) and returns merged content.
pub type AiMergeFn = Box<dyn Fn(&str, &str, &str) -> anyhow::Result<String>>;

/// Options for the sync engine.
pub struct SyncOptions {
    pub dry_run: bool,
    pub force: bool,
    /// Optional AI merge function for fully-managed files with local modifications.
    pub ai_merge: Option<AiMergeFn>,
}

/// Run the sync engine: compare source files in arch repo with downstream repo files.
pub fn run_sync(
    repo_root: &Path,
    arch_root: &Path,
    options: &SyncOptions,
) -> Result<Vec<SyncFileResult>> {
    let manifest_path = repo_root.join("sync-manifest.yaml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "no sync-manifest.yaml found in {}. Run `archon init` first.",
            repo_root.display()
        );
    }

    let manifest = SyncManifest::load(&manifest_path)?;
    let mut state = SyncState::load(repo_root);
    let mut results = Vec::new();

    for entry in &manifest.files {
        let result = match entry.ownership {
            Ownership::Unmanaged => SyncFileResult {
                path: entry.path.clone(),
                action: SyncAction::Skipped,
            },
            Ownership::FullyManaged => {
                sync_fully_managed(repo_root, arch_root, &entry.path, entry.source.as_deref(), &mut state, options)
            }
            Ownership::SectionManaged => {
                sync_section_managed(repo_root, arch_root, &entry.path, entry.source.as_deref(), &mut state, options)
            }
        };
        results.push(result);
    }

    if !options.dry_run {
        state.save(repo_root).context("saving sync state")?;
    }

    Ok(results)
}

fn sync_fully_managed(
    repo_root: &Path,
    arch_root: &Path,
    rel_path: &str,
    source: Option<&str>,
    state: &mut SyncState,
    options: &SyncOptions,
) -> SyncFileResult {
    let source_rel = match source {
        Some(s) => s,
        None => {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error("no source specified for fully-managed file".into()),
            }
        }
    };

    let source_path = arch_root.join(source_rel);
    let dest_path = repo_root.join(rel_path);

    let source_content = match std::fs::read_to_string(&source_path) {
        Ok(c) => c,
        Err(e) => {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error(format!("reading source {source_rel}: {e}")),
            }
        }
    };

    let source_hash = hasher::hash_bytes(source_content.as_bytes());

    // Check if unchanged since last sync
    if !options.force {
        if let Some(last) = state.last_hash(rel_path) {
            if last == source_hash {
                return SyncFileResult {
                    path: rel_path.to_string(),
                    action: SyncAction::Unchanged,
                };
            }
        }
    }

    // Check if the local file exists and has been modified since last sync
    let (action, final_content) = if dest_path.exists() {
        let local_content = std::fs::read_to_string(&dest_path).unwrap_or_default();
        let local_hash = hasher::hash_bytes(local_content.as_bytes());
        let locally_modified = state
            .last_hash(rel_path)
            .is_some_and(|last| last != local_hash);

        if locally_modified {
            // Local file was edited since last sync — try AI merge if available
            if let Some(ai_merge) = &options.ai_merge {
                match ai_merge(rel_path, &source_content, &local_content) {
                    Ok(merged) => (SyncAction::AiMerged, merged),
                    Err(e) => {
                        eprintln!(
                            "  {} AI merge failed for {}: {e} — overwriting",
                            "warning:".yellow(),
                            rel_path
                        );
                        (SyncAction::Updated, source_content)
                    }
                }
            } else {
                (SyncAction::Updated, source_content)
            }
        } else {
            (SyncAction::Updated, source_content)
        }
    } else {
        (SyncAction::Created, source_content)
    };

    if !options.dry_run {
        if let Some(parent) = dest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&dest_path, &final_content) {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error(format!("writing {rel_path}: {e}")),
            };
        }
        state.record(rel_path, &source_hash);
    }

    SyncFileResult {
        path: rel_path.to_string(),
        action,
    }
}

fn sync_section_managed(
    repo_root: &Path,
    arch_root: &Path,
    rel_path: &str,
    source: Option<&str>,
    state: &mut SyncState,
    options: &SyncOptions,
) -> SyncFileResult {
    let source_rel = match source {
        Some(s) => s,
        None => {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error("no source specified for section-managed file".into()),
            }
        }
    };

    let source_path = arch_root.join(source_rel);
    let dest_path = repo_root.join(rel_path);

    // Read new managed section content from arch repo
    let source_content = match std::fs::read_to_string(&source_path) {
        Ok(c) => c,
        Err(e) => {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error(format!("reading source {source_rel}: {e}")),
            }
        }
    };

    let source_hash = hasher::hash_bytes(source_content.as_bytes());

    // Check if unchanged since last sync
    if !options.force {
        if let Some(last) = state.last_hash(rel_path) {
            if last == source_hash {
                return SyncFileResult {
                    path: rel_path.to_string(),
                    action: SyncAction::Unchanged,
                };
            }
        }
    }

    // Read existing downstream file
    let dest_content = match std::fs::read_to_string(&dest_path) {
        Ok(c) => c,
        Err(_) => {
            return SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Error(format!("{rel_path} not found — create it with `archon init` first")),
            }
        }
    };

    // Replace the managed section
    let new_section_content = source_content.trim();
    match sections::replace_managed_section(&dest_content, new_section_content) {
        Some(updated) => {
            if !options.dry_run {
                if let Err(e) = std::fs::write(&dest_path, &updated) {
                    return SyncFileResult {
                        path: rel_path.to_string(),
                        action: SyncAction::Error(format!("writing {rel_path}: {e}")),
                    };
                }
                state.record(rel_path, &source_hash);
            }
            SyncFileResult {
                path: rel_path.to_string(),
                action: SyncAction::Updated,
            }
        }
        None => SyncFileResult {
            path: rel_path.to_string(),
            action: SyncAction::Error(format!(
                "{rel_path} has no ARCHON:MANAGED markers — add them first"
            )),
        },
    }
}

/// Print sync results to stderr.
pub fn print_results(results: &[SyncFileResult]) {
    for r in results {
        match &r.action {
            SyncAction::Updated => eprintln!("  {} {}", "updated".green(), r.path),
            SyncAction::Created => eprintln!("  {} {}", "created".green(), r.path),
            SyncAction::AiMerged => eprintln!("  {} {} (AI merged)", "updated".green(), r.path),
            SyncAction::Unchanged => eprintln!("  {} {}", "unchanged".dimmed(), r.path),
            SyncAction::Skipped => eprintln!("  {} {} (unmanaged)", "skipped".dimmed(), r.path),
            SyncAction::Error(e) => eprintln!("  {} {} ({})", "error".red(), r.path, e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_sync_test() -> (tempfile::TempDir, tempfile::TempDir) {
        let arch = tempdir().unwrap();
        let repo = tempdir().unwrap();

        // Create arch repo source file
        std::fs::create_dir_all(arch.path().join("templates")).unwrap();
        std::fs::write(
            arch.path().join("templates/upstream.md"),
            "# Upstream Architecture\n\nShared content.",
        )
        .unwrap();

        std::fs::write(
            arch.path().join("templates/agents-section.md"),
            "## Ecosystem Rules\n- Use hologram- prefix\n- Follow ADRs",
        )
        .unwrap();

        // Create sync manifest in repo
        std::fs::write(
            repo.path().join("sync-manifest.yaml"),
            r#"
version: "2026.03"
files:
  - path: upstream.md
    ownership: fully-managed
    source: templates/upstream.md
  - path: AGENTS.md
    ownership: section-managed
    source: templates/agents-section.md
  - path: local.md
    ownership: unmanaged
"#,
        )
        .unwrap();

        // Create existing AGENTS.md with markers
        std::fs::write(
            repo.path().join("AGENTS.md"),
            "# AGENTS.md\n\nProject stuff.\n\n<!-- ARCHON:MANAGED:BEGIN -->\nOld content\n<!-- ARCHON:MANAGED:END -->\n",
        )
        .unwrap();

        (arch, repo)
    }

    #[test]
    fn sync_fully_managed_creates_file() {
        let (arch, repo) = setup_sync_test();
        let options = SyncOptions {
            dry_run: false,
            force: false,
            ai_merge: None,
        };
        let results = run_sync(repo.path(), arch.path(), &options).unwrap();

        assert_eq!(results[0].action, SyncAction::Created); // upstream.md created
        let content = std::fs::read_to_string(repo.path().join("upstream.md")).unwrap();
        assert!(content.contains("Shared content."));
    }

    #[test]
    fn sync_section_managed_replaces_section() {
        let (arch, repo) = setup_sync_test();
        let options = SyncOptions {
            dry_run: false,
            force: false,
            ai_merge: None,
        };
        let results = run_sync(repo.path(), arch.path(), &options).unwrap();

        assert_eq!(results[1].action, SyncAction::Updated); // AGENTS.md updated
        let content = std::fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
        assert!(content.contains("Project stuff.")); // preserved
        assert!(content.contains("Use hologram- prefix")); // new managed content
        assert!(!content.contains("Old content")); // replaced
    }

    #[test]
    fn sync_unmanaged_skipped() {
        let (arch, repo) = setup_sync_test();
        let options = SyncOptions {
            dry_run: false,
            force: false,
            ai_merge: None,
        };
        let results = run_sync(repo.path(), arch.path(), &options).unwrap();
        assert_eq!(results[2].action, SyncAction::Skipped);
    }

    #[test]
    fn sync_dry_run_no_writes() {
        let (arch, repo) = setup_sync_test();
        let options = SyncOptions {
            dry_run: true,
            force: false,
            ai_merge: None,
        };
        let results = run_sync(repo.path(), arch.path(), &options).unwrap();

        assert_eq!(results[0].action, SyncAction::Created);
        // File should NOT actually exist
        assert!(!repo.path().join("upstream.md").exists());
    }

    #[test]
    fn sync_unchanged_after_second_run() {
        let (arch, repo) = setup_sync_test();
        let options = SyncOptions {
            dry_run: false,
            force: false,
            ai_merge: None,
        };
        run_sync(repo.path(), arch.path(), &options).unwrap();
        let results = run_sync(repo.path(), arch.path(), &options).unwrap();

        assert_eq!(results[0].action, SyncAction::Unchanged); // upstream.md unchanged
        assert_eq!(results[1].action, SyncAction::Unchanged); // AGENTS.md unchanged
    }
}
