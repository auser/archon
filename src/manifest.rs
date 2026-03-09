use serde::{Deserialize, Serialize};
use std::path::Path;

use anyhow::{Context, Result};

const MANIFEST_FILE: &str = "archon.yaml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Core,
    Extension,
    Tool,
    Service,
    Library,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Core => write!(f, "core"),
            Role::Extension => write!(f, "extension"),
            Role::Tool => write!(f, "tool"),
            Role::Service => write!(f, "service"),
            Role::Library => write!(f, "library"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateEntry {
    pub name: String,
    #[serde(default = "default_true")]
    pub public: bool,
}

fn default_true() -> bool {
    true
}

/// A rule that must pass for the repo to be considered conformant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Short identifier for the rule (e.g. "tests-pass", "no-clippy-warnings").
    pub id: String,
    /// Shell command to run. Exit code 0 = pass, non-zero = fail.
    pub run: String,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crates: Option<Vec<CrateEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    /// Path to the archon registry (relative to repo root or absolute).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// Rules that must pass for this repo (run via shell commands).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
}

impl Manifest {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(MANIFEST_FILE);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let manifest: Manifest = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(manifest)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let path = dir.join(MANIFEST_FILE);
        let yaml = serde_yaml::to_string(self).context("serializing manifest")?;
        std::fs::write(&path, yaml).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn manifest_path(dir: &Path) -> std::path::PathBuf {
        dir.join(MANIFEST_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_manifest() {
        let manifest = Manifest {
            name: "hologram-sandbox".into(),
            description: "Sandboxed execution environment".into(),
            role: Role::Extension,
            depends_on: vec!["hologram".into(), "hologram-ai".into()],
            provides: vec!["sandbox-runtime".into()],
            crates: Some(vec![CrateEntry {
                name: "hologram-sandbox-core".into(),
                public: true,
            }]),
            auto_update: None,
            registry: None,
            rules: vec![],
        };

        let yaml = serde_yaml::to_string(&manifest).unwrap();
        let parsed: Manifest = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.name, "hologram-sandbox");
        assert_eq!(parsed.role, Role::Extension);
        assert_eq!(parsed.depends_on.len(), 2);
        assert_eq!(parsed.provides, vec!["sandbox-runtime"]);
    }

    #[test]
    fn minimal_manifest() {
        let yaml = r#"
name: my-tool
description: "A simple tool"
role: tool
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.name, "my-tool");
        assert!(manifest.depends_on.is_empty());
        assert!(manifest.provides.is_empty());
        assert!(manifest.crates.is_none());
    }
}
