use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use archon_adr::numbering;
use archon_ai::backend;
use archon_ai::generate::{self, RefinementResponse, StructuredInput};
use archon_core::paths;
use archon_templates::tera_renderer;

use crate::app::{DocType, GenerateArgs};
use crate::output;

const MAX_REFINEMENT_ROUNDS: usize = 5;
const AI_MODEL: &str = "claude-sonnet-4-5-20250514";

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("  {spinner:.magenta} {msg}")
        .unwrap()
        .tick_chars("\u{2800}\u{2801}\u{2803}\u{2807}\u{280f}\u{281f}\u{283f}\u{287f}\u{28ff}\u{28fe}\u{28fc}\u{28f8}\u{28f0}\u{28e0}\u{28c0}\u{2880} ")
}

pub fn run(args: GenerateArgs) -> Result<()> {
    print_banner();

    // ── Phase 1: Detect AI backend ──────────────────────────────────
    let ai_backend = backend::detect().context(
        "AI backend required for generate command.\n  \
         Set ANTHROPIC_API_KEY or run: archon auth login",
    )?;

    eprintln!(
        "  {} AI backend detected",
        "\u{2713}".green().bold()
    );
    println!();

    // ── Phase 2: Structured questions ───────────────────────────────
    eprintln!(
        "  {} {}",
        "[1/3]".blue().bold(),
        "Gathering requirements".bold()
    );
    println!();

    let doc_type = match args.doc_type {
        Some(t) => t,
        None => prompt_doc_type()?,
    };

    let title = match args.title {
        Some(t) => t,
        None => {
            inquire::Text::new(&format!(
                "  {} Title:",
                "\u{25b8}".cyan()
            ))
            .with_help_message("A concise name for your document")
            .prompt()?
        }
    };

    let description = match args.description {
        Some(d) => d,
        None => {
            inquire::Text::new(&format!(
                "  {} Purpose / description:",
                "\u{25b8}".cyan()
            ))
            .with_help_message("What problem does this document address?")
            .prompt()?
        }
    };

    let audience = inquire::Text::new(&format!(
        "  {} Target audience:",
        "\u{25b8}".cyan()
    ))
    .with_default(default_audience(&doc_type))
    .with_help_message("Who will read this document?")
    .prompt()?;

    let constraints = inquire::Text::new(&format!(
        "  {} Constraints or scope:",
        "\u{25b8}".cyan()
    ))
    .with_default("")
    .with_help_message("Optional — press Enter to skip")
    .prompt()?;

    // Doc-type-specific extras
    let extra_fields = collect_extra_fields(&doc_type)?;

    let input = StructuredInput {
        doc_type: doc_type.as_str().to_string(),
        title: title.clone(),
        description: description.clone(),
        audience: audience.clone(),
        constraints: constraints.clone(),
        extra_fields,
    };

    // ── Summary card ────────────────────────────────────────────────
    println!();
    print_summary_card(&input);
    println!();

    // ── Phase 3: AI generation ──────────────────────────────────────
    eprintln!(
        "  {} {}",
        "[2/3]".blue().bold(),
        "AI generation".bold()
    );
    println!();

    let content = if args.no_refine {
        let pb = spinner("Generating document...");
        let result = generate::generate_direct(&input, ai_backend, AI_MODEL)?;
        pb.finish_and_clear();
        eprintln!(
            "  {} Document generated",
            "\u{2713}".green().bold()
        );
        result
    } else {
        run_refinement_loop(&input, ai_backend)?
    };

    // ── Phase 4: Render and output ──────────────────────────────────
    println!();
    eprintln!(
        "  {} {}",
        "[3/3]".blue().bold(),
        "Finalizing".bold()
    );

    let rendered = render_document(&doc_type, &input, &content, args.arch_root.as_deref())?;

    if args.dry_run {
        println!();
        eprintln!("{}", "\u{2500}\u{2500}\u{2500} Preview \u{2500}\u{2500}\u{2500}".dimmed());
        println!("{rendered}");
        eprintln!("{}", "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".dimmed());
        println!();
        eprintln!("  {}", "(dry run \u{2014} no files written)".dimmed());
    } else {
        let output_path = resolve_output_path(&doc_type, &title, args.arch_root.as_deref())?;
        std::fs::create_dir_all(output_path.parent().unwrap())
            .with_context(|| format!("creating directory {}", output_path.parent().unwrap().display()))?;
        std::fs::write(&output_path, &rendered)
            .with_context(|| format!("writing {}", output_path.display()))?;

        println!();
        output::print_created(&output_path.display().to_string());

        // Show preview snippet
        let preview_lines: Vec<&str> = rendered.lines().take(8).collect();
        println!();
        for line in &preview_lines {
            eprintln!("  {}", line.dimmed());
        }
        if rendered.lines().count() > 8 {
            eprintln!("  {}", "...".dimmed());
        }

        // Stats
        let word_count = rendered.split_whitespace().count();
        let section_count = rendered.lines().filter(|l| l.starts_with("## ")).count();
        println!();
        eprintln!(
            "  {} {} words, {} sections",
            "\u{2139}".blue(),
            word_count,
            section_count,
        );
    }

    println!();
    Ok(())
}

