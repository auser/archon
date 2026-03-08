use crate::model::{CheckSpec, PolicyFile, PolicyRule, RuleCategory, Severity};

/// Returns hardcoded default policy rules that work without an architecture repo.
pub fn builtin_policies() -> PolicyFile {
    PolicyFile {
        version: "2026.03".to_string(),
        rules: vec![
            PolicyRule {
                id: "STR-001".to_string(),
                category: RuleCategory::Structural,
                severity: Severity::Error,
                description: "hologram.repo.yaml must exist".to_string(),
                check: CheckSpec::FileExists {
                    path: "hologram.repo.yaml".to_string(),
                },
            },
            PolicyRule {
                id: "STR-002".to_string(),
                category: RuleCategory::Structural,
                severity: Severity::Error,
                description: "AGENTS.md must exist".to_string(),
                check: CheckSpec::FileExists {
                    path: "AGENTS.md".to_string(),
                },
            },
            PolicyRule {
                id: "STR-003".to_string(),
                category: RuleCategory::Structural,
                severity: Severity::Warning,
                description: "specs/docs/ directory should exist".to_string(),
                check: CheckSpec::DirExists {
                    path: "specs/docs".to_string(),
                },
            },
            PolicyRule {
                id: "STR-004".to_string(),
                category: RuleCategory::Structural,
                severity: Severity::Warning,
                description: "specs/docs/architecture.md should exist".to_string(),
                check: CheckSpec::FileExists {
                    path: "specs/docs/architecture.md".to_string(),
                },
            },
            PolicyRule {
                id: "POL-001".to_string(),
                category: RuleCategory::Policy,
                severity: Severity::Error,
                description: "standards_version must be present and valid".to_string(),
                check: CheckSpec::MetadataField {
                    field: "standards_version".to_string(),
                    condition: "present".to_string(),
                },
            },
            PolicyRule {
                id: "POL-002".to_string(),
                category: RuleCategory::Policy,
                severity: Severity::Warning,
                description: "owners must not be empty".to_string(),
                check: CheckSpec::MetadataField {
                    field: "owners".to_string(),
                    condition: "non_empty".to_string(),
                },
            },
        ],
    }
}
