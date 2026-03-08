use anyhow::Result;

use crate::backend::{self, Backend};

/// AI-merge two versions of a documentation file.
///
/// Uses the architecture version as the structural template, preserving
/// subproject-specific content where it exists.
pub fn merge_docs(
    filename: &str,
    arch_content: &str,
    local_content: &str,
    ai_backend: Backend,
    model: &str,
) -> Result<String> {
    let prompt = format!(
        "Merge these two versions of a project documentation file: {filename}\n\n\
         ARCH VERSION:\n{arch_content}\n\n\
         SUBPROJECT VERSION:\n{local_content}\n\n\
         Rules:\n\
         1. Use ARCH VERSION as the structural template.\n\
         2. Include all sections from ARCH VERSION, including newly added ones.\n\
         3. Where SUBPROJECT VERSION has real content (not <!-- TODO -->), preserve it.\n\
         4. Blend where both versions have content: keep subproject specifics, add new arch guidance.\n\
         5. Remove <!-- TODO --> placeholders where real content already exists.\n\
         6. Output ONLY the merged markdown -- no explanation, no code fences.",
    );

    backend::call(&prompt, model, ai_backend)
}
