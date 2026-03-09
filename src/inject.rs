const MANAGED_BEGIN: &str = "<!-- ARCHON:CONTEXT:BEGIN -->";
const MANAGED_END: &str = "<!-- ARCHON:CONTEXT:END -->";

/// Replace the managed context section in a file with new content.
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

/// Return the marker pair for inserting into new files.
pub fn empty_section_markers() -> String {
    format!("{MANAGED_BEGIN}\n{MANAGED_END}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# CLAUDE.md

Project-specific content here.

<!-- ARCHON:CONTEXT:BEGIN -->
Old managed content.
Line two.
<!-- ARCHON:CONTEXT:END -->

More project content.
"#;

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
    fn replace_no_markers() {
        assert!(replace_managed_section("no markers", "content").is_none());
    }

    #[test]
    fn round_trip_preserves_structure() {
        let new_content = "Updated rules.\n- Rule one\n- Rule two";
        let replaced = replace_managed_section(SAMPLE, new_content).unwrap();
        assert!(replaced.contains(new_content));
        assert!(replaced.contains("Project-specific content here."));
    }
}
