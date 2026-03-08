use std::path::Path;

use anyhow::{Context, Result};

use crate::model::AdrStatus;
use crate::numbering;

/// Create a new ADR file in the given directory.
/// Returns the relative path of the created file.
pub fn create_adr(
    adr_dir: &Path,
    title: &str,
    status: AdrStatus,
) -> Result<String> {
    std::fs::create_dir_all(adr_dir)
        .with_context(|| format!("creating ADR directory {}", adr_dir.display()))?;

    let number = numbering::next_number(adr_dir)?;
    let slug = numbering::slugify(title);
    let filename = format!("{}-{slug}.md", numbering::format_number(number));
    let path = adr_dir.join(&filename);

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let content = format!(
        r#"# ADR-{number:04}: {title}

## Status

{status}

## Date

{today}

## Context

<!-- Describe the context and problem that led to this decision. -->

## Decision

<!-- Describe the decision that was made. -->

## Consequences

<!-- Describe the consequences of this decision, both positive and negative. -->
"#
    );

    std::fs::write(&path, &content)
        .with_context(|| format!("writing ADR {}", path.display()))?;

    Ok(filename)
}

/// List existing ADRs by scanning the directory.
pub fn list_adrs(adr_dir: &Path) -> Result<Vec<AdrSummary>> {
    if !adr_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut adrs: Vec<AdrSummary> = std::fs::read_dir(adr_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "md")
        })
        .filter_map(|e| {
            let filename = e.file_name().to_string_lossy().to_string();
            let number: u32 = filename.split('-').next()?.parse().ok()?;
            // Read first line for title
            let content = std::fs::read_to_string(e.path()).ok()?;
            let title = content
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches('#')
                .trim()
                .to_string();
            Some(AdrSummary {
                number,
                filename,
                title,
            })
        })
        .collect();

    adrs.sort_by_key(|a| a.number);
    Ok(adrs)
}

#[derive(Debug)]
pub struct AdrSummary {
    pub number: u32,
    pub filename: String,
    pub title: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_adr_file() {
        let dir = tempdir().unwrap();
        let adr_dir = dir.path().join("specs/adrs");
        let filename = create_adr(&adr_dir, "Use YAML for configs", AdrStatus::Proposed).unwrap();

        assert_eq!(filename, "0001-use-yaml-for-configs.md");
        let content = std::fs::read_to_string(adr_dir.join(&filename)).unwrap();
        assert!(content.contains("ADR-0001: Use YAML for configs"));
        assert!(content.contains("Proposed"));
    }

    #[test]
    fn create_increments_number() {
        let dir = tempdir().unwrap();
        let adr_dir = dir.path().join("specs/adrs");
        create_adr(&adr_dir, "First", AdrStatus::Accepted).unwrap();
        let second = create_adr(&adr_dir, "Second", AdrStatus::Proposed).unwrap();
        assert!(second.starts_with("0002"));
    }

    #[test]
    fn list_adrs_empty() {
        let dir = tempdir().unwrap();
        let result = list_adrs(&dir.path().join("nonexistent")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_adrs_finds_files() {
        let dir = tempdir().unwrap();
        let adr_dir = dir.path().join("specs/adrs");
        create_adr(&adr_dir, "First decision", AdrStatus::Accepted).unwrap();
        create_adr(&adr_dir, "Second decision", AdrStatus::Proposed).unwrap();

        let list = list_adrs(&adr_dir).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].number, 1);
        assert_eq!(list[1].number, 2);
    }
}
