use serde::{Deserialize, Serialize};

/// Sync manifest loaded from `sync-manifest.yaml` in a downstream repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub version: String,
    #[serde(default)]
    pub files: Vec<SyncEntry>,
}

/// A single file entry in the sync manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    /// Relative path in the downstream repo.
    pub path: String,
    /// How archon manages this file.
    pub ownership: Ownership,
    /// Source path relative to the architecture repo root (for managed files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// File ownership model — controls how `archon sync` handles each file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ownership {
    /// Entire file is owned by archon and overwritten on sync.
    FullyManaged,
    /// Only content between ARCHON:MANAGED markers is replaced.
    SectionManaged,
    /// archon never touches this file.
    Unmanaged,
}

impl SyncManifest {
    /// Load a sync manifest from a file path.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading sync manifest: {e}"))?;
        let manifest: Self = serde_yaml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("parsing sync manifest: {e}"))?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest() {
        let yaml = r#"
version: "2026.03"
files:
  - path: AGENTS.md
    ownership: section-managed
    source: templates/agents-managed-section.md
  - path: specs/docs/upstream-architecture.md
    ownership: fully-managed
    source: templates/upstream-architecture.md
  - path: specs/docs/architecture.md
    ownership: unmanaged
"#;
        let manifest: SyncManifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.version, "2026.03");
        assert_eq!(manifest.files.len(), 3);
        assert_eq!(manifest.files[0].ownership, Ownership::SectionManaged);
        assert_eq!(manifest.files[1].ownership, Ownership::FullyManaged);
        assert_eq!(manifest.files[2].ownership, Ownership::Unmanaged);
        assert!(manifest.files[2].source.is_none());
    }

    #[test]
    fn round_trip() {
        let manifest = SyncManifest {
            version: "2026.03".into(),
            files: vec![SyncEntry {
                path: "AGENTS.md".into(),
                ownership: Ownership::SectionManaged,
                source: Some("templates/agents.md".into()),
            }],
        };
        let yaml = serde_yaml::to_string(&manifest).unwrap();
        let parsed: SyncManifest = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.files[0].path, "AGENTS.md");
    }
}
