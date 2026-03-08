use serde::{Deserialize, Serialize};

/// A policy file loaded from hologram-architecture/policies/*.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFile {
    pub version: String,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub category: RuleCategory,
    pub severity: Severity,
    pub description: String,
    pub check: CheckSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleCategory {
    Structural,
    Policy,
    Architectural,
}

impl std::fmt::Display for RuleCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structural => write!(f, "structural"),
            Self::Policy => write!(f, "policy"),
            Self::Architectural => write!(f, "architectural"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckSpec {
    FileExists { path: String },
    DirExists { path: String },
    MetadataField { field: String, condition: String },
    StandardsVersion { minimum: String },
    CrateTaxonomy { require_classes: bool },
    DependencyDirection { disallowed: Vec<DependencyPair> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyPair {
    pub from: String,
    pub to: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_policy_yaml() {
        let yaml = r#"
version: "2026.03"
rules:
  - id: STR-001
    category: structural
    severity: error
    description: "hologram.repo.yaml must exist"
    check:
      type: file_exists
      path: "hologram.repo.yaml"
  - id: POL-001
    category: policy
    severity: error
    description: "standards_version must be present"
    check:
      type: metadata_field
      field: standards_version
      condition: present
"#;
        let policy: PolicyFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(policy.version, "2026.03");
        assert_eq!(policy.rules.len(), 2);
        assert_eq!(policy.rules[0].id, "STR-001");
        assert!(matches!(policy.rules[0].check, CheckSpec::FileExists { .. }));
    }
}