// ── Interactive prompts ─────────────────────────────────────────────

fn prompt_doc_type() -> Result<DocType> {
    let choices = vec![
        "spec     \u{2500} Technical specification with requirements",
        "prompt   \u{2500} Prompt template for AI interactions",
        "adr      \u{2500} Architecture Decision Record",
        "plan     \u{2500} Implementation plan with phases and milestones",
    ];

    let selected = inquire::Select::new(
        &format!("  {} Document type:", "\u{25b8}".cyan()),
        choices,
    )
    .prompt()?;

    match selected.split_whitespace().next().unwrap_or("spec") {
        "spec" => Ok(DocType::Spec),
        "prompt" => Ok(DocType::Prompt),
        "adr" => Ok(DocType::Adr),
        "plan" => Ok(DocType::Plan),
        _ => Ok(DocType::Spec),
    }
}

fn collect_extra_fields(doc_type: &DocType) -> Result<Vec<(String, String)>> {
    let mut extras = Vec::new();

    match doc_type {
        DocType::Spec => {
            let scope = inquire::Text::new(&format!(
                "  {} Scope / boundaries:",
                "\u{25b8}".cyan()
            ))
            .with_default("")
            .with_help_message("What's in scope and out of scope?")
            .prompt()?;
            if !scope.is_empty() {
                extras.push(("Scope".to_string(), scope));
            }
        }
        DocType::Prompt => {
            let target_model = inquire::Text::new(&format!(
                "  {} Target AI model:",
                "\u{25b8}".cyan()
            ))
            .with_default("Any")
            .with_help_message("e.g., Claude, GPT-4, Any")
            .prompt()?;
            extras.push(("Target model".to_string(), target_model));

            let output_format = inquire::Text::new(&format!(
                "  {} Expected output format:",
                "\u{25b8}".cyan()
            ))
            .with_default("")
            .with_help_message("e.g., JSON, Markdown, code — press Enter to skip")
            .prompt()?;
            if !output_format.is_empty() {
                extras.push(("Expected output format".to_string(), output_format));
            }
        }
        DocType::Adr => {
            let question = inquire::Text::new(&format!(
                "  {} Decision question:",
                "\u{25b8}".cyan()
            ))
            .with_help_message("What architectural question are you deciding?")
            .prompt()?;
            if !question.is_empty() {
                extras.push(("Decision question".to_string(), question));
            }
        }
        DocType::Plan => {
            let timeline = inquire::Text::new(&format!(
                "  {} Timeline / deadline:",
                "\u{25b8}".cyan()
            ))
            .with_default("")
            .with_help_message("Optional — press Enter to skip")
            .prompt()?;
            if !timeline.is_empty() {
                extras.push(("Timeline".to_string(), timeline));
            }

            let dependencies = inquire::Text::new(&format!(
                "  {} Key dependencies:",
                "\u{25b8}".cyan()
            ))
            .with_default("")
            .with_help_message("What must be in place first?")
            .prompt()?;
            if !dependencies.is_empty() {
                extras.push(("Dependencies".to_string(), dependencies));
            }
        }
    }

    Ok(extras)
}

