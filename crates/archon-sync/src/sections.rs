/// Markers used for section-managed files.
const MANAGED_BEGIN: &str = "<!-- ARCHON:MANAGED:BEGIN -->";
const MANAGED_END: &str = "<!-- ARCHON:MANAGED:END -->";

/// Extract the managed section content from a file.
/// Returns the content between BEGIN and END markers (exclusive), or None if not found.
pub fn extract_managed_section(content: &str) -> Option<&str> {
    let begin_pos = content.find(MANAGED_BEGIN)?;
    let after_begin = begin_pos + MANAGED_BEGIN.len();
    // Skip the newline after BEGIN marker
    let content_start = if content[after_begin..].starts_with('\n') {
        after_begin + 1
    } else {
        after_begin
    };
    let end_pos = content.find(MANAGED_END)?;
    if end_pos <= content_start {
        return None;
    }
    Some(content[content_start..end_pos].trim_end_matches('\n'))
}

/// Replace the managed section in a file with new content.
/// Preserves everything outside the BEGIN/END markers.
/// Returns None if the file doesn't have managed section markers.
pub fn replace_managed_section(file_content: &str, new_section: &str) -> Option<String> {
    let begin_pos = file_content.find(MANAGED_BEGIN)?;
    let end_pos = file_content.find(MANAGED_END)?;
    if end_pos <= begin_pos {
        return None;
    }

    let before = &file_content[..begin_pos + MANAGED_BEGIN.len()];
    let after = &file_content[end_pos..];

    Some(format!("{before}\n{new_section}\n{after}"))
}

/// Check if a file has managed section markers.
pub fn has_managed_sections(content: &str) -> bool {
    content.contains(MANAGED_BEGIN) && content.contains(MANAGED_END)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# AGENTS.md

Project-specific content here.

<!-- ARCHON:MANAGED:BEGIN -->
Old managed content.
Line two.
<!-- ARCHON:MANAGED:END -->

More project content.
"#;

    #[test]
    fn extract_managed() {
        let section = extract_managed_section(SAMPLE).unwrap();
        assert_eq!(section, "Old managed content.\nLine two.");
    }

    #[test]
    fn replace_managed() {
        let result = replace_managed_section(SAMPLE, "New content here.").unwrap();
        assert!(result.contains("New content here."));
        assert!(result.contains("Project-specific content here."));
        assert!(result.contains("More project content."));
        assert!(!result.contains("Old managed content."));
    }

    #[test]
    fn has_markers() {
        assert!(has_managed_sections(SAMPLE));
        assert!(!has_managed_sections("no markers here"));
    }

    #[test]
    fn extract_no_markers() {
        assert!(extract_managed_section("no markers").is_none());
    }

    #[test]
    fn replace_no_markers() {
        assert!(replace_managed_section("no markers", "content").is_none());
    }

    #[test]
    fn round_trip_preserves_structure() {
        let new_content = "Updated rules.\n- Rule one\n- Rule two";
        let replaced = replace_managed_section(SAMPLE, new_content).unwrap();
        let extracted = extract_managed_section(&replaced).unwrap();
        assert_eq!(extracted, new_content);
    }
}
