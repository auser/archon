use anyhow::Result;

use crate::backend::{self, Backend};
use crate::context::FillContext;

/// Fill in `<!-- TODO -->` placeholders in a single template file using AI.
///
/// Pass a pre-loaded `FillContext` for efficiency across multiple files.
pub fn fill_todos(
    rel_path: &str,
    content: &str,
    project_name: &str,
    ctx: &FillContext,
    ai_backend: Backend,
    model: &str,
) -> Result<String> {
    let prompt = format!(
        "Fill in the <!-- TODO --> placeholders in this project documentation template.\n\n\
         Project name: {project_name}\n\
         File: {rel_path}\n\
         Ecosystem: Hologram (Rust, AI-agnostic execution substrate)\n\n\
         Existing ADRs for context:\n<adrs>\n{adrs}\n</adrs>\n\n\
         Example project (hologram-ai) for style reference:\n<example>\n{example}\n</example>\n\n\
         Template to fill in:\n<template>\n{content}\n</template>\n\n\
         Rules:\n\
         - Replace every <!-- TODO --> and <!-- TODO: ... --> with specific, relevant content.\n\
         - Keep all headings and structure intact.\n\
         - Output ONLY the filled-in markdown -- no explanation, no code fences.",
        adrs = ctx.adrs_context,
        example = ctx.example_project,
    );

    backend::call(&prompt, model, ai_backend)
}

/// Check if content contains any TODO placeholders that AI could fill.
pub fn has_todos(content: &str) -> bool {
    content.contains("<!-- TODO")
}
