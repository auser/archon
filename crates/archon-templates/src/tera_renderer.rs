use anyhow::{bail, Context, Result};

const SPEC_TEMPLATE: &str = include_str!("../templates/generate/spec.md.tera");
const PROMPT_TEMPLATE: &str = include_str!("../templates/generate/prompt.md.tera");
const ADR_TEMPLATE: &str = include_str!("../templates/generate/adr.md.tera");
const PLAN_TEMPLATE: &str = include_str!("../templates/generate/plan.md.tera");

/// Render a generate-command document through the appropriate Tera template.
///
/// `doc_type` must be one of: `"spec"`, `"prompt"`, `"adr"`, `"plan"`.
pub fn render_generate_template(
    doc_type: &str,
    context: &tera::Context,
) -> Result<String> {
    let mut tera = tera::Tera::default();

    let template_name = format!("{doc_type}.md.tera");
    let template_src = match doc_type {
        "spec" => SPEC_TEMPLATE,
        "prompt" => PROMPT_TEMPLATE,
        "adr" => ADR_TEMPLATE,
        "plan" => PLAN_TEMPLATE,
        _ => bail!("unknown doc type for template: {doc_type}"),
    };

    tera.add_raw_template(&template_name, template_src)
        .context("parsing Tera template")?;

    tera.render(&template_name, context)
        .context("rendering Tera template")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_spec_template() {
        let mut ctx = tera::Context::new();
        ctx.insert("title", "Test Spec");
        ctx.insert("date", "2026-03-07");
        ctx.insert("audience", "Engineers");
        ctx.insert("description", "A test specification.");
        ctx.insert("content", "## Details\n\nSome content here.");
        ctx.insert("constraints", "Must be fast.");

        let result = render_generate_template("spec", &ctx).unwrap();
        assert!(result.contains("# Test Spec"));
        assert!(result.contains("Specification"));
        assert!(result.contains("Must be fast."));
    }

    #[test]
    fn render_plan_without_constraints() {
        let mut ctx = tera::Context::new();
        ctx.insert("title", "Migration Plan");
        ctx.insert("date", "2026-03-07");
        ctx.insert("audience", "Team");
        ctx.insert("description", "Migrate the database.");
        ctx.insert("content", "## Phase 1\n\nDo stuff.");
        ctx.insert("constraints", "");

        let result = render_generate_template("plan", &ctx).unwrap();
        assert!(result.contains("# Migration Plan"));
        assert!(!result.contains("Constraints & Risks"));
    }
}
