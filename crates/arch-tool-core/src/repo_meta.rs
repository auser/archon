use serde::{Deserialize, Serialize};

use crate::standards_version::StandardsVersion;

/// Root of hologram.repo.yaml in each downstream repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMeta {
    pub name: String,
    pub role: RepoRole,
    pub repo_type: RepoType,
    pub standards_version: StandardsVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture_version: Option<String>,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub contracts: ContractDecl,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crate_classes: Vec<CrateClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exceptions: Vec<ExceptionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RepoRole {
    Core,
    Extension,
    Tool,
    Service,
    Library,
}

impl std::fmt::Display for RepoRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "core"),
            Self::Extension => write!(f, "extension"),
            Self::Tool => write!(f, "tool"),
            Self::Service => write!(f, "service"),
            Self::Library => write!(f, "library"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RepoType {
    RustWorkspace,
    RustBinary,
    Mixed,
}

impl std::fmt::Display for RepoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RustWorkspace => write!(f, "rust-workspace"),
            Self::RustBinary => write!(f, "rust-binary"),
            Self::Mixed => write!(f, "mixed"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractDecl {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateClass {
    pub name: String,
    pub class: CrateClassKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CrateClassKind {
    PublicApi,
    Internal,
    Binary,
    TestSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionRef {
    pub id: String,
    pub rule: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_yaml() {
        let yaml = r#"
name: hologram-sandbox
role: extension
repo_type: rust-workspace
standards_version: "2026.03"
architecture_version: "1.0"
owners:
  - "@core-team"
contracts:
  implements:
    - sandbox-runtime
  depends_on:
    - hologram-execution-plan
crate_classes:
  - name: hologram-sandbox
    class: public-api
  - name: hologram-sandbox-wasm
    class: internal
exceptions:
  - id: EXC-2026-001
    rule: STR-004
    expires: "2026-06-01"
"#;
        let meta: RepoMeta = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(meta.name, "hologram-sandbox");
        assert_eq!(meta.role, RepoRole::Extension);
        assert_eq!(meta.repo_type, RepoType::RustWorkspace);
        assert_eq!(meta.owners, vec!["@core-team"]);
        assert_eq!(meta.contracts.implements, vec!["sandbox-runtime"]);
        assert_eq!(meta.crate_classes.len(), 2);
        assert_eq!(meta.exceptions.len(), 1);
        assert_eq!(meta.exceptions[0].id, "EXC-2026-001");

        // Round-trip: serialize back and deserialize again
        let serialized = serde_yaml::to_string(&meta).unwrap();
        let meta2: RepoMeta = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(meta2.name, meta.name);
        assert_eq!(meta2.role, meta.role);
    }
}