fn default_audience(doc_type: &DocType) -> &'static str {
    match doc_type {
        DocType::Spec => "Engineers",
        DocType::Prompt => "AI / LLM users",
        DocType::Adr => "Architecture team",
        DocType::Plan => "Engineering team",
    }
}

// ── Refinement loop ─────────────────────────────────────────────────

fn run_refinement_loop(input: &StructuredInput, ai_backend: backend::Backend) -> Result<String> {
    let mut history: Vec<(String, String)> = Vec::new();

    for round in 0..MAX_REFINEMENT_ROUNDS {
        let msg = match round {
            0 => "AI analyzing your requirements...".to_string(),
            r if r == MAX_REFINEMENT_ROUNDS - 1 => "AI finalizing document...".to_string(),
            _ => format!("AI refining... {}", format!("(round {}/{})", round + 1, MAX_REFINEMENT_ROUNDS).dimmed()),
        };

        let pb = spinner(&msg);
        let response = generate::refine_turn(input, &history, ai_backend, AI_MODEL)?;
        pb.finish_and_clear();

        match response {
            RefinementResponse::FinalDocument(doc) => {
                eprintln!(
                    "  {} Document generated",
                    "\u{2713}".green().bold()
                );
                return Ok(doc);
            }
            RefinementResponse::Questions(questions) => {
                println!();
                eprintln!(
                    "  {} {} {}",
                    "?".blue().bold(),
                    "AI has some questions".bold(),
                    format!("(round {}/{})", round + 1, MAX_REFINEMENT_ROUNDS).dimmed(),
                );
                println!();

                let mut answers = Vec::new();
                for (i, q) in questions.iter().enumerate() {
                    eprintln!(
                        "  {}  {}",
                        format!("{}.", i + 1).cyan().bold(),
                        q
                    );

                    let answer = inquire::Text::new(&format!("  {} >", "\u{25b8}".cyan()))
                        .with_help_message("Type 'done' to skip remaining questions and generate now")
                        .prompt()?;

                    if answer.trim().eq_ignore_ascii_case("done") {
                        history.push((
                            "user".into(),
                            "Please generate the document now with the information you have.".into(),
                        ));
                        // Force final generation on next round
                        break;
                    }
                    answers.push((q.clone(), answer));
                }

                if !answers.is_empty() {
                    // Record AI questions and user answers in history
                    let q_text = questions
                        .iter()
                        .enumerate()
                        .map(|(i, q)| format!("{}. {q}", i + 1))
                        .collect::<Vec<_>>()
                        .join("\n");
                    history.push(("assistant".into(), q_text));

                    let a_text = answers
                        .iter()
                        .map(|(q, a)| format!("Q: {q}\nA: {a}"))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    history.push(("user".into(), a_text));
                }

                println!();
            }
        }
    }

    // Exhausted rounds — force direct generation
    eprintln!(
        "  {} Max refinement rounds reached, generating...",
        "\u{2192}".blue()
    );
    let pb = spinner("Generating final document...");
    let result = generate::generate_direct(input, ai_backend, AI_MODEL)?;
    pb.finish_and_clear();
    eprintln!(
        "  {} Document generated",
        "\u{2713}".green().bold()
    );
    Ok(result)
}

// ── Rendering ───────────────────────────────────────────────────────

fn render_document(
    doc_type: &DocType,
    input: &StructuredInput,
    content: &str,
    arch_root: Option<&str>,
) -> Result<String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut ctx = tera::Context::new();

    ctx.insert("title", &input.title);
    ctx.insert("date", &today);
    ctx.insert("audience", &input.audience);
    ctx.insert("description", &input.description);
    ctx.insert("constraints", &input.constraints);

    match doc_type {
        DocType::Adr => {
            // For ADR, extract sections from the AI content
            let (context_section, decision_section, consequences_section) =
                parse_adr_sections(content);

            let adr_dir = if let Some(root) = arch_root {
                PathBuf::from(root).join("specs/adrs")
            } else {
                PathBuf::from("specs/adrs")
            };
            let next_num = numbering::next_number(&adr_dir).unwrap_or(1);
            ctx.insert("adr_number", &numbering::format_number(next_num));
            ctx.insert("context_section", &context_section);
            ctx.insert("decision_section", &decision_section);
            ctx.insert("consequences_section", &consequences_section);
        }
        DocType::Prompt => {
            // Extract target_model from extras if present
            let target_model = input
                .extra_fields
                .iter()
                .find(|(k, _)| k == "Target model")
                .map(|(_, v)| v.as_str())
                .unwrap_or("Any");
            ctx.insert("target_model", target_model);
            ctx.insert("content", content);
        }
        _ => {
            ctx.insert("content", content);
        }
    }

    tera_renderer::render_generate_template(doc_type.as_str(), &ctx)
}

