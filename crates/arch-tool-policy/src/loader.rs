use std::path::Path;

use anyhow::{Context, Result};

use crate::builtin::builtin_policies;
use crate::model::PolicyFile;

/// Load policy files from the architecture repo's policies/ directory.
/// Falls back to built-in policies if no arch repo is available.
pub fn load_policies(arch_root: Option<&Path>) -> Result<Vec<PolicyFile>> {
    if let Some(root) = arch_root {
        let policies_dir = root.join("policies");
        if policies_dir.is_dir() {
            let mut policies = Vec::new();
            for entry in std::fs::read_dir(&policies_dir)
                .with_context(|| format!("reading {}", policies_dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                    let contents = std::fs::read_to_string(&path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    let policy: PolicyFile = serde_yaml::from_str(&contents)
                        .with_context(|| format!("parsing {}", path.display()))?;
                    policies.push(policy);
                }
            }
            if !policies.is_empty() {
                return Ok(policies);
            }
        }
    }

    // Fall back to built-in policies
    Ok(vec![builtin_policies()])
}
