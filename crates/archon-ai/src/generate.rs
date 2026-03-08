use anyhow::{Context, Result};

use crate::backend::{self, Backend};

/// Structured input gathered from the user in Phase 1 of the generate flow.
#[derive(Clone, Debug)]
pub struct StructuredInput {
    pub doc_type: String,
    pub title: String,
    pub description: String,
    pub audience: String,
    pub constraints: String,
    /// Doc-type-specific extra fields (key, value).
    pub extra_fields: Vec<(String, String)>,
}

/// The AI's response during a refinement turn.
#[derive(Debug)]
pub enum RefinementResponse {
    /// AI wants to ask clarifying questions before generating.
    Questions(Vec<String>),
    /// AI has produced the final document content.
    FinalDocument(String),
}

/// Generate a document directly without interactive refinement.
pub fn generate_direct(
    input: &StructuredInput,
    backend: Backend,
    model: &str,
) -> Result<String> {
    let prompt = build_direct_prompt(input);
    let response = backend::call(&prompt, model, backend).context("AI document generation")?;

    // Strip any DOCUMENT: prefix the AI may add
    let content = response
        .trim()
        .strip_prefix("DOCUMENT:")
        .unwrap_or(response.trim())
        .trim();

    Ok(content.to_string())
}

/// Run one turn of the interactive refinement loop.
///
/// Sends the structured input plus conversation history to the AI.
/// Returns either clarifying questions or the final document.
pub fn refine_turn(
    input: &StructuredInput,
    history: &[(String, String)],
    backend: Backend,
    model: &str,
) -> Result<RefinementResponse> {
    let prompt = build_refinement_prompt(input, history);
    let response = backend::call(&prompt, model, backend).context("AI refinement turn")?;
    parse_refinement_response(&response)
}

fn build_direct_prompt(input: &StructuredInput) -> String {
    let extras = format_extras(&input.extra_fields);
    let type_guidance = type_specific_guidance(&input.doc_type);

    format!(
        r#"You are a technical writer for the Hologram architecture ecosystem — a multi-repo Rust project governed by shared architecture standards.

Generate a complete {doc_type} document with the following parameters:

Title: {title}
Purpose: {description}
Audience: {audience}
{constraints_section}
{extras}

{type_guidance}

Output ONLY the document body content in markdown. Do NOT include the title header or metadata table — those will be added by the template system. Start directly with the substantive content (sections, paragraphs, code blocks, etc.).

For ADR documents, output three clearly labeled sections using these exact headers:
## Context
## Decision
## Consequences

For all other document types, use appropriate markdown headers (##, ###) to organize the content logically."#,
        doc_type = input.doc_type,
        title = input.title,
        description = input.description,
        audience = input.audience,
        constraints_section = if input.constraints.is_empty() {
            String::new()
        } else {
            format!("Constraints: {}", input.constraints)
        },
    )
}

fn build_refinement_prompt(input: &StructuredInput, history: &[(String, String)]) -> String {
    let extras = format_extras(&input.extra_fields);
    let type_guidance = type_specific_guidance(&input.doc_type);

    let history_section = if history.is_empty() {
        String::from("(No previous conversation yet — this is the first turn.)")
    } else {
        history
            .iter()
            .map(|(role, content)| format!("{}: {content}", role.to_uppercase()))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    format!(
        r#"You are a technical writer for the Hologram architecture ecosystem — a multi-repo Rust project governed by shared architecture standards.

The user wants to create a {doc_type} document with these parameters:

Title: {title}
Purpose: {description}
Audience: {audience}
{constraints_section}
{extras}

{type_guidance}

PREVIOUS CONVERSATION:
{history_section}

INSTRUCTIONS:
Evaluate whether you have enough information to write a high-quality {doc_type} document. Consider:
- Is the scope clear and specific enough?
- Are there ambiguities that would lead to a vague or unhelpful document?
- Are there important details missing for this type of document?

If you need more information, respond with EXACTLY this format (2-4 questions max):

QUESTIONS:
1. [your question]
2. [your question]

If you have enough information to write an excellent document, respond with EXACTLY this format:

DOCUMENT:
[full document body in markdown — sections, paragraphs, code blocks, etc.]

Do NOT include the title header or metadata table — those are handled by the template.
For ADR documents, output three clearly labeled sections: ## Context, ## Decision, ## Consequences.
Do NOT mix questions and document content in the same response."#,
        doc_type = input.doc_type,
        title = input.title,
        description = input.description,
        audience = input.audience,
        constraints_section = if input.constraints.is_empty() {
            String::new()
        } else {
            format!("Constraints: {}", input.constraints)
        },
    )
}

fn parse_refinement_response(response: &str) -> Result<RefinementResponse> {
    let trimmed = response.trim();

    if let Some(rest) = trimmed.strip_prefix("DOCUMENT:") {
        return Ok(RefinementResponse::FinalDocument(rest.trim().to_string()));
    }

    if let Some(rest) = trimmed.strip_prefix("QUESTIONS:") {
        let questions: Vec<String> = rest
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                // Strip leading numbering like "1. " or "1) "
                l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ')
                    .to_string()
            })
            .filter(|q| !q.is_empty())
            .collect();

        if !questions.is_empty() {
            return Ok(RefinementResponse::Questions(questions));
        }
    }

    // Heuristic fallback: short responses with question marks are likely questions
    if trimmed.len() < 500 && trimmed.matches('?').count() >= 2 {
        let questions: Vec<String> = trimmed
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && l.contains('?'))
            .map(|l| {
                l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == ' ')
                    .to_string()
            })
            .collect();

        if !questions.is_empty() {
            return Ok(RefinementResponse::Questions(questions));
        }
    }

    // Default: treat as final document
    Ok(RefinementResponse::FinalDocument(trimmed.to_string()))
}

fn type_specific_guidance(doc_type: &str) -> &'static str {
    match doc_type {
        "spec" => {
            "This is a SPECIFICATION document. Include:\n\
             - Clear requirements (use MUST, SHOULD, MAY language)\n\
             - Interface definitions or data structures where relevant\n\
             - Edge cases and error handling\n\
             - Examples where they aid understanding"
        }
        "prompt" => {
            "This is a PROMPT TEMPLATE for AI interactions. Include:\n\
             - System role/context setup\n\
             - Clear task instructions\n\
             - Input/output format expectations\n\
             - Example inputs and expected outputs where helpful\n\
             - Guardrails or constraints for the AI"
        }
        "adr" => {
            "This is an ARCHITECTURE DECISION RECORD. Follow this structure:\n\
             - Context: 2-4 paragraphs explaining the problem and constraints\n\
             - Decision: Clear, prescriptive decision with concrete rules\n\
             - Consequences: Positive benefits, negative trade-offs, and migration steps"
        }
        "plan" => {
            "This is an IMPLEMENTATION PLAN. Include:\n\
             - Phases or milestones with clear deliverables\n\
             - Dependencies between phases\n\
             - Risk assessment and mitigation strategies\n\
             - Success criteria for each phase"
        }
        _ => "",
    }
}

fn format_extras(extras: &[(String, String)]) -> String {
    if extras.is_empty() {
        return String::new();
    }
    extras
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("\n")
}