fn parse_adr_sections(content: &str) -> (String, String, String) {
    let mut context = String::new();
    let mut decision = String::new();
    let mut consequences = String::new();
    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Context") {
            current_section = "context";
            continue;
        } else if trimmed.starts_with("## Decision") {
            current_section = "decision";
            continue;
        } else if trimmed.starts_with("## Consequences") {
            current_section = "consequences";
            continue;
        }

        match current_section {
            "context" => {
                context.push_str(line);
                context.push('\n');
            }
            "decision" => {
                decision.push_str(line);
                decision.push('\n');
            }
            "consequences" => {
                consequences.push_str(line);
                consequences.push('\n');
            }
            _ => {
                // Content before any section header — put in context
                if !trimmed.is_empty() {
                    context.push_str(line);
                    context.push('\n');
                }
            }
        }
    }

    (
        context.trim().to_string(),
        decision.trim().to_string(),
        consequences.trim().to_string(),
    )
}

// ── Output path resolution ──────────────────────────────────────────

fn resolve_output_path(
    doc_type: &DocType,
    title: &str,
    arch_root: Option<&str>,
) -> Result<PathBuf> {
    let slug = numbering::slugify(title);
    let base = if let Some(root) = arch_root {
        PathBuf::from(root)
    } else if let Ok(cwd) = std::env::current_dir() {
        if let Ok(Some((root, _))) = paths::find_repo_meta(&cwd) {
            root
        } else {
            cwd
        }
    } else {
        std::env::current_dir()?
    };

    match doc_type {
        DocType::Spec => Ok(base.join(format!("specs/docs/{slug}.md"))),
        DocType::Prompt => Ok(base.join(format!("specs/prompts/{slug}.md"))),
        DocType::Adr => {
            let adr_dir = base.join("specs/adrs");
            let next_num = numbering::next_number(&adr_dir)?;
            Ok(adr_dir.join(format!(
                "{}-{slug}.md",
                numbering::format_number(next_num)
            )))
        }
        DocType::Plan => Ok(base.join(format!("specs/plans/{slug}.md"))),
    }
}

// ── UX helpers ──────────────────────────────────────────────────────

fn print_banner() {
    println!();
    eprintln!(
        "  {}",
        "\u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}".cyan()
    );
    eprintln!(
        "  {}  {}  {}",
        "\u{2502}".cyan(),
        "archon generate".bold(),
        format!("{:>21}", "\u{2502}").cyan()
    );
    eprintln!(
        "  {}  {}  {}",
        "\u{2502}".cyan(),
        "Interactive document generation".dimmed(),
        format!("{:>6}", "\u{2502}").cyan()
    );
    eprintln!(
        "  {}",
        "\u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}".cyan()
    );
    println!();
}

fn print_summary_card(input: &StructuredInput) {
    eprintln!(
        "  {}",
        "\u{2500}\u{2500}\u{2500} Summary \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".dimmed()
    );
    eprintln!(
        "  {}  {}",
        "Type:".cyan().bold(),
        input.doc_type
    );
    eprintln!(
        "  {}  {}",
        "Title:".cyan().bold(),
        input.title
    );
    eprintln!(
        "  {}  {}",
        "Purpose:".cyan().bold(),
        input.description
    );
    eprintln!(
        "  {}  {}",
        "Audience:".cyan().bold(),
        input.audience
    );
    if !input.constraints.is_empty() {
        eprintln!(
            "  {}  {}",
            "Constraints:".cyan().bold(),
            input.constraints
        );
    }
    for (key, value) in &input.extra_fields {
        eprintln!(
            "  {}  {}",
            format!("{key}:").cyan().bold(),
            value
        );
    }
    eprintln!(
        "  {}",
        "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".dimmed()
    );
}

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(spinner_style());
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}
