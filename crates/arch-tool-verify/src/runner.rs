use std::path::Path;

use anyhow::Result;
use arch_tool_core::paths;
use arch_tool_policy::evaluator::PolicyEvaluator;
use arch_tool_policy::loader::load_policies;

use crate::report::ConformanceReport;

/// Run all conformance checks on a repository.
pub fn run_verify(repo_root: &Path, arch_root: Option<&Path>) -> Result<ConformanceReport> {
    // Load repo metadata (may not exist yet)
    let meta_result = paths::find_repo_meta(repo_root)?;
    let (repo_name, meta, exceptions) = match &meta_result {
        Some((_, meta)) => (
            meta.name.clone(),
            Some(meta),
            meta.exceptions.as_slice(),
        ),
        None => {
            let name = repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            (name, None, [].as_slice())
        }
    };

    // Load policies
    let policy_files = load_policies(arch_root)?;
    let all_rules: Vec<_> = policy_files.iter().flat_map(|p| &p.rules).collect();

    // Build a flat rules vec for the evaluator
    let rules_owned: Vec<_> = all_rules.into_iter().cloned().collect();
    let evaluator = PolicyEvaluator::new(&rules_owned, exceptions);
    let results = evaluator.evaluate(repo_root, meta);

    Ok(ConformanceReport {
        results,
        repo_name,
        standards_version: meta.map(|m| m.standards_version.clone()),
    })
}
