use anyhow::{Context, Result};
use colored::Colorize;

use archon_adr::numbering;
use archon_ai::backend;
use archon_ai::decide::{self, DecisionContext};
use archon_core::paths;

use crate::app::DecideArgs;
use crate::output;

pub fn run(args: DecideArgs) -> Result<()> {
    let arch_root = paths::resolve_arch_root(args.arch_root.as_deref())?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "architecture repo not found. Use --arch-root or set ARCHON_ROOT.\n  \
                 The decide command needs the architecture repo to read existing ADRs and policies."
            )
        })?;

    let ai_backend = backend::detect().context(
        "AI backend required for decide command.\n  \
         Set ANTHROPIC_API_KEY or install the claude CLI.",
    )?;

    let title = &args.title;
    let question = args.question.as_deref().unwrap_or(title);

    output::print_header(&format!("archon decide: {title}"));
    println!();

    eprintln!(
        "  {} Loading context from {}...",
        "→".blue(),
        arch_root.display()
    );
    let context = DecisionContext::load(&arch_root);

    let adr_dir = arch_root.join("specs/adrs");
    let next_num = numbering::next_number(&adr_dir)?;

    eprintln!(
        "  {} Drafting ADR-{:04} with AI...",
        "→".blue(),
        next_num
    );
    let model = "claude-sonnet-4-5-20250514";
    let draft = decide::draft_decision(question, &context, ai_backend, model, title, next_num)?;

    if args.dry_run {
        println!("{}", "--- ADR Draft ---".dimmed());
        println!();
        println!("{}", draft.adr_content);
        println!();

        if let Some(ref snippet) = draft.policy_snippet {
            println!("{}", "--- Suggested Policy Rule ---".dimmed());
            println!();
            println!("{snippet}");
            println!();
        }

        if let Some(ref suggestion) = draft.template_suggestion {
            println!("{}", "--- Template Update ---".dimmed());
            println!();
            println!("{suggestion}");
            println!();
        }

        println!("  (dry run -- no files written)");
    } else {
        // Write the ADR file
        let slug = numbering::slugify(title);
        let filename = format!("{}-{slug}.md", numbering::format_number(next_num));
        let adr_path = adr_dir.join(&filename);

        std::fs::create_dir_all(&adr_dir)
            .with_context(|| format!("creating {}", adr_dir.display()))?;
        std::fs::write(&adr_path, &draft.adr_content)
            .with_context(|| format!("writing {}", adr_path.display()))?;

        output::print_created(&format!("specs/adrs/{filename}"));

        // Write policy snippet as a sidecar suggestion file
        if let Some(ref snippet) = draft.policy_snippet {
            let suggestion_dir = arch_root.join(".archon/suggestions");
            std::fs::create_dir_all(&suggestion_dir)?;
            let suggestion_path = suggestion_dir.join(format!("{}-{slug}.yaml", numbering::format_number(next_num)));
            std::fs::write(&suggestion_path, snippet)
                .with_context(|| format!("writing {}", suggestion_path.display()))?;

            eprintln!(
                "  {} policy suggestion saved to .archon/suggestions/{}-{slug}.yaml",
                "✓".green(),
                numbering::format_number(next_num)
            );
            eprintln!(
                "  {}",
                "Review and append to the appropriate policies/ file.".dimmed()
            );
        }

        if let Some(ref suggestion) = draft.template_suggestion {
            eprintln!();
            eprintln!(
                "  {} Template update suggested:",
                "→".blue()
            );
            eprintln!("  {suggestion}");
            eprintln!(
                "  {}",
                "Add this to templates/agents-managed-section.md if appropriate.".dimmed()
            );
        }
    }

    println!();
    Ok(())
}
