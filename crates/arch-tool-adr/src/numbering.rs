use std::path::Path;

use anyhow::Result;

/// Scan the ADR directory and return the next available number.
pub fn next_number(adr_dir: &Path) -> Result<u32> {
    if !adr_dir.is_dir() {
        return Ok(1);
    }

    let max = std::fs::read_dir(adr_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // Parse NNNN from "NNNN-title.md"
            name.split('-').next()?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);

    Ok(max + 1)
}

/// Format a number as a zero-padded 4-digit string.
pub fn format_number(n: u32) -> String {
    format!("{n:04}")
}

/// Convert a title to a filename-safe slug.
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn next_number_empty_dir() {
        let dir = tempdir().unwrap();
        let adr_dir = dir.path().join("adrs");
        std::fs::create_dir(&adr_dir).unwrap();
        assert_eq!(next_number(&adr_dir).unwrap(), 1);
    }

    #[test]
    fn next_number_with_existing() {
        let dir = tempdir().unwrap();
        let adr_dir = dir.path().join("adrs");
        std::fs::create_dir(&adr_dir).unwrap();
        std::fs::write(adr_dir.join("0001-first.md"), "").unwrap();
        std::fs::write(adr_dir.join("0002-second.md"), "").unwrap();
        assert_eq!(next_number(&adr_dir).unwrap(), 3);
    }

    #[test]
    fn next_number_nonexistent_dir() {
        let dir = tempdir().unwrap();
        assert_eq!(next_number(&dir.path().join("nope")).unwrap(), 1);
    }

    #[test]
    fn format_padded() {
        assert_eq!(format_number(1), "0001");
        assert_eq!(format_number(42), "0042");
        assert_eq!(format_number(9999), "9999");
    }

    #[test]
    fn slugify_title() {
        assert_eq!(slugify("Use YAML for all configs"), "use-yaml-for-all-configs");
        assert_eq!(slugify("ADR: repo boundaries!"), "adr-repo-boundaries");
    }
}
