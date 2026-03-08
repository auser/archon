use std::path::Path;

use anyhow::{Context, Result};

use crate::backend::{self, Backend};

/// Context gathered from the architecture repo to inform a decision.
pub struct DecisionContext {
    /// Existing ADR summaries (titles + statuses).
    pub existing_adrs: String,
    /// Current policy rules.
    pub policies: String,
    /// Ecosystem registry.
    pub ecosystem: String,
    /// Standards documentation.
    pub standards: String,
}

impl DecisionContext {
    /// Load decision context from an architecture repo root.
    pub fn load(arch_root: &Path) -> Self {
        Self {
            existing_adrs: glob_read_summaries(&arch_root.join("specs/adrs"), 200),
            policies: glob_read_yaml(&arch_root.join("policies"), 200),
            ecosystem: read_file_truncated(&arch_root.join("ecosystem/repos.yaml"), 100),
            standards: read_file_truncated(&arch_root.join("standards/current.md"), 100),
        }
    }
}

/// Result of an AI-driven architecture decision.
pub struct DecisionDraft {
    /// The full ADR markdown content.
    pub adr_content: String,
    /// Optional policy rule YAML snippet to add.
    pub policy_snippet: Option<String>,
    /// Optional template update suggestion.
    pub template_suggestion: Option<String>,
}

/// Use AI to draft an architecture decision based on a question and existing context.
pub fn draft_decision(
    question: &str,
    context: &DecisionContext,
    backend: Backend,
    model: &str,
    title: &str,
    adr_number: u32,
) -> Result<DecisionDraft> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let prompt = format!(
        r#"You are an architecture decision advisor for the Hologram ecosystem — a multi-repo Rust project governed by shared architecture standards.

Your task: Draft an Architecture Decision Record (ADR) that addresses the following question:

QUESTION: {question}

Use the context below to ensure your decision is consistent with existing decisions and the ecosystem's patterns.

EXISTING ADRs:
{existing_adrs}

CURRENT POLICIES:
{policies}

ECOSYSTEM REGISTRY:
{ecosystem}

CURRENT STANDARDS:
{standards}

Write the ADR in exactly this format:

# ADR-{adr_number:04}: {title}

## Status

Proposed

## Date

{today}

## Context

[2-4 paragraphs explaining the problem, why it matters, and what constraints exist. Reference relevant existing ADRs by number.]

## Decision

[Clear, specific decision. Include concrete rules, naming conventions, or structural requirements. Be prescriptive enough that a machine-readable policy rule can be derived.]

## Consequences

### Positive
[Bulleted list of benefits]

### Negative
[Bulleted list of costs or trade-offs]

### Migration
[What existing repos need to do to comply, if anything]

---

After the ADR, on a new line, output a YAML block fenced with ```yaml that contains a suggested policy rule for this decision. Use this format:

```yaml
- id: [CATEGORY]-[NNN]
  category: [structural|policy|architectural]
  severity: [error|warning]
  description: "[one-line description]"
  check:
    type: [file_exists|dir_exists|metadata_field|dependency_direction|crate_taxonomy]
    [additional fields as appropriate]
```

If the decision doesn't map to a concrete policy rule (e.g., it's a process or convention decision), output `# No policy rule needed` instead of the YAML block.

After the policy rule, if the decision should be communicated to AI agents via AGENTS.md managed sections, output a line starting with `TEMPLATE_UPDATE:` followed by the text to add to the managed section template. If no template update is needed, omit this line."#,
        existing_adrs = context.existing_adrs,
        policies = context.policies,
        ecosystem = context.ecosystem,
        standards = context.standards,
    );

    let response = backend::call(&prompt, model, backend).context("AI decision drafting")?;

    parse_decision_response(&response)
}

fn parse_decision_response(response: &str) -> Result<DecisionDraft> {
    // Split the response to extract: ADR content, policy snippet, template suggestion

    // Find the policy YAML block
    let (adr_part, rest) = if let Some(idx) = response.find("```yaml") {
        (&response[..idx], &response[idx..])
    } else {
        (response, "")
    };

    // Extract policy snippet
    let policy_snippet = if rest.contains("```yaml") {
        let start = rest.find("```yaml").unwrap() + 7;
        let end = rest[start..].find("```").map(|i| start + i).unwrap_or(rest.len());
        let snippet = rest[start..end].trim().to_string();
        if snippet.contains("No policy rule needed") {
            None
        } else {
            Some(snippet)
        }
    } else {
        None
    };

    // Extract template suggestion
    let template_suggestion = response
        .lines()
        .find(|line| line.starts_with("TEMPLATE_UPDATE:"))
        .map(|line| line.trim_start_matches("TEMPLATE_UPDATE:").trim().to_string());

    let adr_content = adr_part.trim().to_string();

    Ok(DecisionDraft {
        adr_content,
        policy_snippet,
        template_suggestion,
    })
}

fn read_file_truncated(path: &Path, max_lines: usize) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|_| "(not found)".into())
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn glob_read_summaries(dir: &Path, max_lines: usize) -> String {
    if !dir.is_dir() {
        return "(no ADRs found)".into();
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    entries
        .iter()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .flat_map(|s| s.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn glob_read_yaml(dir: &Path, max_lines: usize) -> String {
    if !dir.is_dir() {
        return "(no policies found)".into();
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml" || ext == "yml"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    entries
        .iter()
        .filter_map(|e| {
            let content = std::fs::read_to_string(e.path()).ok()?;
            let name = e.file_name().to_string_lossy().to_string();
            Some(format!("# {name}\n{content}"))
        })
        .flat_map(|s| s.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}
