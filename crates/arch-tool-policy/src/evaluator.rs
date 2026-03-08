use std::path::Path;

use arch_tool_core::repo_meta::{ExceptionRef, RepoMeta};
use arch_tool_core::standards_version::StandardsVersion;

use crate::model::{CheckSpec, PolicyRule, Severity};

/// Result of evaluating a single policy rule.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub rule_id: String,
    pub category: String,
    pub severity: Severity,
    pub passed: bool,
    pub message: String,
    pub excepted: bool,
}

/// Evaluates policy rules against a repository's state.
pub struct PolicyEvaluator<'a> {
    rules: &'a [PolicyRule],
    exceptions: &'a [ExceptionRef],
}

impl<'a> PolicyEvaluator<'a> {
    pub fn new(rules: &'a [PolicyRule], exceptions: &'a [ExceptionRef]) -> Self {
        Self { rules, exceptions }
    }

    /// Evaluate all rules against the given repo root and metadata.
    pub fn evaluate(&self, repo_root: &Path, meta: Option<&RepoMeta>) -> Vec<CheckResult> {
        self.rules
            .iter()
            .map(|rule| {
                let excepted = self.exceptions.iter().any(|e| e.rule == rule.id);
                let (passed, message) = self.check_rule(rule, repo_root, meta);

                CheckResult {
                    rule_id: rule.id.clone(),
                    category: rule.category.to_string(),
                    severity: rule.severity,
                    passed: passed || excepted,
                    message: if excepted && !passed {
                        format!("{} (excepted)", message)
                    } else {
                        message
                    },
                    excepted,
                }
            })
            .collect()
    }

    fn check_rule(
        &self,
        rule: &PolicyRule,
        repo_root: &Path,
        meta: Option<&RepoMeta>,
    ) -> (bool, String) {
        match &rule.check {
            CheckSpec::FileExists { path } => {
                let full = repo_root.join(path);
                if full.is_file() {
                    (true, format!("{path} exists"))
                } else {
                    (false, format!("{path} not found"))
                }
            }
            CheckSpec::DirExists { path } => {
                let full = repo_root.join(path);
                if full.is_dir() {
                    (true, format!("{path}/ exists"))
                } else {
                    (false, format!("{path}/ not found"))
                }
            }
            CheckSpec::MetadataField { field, condition } => {
                self.check_metadata_field(field, condition, meta)
            }
            CheckSpec::StandardsVersion { minimum } => {
                self.check_standards_version(minimum, meta)
            }
            CheckSpec::CrateTaxonomy { .. } | CheckSpec::DependencyDirection { .. } => {
                // Phase 3: architectural checks
                (true, "check not yet implemented".to_string())
            }
        }
    }

    fn check_metadata_field(
        &self,
        field: &str,
        condition: &str,
        meta: Option<&RepoMeta>,
    ) -> (bool, String) {
        let Some(meta) = meta else {
            return (false, format!("{field}: no metadata loaded"));
        };

        match (field, condition) {
            ("standards_version", "present") => {
                let v = &meta.standards_version;
                match StandardsVersion::parse(&v.0) {
                    Ok(_) => (true, format!("standards_version: {v}")),
                    Err(e) => (false, format!("standards_version invalid: {e}")),
                }
            }
            ("owners", "non_empty") => {
                if meta.owners.is_empty() {
                    (false, "owners list is empty".to_string())
                } else {
                    (true, format!("owners: {} entries", meta.owners.len()))
                }
            }
            _ => (true, format!("{field}: unknown check, skipped")),
        }
    }

    fn check_standards_version(
        &self,
        minimum: &str,
        meta: Option<&RepoMeta>,
    ) -> (bool, String) {
        let Some(meta) = meta else {
            return (false, "no metadata loaded".to_string());
        };

        let Ok(min) = StandardsVersion::parse(minimum) else {
            return (false, format!("invalid minimum version: {minimum}"));
        };

        if meta.standards_version >= min {
            (true, format!("standards_version {} >= {minimum}", meta.standards_version))
        } else {
            (false, format!("standards_version {} < {minimum}", meta.standards_version))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_policies;
    use arch_tool_core::repo_meta::{RepoRole, RepoType};
    use tempfile::tempdir;

    fn make_meta() -> RepoMeta {
        RepoMeta {
            name: "test".to_string(),
            role: RepoRole::Tool,
            repo_type: RepoType::RustBinary,
            standards_version: StandardsVersion("2026.03".to_string()),
            architecture_version: None,
            owners: vec!["@team".to_string()],
            contracts: Default::default(),
            crate_classes: vec![],
            exceptions: vec![],
        }
    }

    #[test]
    fn builtin_rules_pass_compliant_repo() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("hologram.repo.yaml"), "").unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("specs/docs")).unwrap();
        std::fs::write(dir.path().join("specs/docs/architecture.md"), "").unwrap();

        let policies = builtin_policies();
        let meta = make_meta();
        let evaluator = PolicyEvaluator::new(&policies.rules, &meta.exceptions);
        let results = evaluator.evaluate(dir.path(), Some(&meta));

        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    }

    #[test]
    fn missing_files_cause_failures() {
        let dir = tempdir().unwrap();
        let policies = builtin_policies();
        let meta = make_meta();
        let evaluator = PolicyEvaluator::new(&policies.rules, &meta.exceptions);
        let results = evaluator.evaluate(dir.path(), Some(&meta));

        let errors: Vec<_> = results
            .iter()
            .filter(|r| !r.passed && r.severity == Severity::Error)
            .collect();
        assert!(errors.len() >= 2, "expected at least 2 errors for missing files");
    }

    #[test]
    fn exception_suppresses_failure() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("hologram.repo.yaml"), "").unwrap();
        // AGENTS.md is missing, but excepted
        let exceptions = vec![ExceptionRef {
            id: "EXC-2026-001".to_string(),
            rule: "STR-002".to_string(),
            expires: None,
        }];

        let policies = builtin_policies();
        let meta = make_meta();
        let evaluator = PolicyEvaluator::new(&policies.rules, &exceptions);
        let results = evaluator.evaluate(dir.path(), Some(&meta));

        let str002 = results.iter().find(|r| r.rule_id == "STR-002").unwrap();
        assert!(str002.passed, "STR-002 should pass due to exception");
        assert!(str002.excepted);
    }
}
