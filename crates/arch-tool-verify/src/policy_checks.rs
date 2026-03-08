use arch_tool_core::repo_meta::RepoMeta;
use arch_tool_core::standards_version::StandardsVersion;

/// Validate that the standards_version field is present and well-formed.
pub fn validate_standards_version(meta: &RepoMeta) -> Result<(), String> {
    StandardsVersion::parse(&meta.standards_version.0)
        .map(|_| ())
        .map_err(|e| format!("invalid standards_version: {e}"))
}

/// Validate that required metadata fields are populated.
pub fn validate_required_fields(meta: &RepoMeta) -> Vec<String> {
    let mut issues = Vec::new();

    if meta.name.is_empty() {
        issues.push("name is empty".to_string());
    }

    if meta.owners.is_empty() {
        issues.push("owners list is empty".to_string());
    }

    issues
}
