use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use arch_tool_ai::backend;
use arch_tool_ai::context::{FillContext, ProjectContext};
use arch_tool_ai::doc_selection;
use arch_tool_ai::fill;
use arch_tool_core::paths;
use arch_tool_core::paths::REPO_META_FILENAME;
use arch_tool_core::profile::Profile;
use arch_tool_core::standards_version::StandardsVersion;
use arch_tool_templates::renderer;

use crate::app::InitArgs;
use crate::output;

struct FileToCreate {
    path: String,
    content: String,
}

pub fn run(args: InitArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    run_in(&cwd, args)
}

pub fn run_in(root: &Path, args: InitArgs) -> Result<()> {
    // Validate standards version
    StandardsVersion::parse(&args.standards_version)
        .map_err(|e| anyhow::anyhow!("invalid --standards-version: {e}"))?;

    // Detect repo name from directory
    let repo_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown-repo")
        .to_string();

    let profile = args.profile.unwrap_or(Profile::RustWorkspace);

    // Build template variables
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), repo_name.clone());
    vars.insert("role".to_string(), profile_to_role(&profile));
    vars.insert("repo_type".to_string(), "rust-workspace".to_string());
    vars.insert(
        "standards_version".to_string(),
        args.standards_version.clone(),
    );
    vars.insert("owner".to_string(), "@team".to_string());

    // Determine base files to create
    let mut files = vec![
        FileToCreate {
            path: REPO_META_FILENAME.to_string(),
            content: renderer::render(renderer::REPO_META_TEMPLATE, &vars),
        },
        FileToCreate {
            path: "AGENTS.md".to_string(),
            content: renderer::render(renderer::AGENTS_MD_TEMPLATE, &vars),
        },
        FileToCreate {
            path: "CLAUDE.md".to_string(),
            content: renderer::render(renderer::CLAUDE_MD_TEMPLATE, &vars),
        },
        FileToCreate {
            path: "specs/docs/architecture.md".to_string(),
            content: renderer::render(renderer::ARCHITECTURE_MD_TEMPLATE, &vars),
        },
        FileToCreate {
            path: "specs/docs/development.md".to_string(),
            content: renderer::render(renderer::DEVELOPMENT_MD_TEMPLATE, &vars),
        },
    ];

    // AI-driven doc selection: if no profile specified and AI is available,
    // ask AI which optional docs are relevant to this project
    if args.profile.is_none() {
        if let Ok(ai_backend) = backend::detect() {
            eprintln!(
                "  {} using AI to select relevant optional docs...",
                "→".blue()
            );
            let context = ProjectContext::from_repo(root);
            let selected =
                doc_selection::select_relevant_docs(&context, ai_backend, "claude-sonnet-4-5-20250514");

            for rel_path in &selected {
                eprintln!("  {} AI selected: {}", "✓".green(), rel_path);
                // Create a minimal template for AI-selected docs
                let doc_name = Path::new(rel_path)
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                let content = format!(
                    "# {} -- {}\n\n\
                     TODO: Document the {} aspects of this project.\n",
                    capitalize(doc_name),
                    repo_name,
                    doc_name.replace('-', " ")
                );

                files.push(FileToCreate {
                    path: rel_path.clone(),
                    content,
                });
            }
        }
    } else {
        add_profile_files(&profile, &vars, &mut files);
    }

    // Execute
    output::print_header(&format!("arch-tool init: {repo_name} (profile: {profile})"));
    println!();

    // Detect AI backend for TODO filling
    let ai_backend = if args.no_ai {
        None
    } else {
        backend::detect().ok()
    };

    // Load fill context if AI is available and we have an arch root
    let fill_ctx = if ai_backend.is_some() {
        let arch_root = paths::resolve_arch_root(args.arch_root.as_deref())
            .ok()
            .flatten();
        arch_root.map(|ar| FillContext::load(&ar))
    } else {
        None
    };

    let ai_model = "claude-sonnet-4-5-20250514";

    for file in &mut files {
        let full_path = root.join(&file.path);

        if args.dry_run {
            output::print_dry_run(&file.path);
            continue;
        }

        if full_path.exists() && !args.force {
            output::print_skipped(&file.path, "already exists, use --force to overwrite");
            continue;
        }

        // AI fill: replace <!-- TODO --> placeholders if AI is available
        if let (Some(ab), Some(ctx)) = (ai_backend, &fill_ctx) {
            if fill::has_todos(&file.content) {
                eprintln!(
                    "  {} filling TODOs in {}...",
                    "→".blue(),
                    file.path
                );
                match fill::fill_todos(&file.path, &file.content, &repo_name, ctx, ab, ai_model) {
                    Ok(filled) => {
                        file.content = filled;
                        eprintln!("  {} AI filled: {}", "✓".green(), file.path);
                    }
                    Err(e) => {
                        eprintln!(
                            "  {} AI fill failed for {}: {e} — using template",
                            "warning:".yellow(),
                            file.path
                        );
                    }
                }
            }
        }

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }

        std::fs::write(&full_path, &file.content)
            .with_context(|| format!("writing {}", full_path.display()))?;

        output::print_created(&file.path);
    }

    if args.dry_run {
        println!("\n  (dry run -- no files written)");
    }

    println!();
    Ok(())
}

fn profile_to_role(profile: &Profile) -> String {
    match profile {
        Profile::RustWorkspace => "library",
        Profile::RuntimeSystem => "core",
        Profile::CompilerAi => "core",
        Profile::CliTool => "tool",
        Profile::ServiceApp => "service",
    }
    .to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn add_profile_files(
    _profile: &Profile,
    _vars: &HashMap<String, String>,
    _files: &mut Vec<FileToCreate>,
) {
    // Profile-specific optional docs will be added here
}
