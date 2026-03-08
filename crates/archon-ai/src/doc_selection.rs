use colored::Colorize;

use crate::backend::{self, Backend};
use crate::context::ProjectContext;

/// Describes one optional documentation template that AI can recommend.
pub struct OptionalDoc {
    pub rel_path: &'static str,
    pub description: &'static str,
}

/// The set of optional docs available for selection during init.
pub const OPTIONAL_DOCS: &[OptionalDoc] = &[
    OptionalDoc {
        rel_path: "specs/docs/runtime.md",
        description: "Runtime architecture and execution model",
    },
    OptionalDoc {
        rel_path: "specs/docs/performance.md",
        description: "Performance requirements and benchmarking strategy",
    },
    OptionalDoc {
        rel_path: "specs/docs/security.md",
        description: "Security model and threat analysis",
    },
    OptionalDoc {
        rel_path: "specs/docs/import-pipeline.md",
        description: "Data import/conversion pipeline design",
    },
    OptionalDoc {
        rel_path: "specs/docs/data-model.md",
        description: "Core data model and schema design",
    },
    OptionalDoc {
        rel_path: "specs/docs/cli.md",
        description: "CLI command design and UX",
    },
    OptionalDoc {
        rel_path: "specs/docs/api.md",
        description: "API design and endpoint documentation",
    },
    OptionalDoc {
        rel_path: "specs/docs/deployment.md",
        description: "Deployment architecture and infrastructure",
    },
    OptionalDoc {
        rel_path: "specs/docs/operations.md",
        description: "Operational runbooks and monitoring",
    },
    OptionalDoc {
        rel_path: "specs/docs/validation.md",
        description: "Validation and correctness verification strategy",
    },
    OptionalDoc {
        rel_path: "specs/docs/backend-matrix.md",
        description: "Backend/target matrix and compatibility",
    },
];

/// Ask AI which optional documentation files are relevant to this project.
///
/// Returns a vec of `rel_path` strings chosen from `OPTIONAL_DOCS`.
/// On any failure, logs a warning and returns `[]` (caller uses base files only).
pub fn select_relevant_docs(
    context: &ProjectContext,
    ai_backend: Backend,
    model: &str,
) -> Vec<String> {
    let docs_list = OPTIONAL_DOCS
        .iter()
        .map(|d| format!("- `{}` -- {}", d.rel_path, d.description))
        .collect::<Vec<_>>()
        .join("\n");

    let existing = if context.existing_files.is_empty() {
        "(none)".to_owned()
    } else {
        context.existing_files.join(", ")
    };

    let prompt = format!(
        "You are helping initialize a Rust project repository with architecture documentation.\n\n\
         Project name: {name}\n\n\
         Cargo.toml:\n```toml\n{cargo}\n```\n\n\
         Existing notable files: {existing}\n\n\
         The following optional documentation templates are available:\n\
         {docs_list}\n\n\
         Based on the project context above, which of the optional documentation files\n\
         are genuinely relevant to this project?\n\n\
         Respond with ONLY a JSON array of rel_path strings, e.g.:\n\
         [\"specs/docs/runtime.md\", \"specs/docs/performance.md\"]\n\n\
         Include only the optional files that are clearly relevant.\n\
         If none apply, respond with: []\n\
         Do not include any explanation -- only the JSON array.",
        name = context.repo_name,
        cargo = context.cargo_toml,
    );

    let raw = match backend::call(&prompt, model, ai_backend) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "  {} AI doc selection failed: {e} -- using base files only",
                "warning:".yellow(),
            );
            return vec![];
        }
    };

    extract_json_array(&raw)
}

fn extract_json_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let json_str = if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    match serde_json::from_str::<Vec<String>>(json_str) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!(
                "  {} could not parse AI response: {e} -- using base files only",
                "warning:".yellow(),
            );
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_clean_array() {
        let input = r#"["specs/docs/runtime.md", "specs/docs/cli.md"]"#;
        let result = extract_json_array(input);
        assert_eq!(result, vec!["specs/docs/runtime.md", "specs/docs/cli.md"]);
    }

    #[test]
    fn extract_array_with_surrounding_text() {
        let input = "Here are the relevant docs:\n[\"specs/docs/api.md\"]\nThat's it.";
        let result = extract_json_array(input);
        assert_eq!(result, vec!["specs/docs/api.md"]);
    }

    #[test]
    fn extract_empty_array() {
        let result = extract_json_array("[]");
        assert!(result.is_empty());
    }

    #[test]
    fn extract_garbage_returns_empty() {
        let result = extract_json_array("not json at all");
        assert!(result.is_empty());
    }
}
