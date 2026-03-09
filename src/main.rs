mod broadcast;
mod context;
mod dashboard;
mod extract;
mod graph;
mod inject;
mod manifest;
mod render;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::broadcast::Broadcast;
use crate::context::generate_context;
use crate::extract::extract_crate_surface_with_contracts;
use crate::graph::{collect_broadcasts, collect_manifests, Graph};
use crate::inject::{empty_section_markers, has_managed_sections, replace_managed_section};
use crate::manifest::{CrateEntry, Manifest, Role, Rule};

#[derive(Parser)]
#[command(name = "archon", about = "Dependency graph and AI context generator for multi-repo ecosystems")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an archon.yaml manifest for this repo
    Init {
        /// Directory to initialize (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Path to the registry directory (for contract discovery)
        #[arg(long)]
        registry: Option<PathBuf>,
        /// Repo owner (default: @team)
        #[arg(long)]
        owner: Option<String>,
        /// Skip AI inference even if claude CLI is available
        #[arg(long)]
        no_ai: bool,
    },
    /// Extract broadcast and generate context for this repo
    Scan {
        /// Directory containing archon.yaml (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Path to the registry directory
        #[arg(long)]
        registry: Option<PathBuf>,
    },
    /// Assemble graph.yaml from all repos
    Assemble {
        /// Root directory containing repo directories
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Path to the registry directory
        #[arg(long, default_value = "archon-registry")]
        registry: PathBuf,
        /// Distribute context to sibling repos
        #[arg(long)]
        distribute: bool,
        /// Auto-create archon.yaml for repos that have Cargo.toml but no manifest
        #[arg(long)]
        bootstrap: bool,
        /// Skip AI inference even if claude CLI is available
        #[arg(long)]
        no_ai: bool,
    },
    /// Validate graph consistency
    Check {
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from (if no graph.yaml exists)
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Run rules defined in archon.yaml for this repo
    Verify {
        /// Directory containing archon.yaml (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Describe how repos connect in natural language and update manifests
    Describe {
        /// Natural language description of the ecosystem or repo relationships
        description: Vec<String>,
        /// Root directory containing repo directories
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
        /// Show raw AI response for debugging
        #[arg(long)]
        verbose: bool,
    },
    /// Query and visualize the dependency graph
    #[command(subcommand)]
    Graph(GraphCommands),
    /// Interactive architecture dashboard
    Dashboard {
        /// Root directory containing repo directories
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Path to the registry directory
        #[arg(long, default_value = "archon-registry")]
        registry: PathBuf,
        /// Generate a static HTML dashboard instead of launching the TUI
        #[arg(long)]
        web: bool,
    },
}

#[derive(Subcommand)]
enum GraphCommands {
    /// Show the full dependency graph
    Show {
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from (if no graph.yaml exists)
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
        /// Filter by role (core, extension, tool, service, library)
        #[arg(long)]
        role: Option<String>,
    },
    /// Show detailed info about a specific repo
    Info {
        /// Repo name
        name: String,
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Output format
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Show transitive dependencies of a repo
    Deps {
        /// Repo name
        name: String,
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Show only direct dependencies (not transitive)
        #[arg(long)]
        direct: bool,
    },
    /// Show reverse dependencies (what depends on this repo)
    Rdeps {
        /// Repo name
        name: String,
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from
        #[arg(long, default_value = "..")]
        root: PathBuf,
        /// Show only direct dependents (not transitive)
        #[arg(long)]
        direct: bool,
    },
    /// Find dependency path between two repos
    Path {
        /// Source repo
        from: String,
        /// Target repo
        to: String,
        /// Path to graph.yaml or registry directory
        #[arg(long)]
        graph: Option<PathBuf>,
        /// Root directory to assemble from
        #[arg(long, default_value = "..")]
        root: PathBuf,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { path, registry, owner, no_ai } => cmd_init(&path, registry.as_deref(), owner.as_deref(), no_ai),
        Commands::Scan { path, registry } => cmd_scan(&path, registry.as_deref()),
        Commands::Assemble {
            root,
            registry,
            distribute,
            bootstrap,
            no_ai,
        } => cmd_assemble(&root, &registry, distribute, bootstrap, no_ai),
        Commands::Check {
            graph,
            root,
            format,
        } => cmd_check(graph.as_deref(), &root, &format),
        Commands::Verify { path, format } => cmd_verify(&path, &format),
        Commands::Describe {
            description,
            root,
            dry_run,
            verbose,
        } => cmd_describe(&description.join(" "), &root, dry_run, verbose),
        Commands::Graph(sub) => cmd_graph(sub),
        Commands::Dashboard { root, registry, web } => {
            dashboard::run_dashboard(&root, &registry, web)
        }
    };

    if let Err(e) = result {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

// ── AI helpers ──────────────────────────────────────────────────────────────

/// Check if the `claude` CLI is available on PATH.
fn has_claude_cli() -> bool {
    std::process::Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a prompt through `claude` CLI and return the response text.
/// Returns None if claude is unavailable or the call fails.
fn claude_prompt(prompt: &str) -> Option<String> {
    let output = std::process::Command::new("claude")
        .args(["-p", prompt, "--model", "sonnet"])
        .output()
        .ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else {
        None
    }
}

/// AI-inferred metadata for a repo.
#[derive(Default)]
struct AiSuggestion {
    description: Option<String>,
    role: Option<Role>,
    depends_on: Vec<String>,
}

/// Ask AI to analyze a repo and suggest description, role, and dependencies.
fn ai_suggest_init(
    path: &Path,
    name: &str,
    cargo_content: Option<&str>,
    sibling_names: &[String],
) -> Option<AiSuggestion> {
    let cargo_snippet = cargo_content.unwrap_or("(no Cargo.toml)");
    // Truncate cargo content to avoid prompt bloat.
    let cargo_snippet = if cargo_snippet.len() > 2000 {
        &cargo_snippet[..2000]
    } else {
        cargo_snippet
    };

    // Read README if it exists for more context.
    let readme = path.join("README.md");
    let readme_snippet = if readme.exists() {
        std::fs::read_to_string(&readme)
            .ok()
            .map(|c| if c.len() > 1500 { c[..1500].to_string() } else { c })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let siblings_str = if sibling_names.is_empty() {
        "(none known)".to_string()
    } else {
        sibling_names.join(", ")
    };

    let prompt = format!(
        r#"You are analyzing a Rust repository to help configure it for archon (an architecture governance tool).

Repository: {name}
Sibling repos in the ecosystem: {siblings_str}

Cargo.toml:
```
{cargo_snippet}
```

{readme_section}

Based on the above, respond with EXACTLY this format (no extra text):
DESCRIPTION: <one-line description of what this repo does>
ROLE: <one of: core, extension, tool, service, library>
DEPENDS_ON: <comma-separated list of sibling repo names this likely depends on, or NONE>

Role guidelines:
- core: Central system component (runtime, compiler, engine)
- extension: Extends or integrates with a core repo
- tool: Developer tool or CLI utility
- service: Deployed service or API
- library: Shared library consumed by other repos"#,
        readme_section = if readme_snippet.is_empty() {
            String::new()
        } else {
            format!("README.md (excerpt):\n```\n{readme_snippet}\n```")
        }
    );

    let response = claude_prompt(&prompt)?;
    let mut suggestion = AiSuggestion::default();

    for line in response.lines() {
        let line = line.trim();
        if let Some(desc) = line.strip_prefix("DESCRIPTION:") {
            let desc = desc.trim();
            if !desc.is_empty() {
                suggestion.description = Some(desc.to_string());
            }
        } else if let Some(role_str) = line.strip_prefix("ROLE:") {
            suggestion.role = match role_str.trim().to_lowercase().as_str() {
                "core" => Some(Role::Core),
                "extension" => Some(Role::Extension),
                "tool" => Some(Role::Tool),
                "service" => Some(Role::Service),
                "library" => Some(Role::Library),
                _ => None,
            };
        } else if let Some(deps_str) = line.strip_prefix("DEPENDS_ON:") {
            let deps_str = deps_str.trim();
            if deps_str != "NONE" && !deps_str.is_empty() {
                suggestion.depends_on = deps_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && sibling_names.contains(s))
                    .collect();
            }
        }
    }

    Some(suggestion)
}

/// AI-inferred metadata for multiple repos at once (for bootstrap).
struct AiBootstrapEntry {
    name: String,
    role: Role,
    depends_on: Vec<String>,
}

/// Ask AI to infer roles and dependencies for all bootstrapped repos.
fn ai_suggest_bootstrap(
    entries: &[(String, String)], // (name, cargo_toml_snippet)
) -> Vec<AiBootstrapEntry> {
    let repo_list: String = entries
        .iter()
        .map(|(name, cargo)| {
            let snippet = if cargo.len() > 500 { &cargo[..500] } else { cargo.as_str() };
            format!("### {name}\n```toml\n{snippet}\n```")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let all_names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();

    let prompt = format!(
        r#"You are analyzing multiple Rust repositories in an ecosystem to assign roles and dependencies.

Available repos: {}

{repo_list}

For EACH repo, respond with one line in this format:
NAME | ROLE | DEPENDS_ON

Where:
- ROLE is one of: core, extension, tool, service, library
- DEPENDS_ON is a comma-separated list of repos from the list above, or NONE

Respond with ONLY the table lines, no headers or extra text."#,
        all_names.join(", ")
    );

    let response = match claude_prompt(&prompt) {
        Some(r) => r,
        None => return vec![],
    };

    let mut results = Vec::new();
    for line in response.lines() {
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].to_string();
        if !all_names.contains(&name.as_str()) {
            continue;
        }
        let role = match parts[1].to_lowercase().as_str() {
            "core" => Role::Core,
            "extension" => Role::Extension,
            "tool" => Role::Tool,
            "service" => Role::Service,
            _ => Role::Library,
        };
        let depends_on: Vec<String> = if parts[2] == "NONE" || parts[2].is_empty() {
            vec![]
        } else {
            parts[2]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && all_names.contains(&s.as_str()))
                .collect()
        };
        results.push(AiBootstrapEntry {
            name,
            role,
            depends_on,
        });
    }

    results
}

// ── Commands ────────────────────────────────────────────────────────────────

fn cmd_init(path: &Path, registry_path: Option<&Path>, owner_arg: Option<&str>, no_ai: bool) -> Result<()> {
    let manifest_path = Manifest::manifest_path(path);
    if manifest_path.exists() {
        anyhow::bail!("{} already exists", manifest_path.display());
    }

    let is_interactive = std::io::stdin().is_terminal();

    // ── Banner ──
    eprintln!();
    eprintln!();
    eprintln!("  {} — {}", "archon init".cyan().bold(), "Repository onboarding wizard");
    eprintln!();

    // ── Step 1: Registry discovery ──
    eprintln!("  {} {}", "[1/5]".blue().bold(), "Ecosystem discovery".bold());

    // Try to find sibling repos for contract discovery.
    let registry_dir = registry_path
        .map(|p| p.to_path_buf())
        .or_else(|| {
            // Auto-detect: look for archon-registry as sibling.
            let parent = path.canonicalize().ok()?.parent()?.to_path_buf();
            let reg = parent.join("archon-registry");
            if reg.exists() { Some(reg) } else { None }
        });

    // Collect sibling manifests for ecosystem awareness.
    let sibling_manifests = if let Some(ref reg) = registry_dir {
        let root = reg.parent().unwrap_or(Path::new(".."));
        collect_manifests(root).unwrap_or_default()
    } else {
        // Try parent directory.
        let parent = path
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(|pp| pp.to_path_buf()));
        if let Some(p) = parent {
            collect_manifests(&p).unwrap_or_default()
        } else {
            vec![]
        }
    };

    // Collect broadcasts for API awareness.
    let broadcasts = if let Some(ref reg) = registry_dir {
        let bd = reg.join("broadcasts");
        collect_broadcasts(&bd).unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    let sibling_names: Vec<String> = sibling_manifests
        .iter()
        .map(|(_, m)| m.name.clone())
        .collect();

    if sibling_names.is_empty() {
        eprintln!(
            "  {} No sibling repos found — dependency selection will be free-text.",
            "⚠".yellow()
        );
        eprintln!(
            "    Use {} to enable ecosystem-aware init.",
            "--registry <path>".bold()
        );
    } else {
        eprintln!(
            "  {} Found {} sibling repo(s): {}",
            "✓".green().bold(),
            sibling_names.len(),
            sibling_names.join(", ")
        );
    }

    // Detect project name and description from Cargo.toml.
    let cargo_path = path.join("Cargo.toml");
    let cargo_content = if cargo_path.exists() {
        std::fs::read_to_string(&cargo_path).ok()
    } else {
        None
    };
    let default_name = cargo_content
        .as_deref()
        .and_then(extract_cargo_name)
        .unwrap_or_else(|| dir_name(path));

    // ── AI inference ──
    let use_ai = !no_ai && has_claude_cli();
    let ai_suggestion = if use_ai {
        eprintln!(
            "  {} Analyzing repo with AI...",
            "🤖".to_string().cyan()
        );
        let suggestion = ai_suggest_init(path, &default_name, cargo_content.as_deref(), &sibling_names);
        if let Some(ref s) = suggestion {
            eprintln!(
                "  {} AI suggests: {} ({}){}",
                "✓".green().bold(),
                s.description.as_deref().unwrap_or("—").dimmed(),
                s.role.as_ref().map(|r| format!("{r}")).unwrap_or_else(|| "?".into()).bold(),
                if s.depends_on.is_empty() {
                    String::new()
                } else {
                    format!(" deps=[{}]", s.depends_on.join(", "))
                },
            );
        } else {
            eprintln!(
                "  {} AI inference returned no results",
                "⚠".yellow()
            );
        }
        suggestion
    } else {
        if no_ai {
            eprintln!("  {} AI skipped (--no-ai)", "·".dimmed());
        }
        None
    };
    eprintln!();

    // ── Step 2: Repo identity ──
    eprintln!("  {} {}", "[2/5]".blue().bold(), "Repository identity".bold());

    let default_description = ai_suggestion
        .as_ref()
        .and_then(|s| s.description.clone())
        .or_else(|| cargo_content.as_deref().and_then(extract_cargo_description))
        .unwrap_or_else(|| "TODO".to_string());

    if cargo_content.is_some() {
        eprintln!(
            "  {} Detected from {}: {}",
            "✓".green().bold(),
            "Cargo.toml".cyan(),
            default_name.bold(),
        );
    }

    let name = if is_interactive {
        inquire::Text::new(&format!("  {} Project name:", "▸".cyan()))
            .with_default(&default_name)
            .prompt()?
    } else {
        eprintln!("  name: {}", default_name.bold());
        default_name
    };

    let description = if is_interactive {
        inquire::Text::new(&format!("  {} Description:", "▸".cyan()))
            .with_default(&default_description)
            .with_help_message("Brief description of this repo's purpose")
            .prompt()?
    } else {
        eprintln!("  description: {}", default_description.dimmed());
        default_description
    };

    let role_options = vec![
        "core      — Central system component (runtime, compiler, engine)",
        "extension — Extends or integrates with core repos",
        "tool      — Developer tool or CLI utility",
        "service   — Deployed service or API",
        "library   — Shared library consumed by other repos",
    ];
    let ai_role_default = ai_suggestion.as_ref().and_then(|s| s.role.clone());
    let default_role_index = ai_role_default.as_ref().map(|r| match r {
        Role::Core => 0,
        Role::Extension => 1,
        Role::Tool => 2,
        Role::Service => 3,
        Role::Library => 4,
    });
    let role = if is_interactive {
        let role_label = format!("  {} Role:", "▸".cyan());
        let mut selector = inquire::Select::new(&role_label, role_options);
        if let Some(idx) = default_role_index {
            selector = selector.with_starting_cursor(idx);
        }
        let selected = selector.prompt()?;
        match selected.split_whitespace().next().unwrap_or("library") {
            "core" => Role::Core,
            "extension" => Role::Extension,
            "tool" => Role::Tool,
            "service" => Role::Service,
            _ => Role::Library,
        }
    } else {
        ai_role_default.unwrap_or(Role::Library)
    };

    let _owner = owner_arg.unwrap_or("@team");
    eprintln!();

    // ── Step 3: Dependencies (depends_on) ──
    eprintln!("  {} {}", "[3/5]".blue().bold(), "Dependencies".bold());

    let ai_deps = ai_suggestion
        .as_ref()
        .map(|s| s.depends_on.clone())
        .unwrap_or_default();

    let depends_on = if !sibling_names.is_empty() && is_interactive {
        // Build display strings with descriptions.
        let dep_options: Vec<String> = sibling_manifests
            .iter()
            .map(|(_, m)| {
                let api_summary = broadcasts
                    .get(&m.name)
                    .map(|b| {
                        b.crates
                            .iter()
                            .map(|c| render::summarize_crate_surface(c))
                            .collect::<Vec<_>>()
                            .join("; ")
                    })
                    .unwrap_or_default();
                if api_summary.is_empty() {
                    format!("{} ({}) — {}", m.name, m.role, m.description)
                } else {
                    format!("{} ({}) — {} [{}]", m.name, m.role, m.description, api_summary)
                }
            })
            .collect();

        // Pre-select AI-suggested dependencies.
        let default_indices: Vec<usize> = sibling_manifests
            .iter()
            .enumerate()
            .filter(|(_, (_, m))| ai_deps.contains(&m.name))
            .map(|(i, _)| i)
            .collect();

        let prompt_label = format!("  {} Which repos does {} depend on?", "▸".cyan(), name.bold());
        eprintln!();
        let mut multi = inquire::MultiSelect::new(&prompt_label, dep_options)
            .with_help_message("Space to select, Enter to confirm");
        if !default_indices.is_empty() {
            multi = multi.with_default(&default_indices);
        }
        let selected = multi.prompt()?;

        // Extract repo names from the display strings.
        selected
            .iter()
            .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if is_interactive {
        let default_deps = ai_deps.join(", ");
        let deps_str = inquire::Text::new(
            &format!("  {} Dependencies (comma-separated, or empty):", "▸".cyan()),
        )
        .with_default(&default_deps)
        .prompt()?;
        deps_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        ai_deps
    };

    if depends_on.is_empty() {
        eprintln!("  {} No dependencies selected", "·".dimmed());
    } else {
        eprintln!(
            "  {} depends on: {}",
            "✓".green().bold(),
            depends_on.join(", ")
        );
    }
    eprintln!();

    // ── Step 4: Provides (stable API contracts) ──
    eprintln!("  {} {}", "[4/5]".blue().bold(), "Provides".bold());
    eprintln!(
        "  {} {}",
        "·".dimmed(),
        "Named contracts other repos can depend on (e.g. \"execution-plan\", \"sandbox-runtime\").".dimmed()
    );
    eprintln!(
        "  {} {}",
        "·".dimmed(),
        "Tag public items with /// @contract(name) and they'll be auto-detected here.".dimmed()
    );

    // Auto-detect contracts from @contract() annotations.
    let auto_contracts = discover_contracts(path);

    let provides = if is_interactive {
        if !auto_contracts.is_empty() {
            eprintln!(
                "  {} Auto-detected from @contract() annotations: {}",
                "✓".green().bold(),
                auto_contracts.join(", ")
            );
        }
        let provides_str = inquire::Text::new(
            &format!("  {} Contracts this repo provides (comma-separated, or empty):", "▸".cyan()),
        )
        .with_default(&auto_contracts.join(", "))
        .with_help_message("Stable API contracts that downstream repos can depend on. Leave empty if none.")
        .prompt()?;
        provides_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        auto_contracts
    };

    if provides.is_empty() {
        eprintln!("  {} No contracts declared — this repo consumes but doesn't provide named APIs", "·".dimmed());
    } else {
        eprintln!(
            "  {} provides: {}",
            "✓".green().bold(),
            provides.join(", ")
        );
    }
    eprintln!();

    // ── Step 5: Crate classification (automatic) ──
    eprintln!("  {} {}", "[5/5]".blue().bold(), "Crate classification".bold());

    let crates = detect_workspace_crates(path);
    if let Some(ref entries) = crates {
        for entry in entries {
            let class = if entry.public { "public" } else { "internal" };
            eprintln!(
                "  {} {:<30} → {}",
                "✓".green().bold(),
                entry.name,
                class
            );
        }
        eprintln!(
            "  {} crate(s) classified ({} public)",
            entries.len(),
            entries.iter().filter(|c| c.public).count()
        );
    } else {
        eprintln!("  {} No Cargo.toml found — skipping crate classification", "·".dimmed());
    }
    eprintln!();

    // ── Build and save manifest ──
    // Save registry path if one was provided or discovered.
    let registry_value = registry_dir.as_ref().map(|reg| {
        // Store as provided (relative or absolute).
        reg.to_string_lossy().to_string()
    });

    let manifest = Manifest {
        name: name.clone(),
        description: description.clone(),
        role: role.clone(),
        depends_on: depends_on.clone(),
        provides: provides.clone(),
        crates,
        auto_update: None,
        registry: registry_value,
        rules: default_rules(),
    };

    manifest.save(path)?;

    // Add context markers to CLAUDE.md and AGENTS.md if they exist.
    for filename in ["CLAUDE.md", "AGENTS.md"] {
        let md_path = path.join(filename);
        if md_path.exists() {
            let content = std::fs::read_to_string(&md_path)?;
            if !has_managed_sections(&content) {
                let markers = inject::empty_section_markers();
                let updated = format!("{}\n\n{}\n", content.trim_end(), markers);
                std::fs::write(&md_path, updated)?;
            }
        }
    }
    // ── Summary card ──
    eprintln!();
    eprintln!("  {}", "─── Summary ──────────────────────────────".dimmed());
    eprintln!("  {} {}", "Name:".dimmed(), name.bold().cyan());
    eprintln!("  {} {}", "Desc:".dimmed(), description.dimmed());
    eprintln!("  {} {}", "Role:".dimmed(), format!("{role}").bold());
    eprintln!("  {} {}", "Deps:".dimmed(), if depends_on.is_empty() { "(none)".dimmed().to_string() } else { depends_on.join(", ") });
    eprintln!("  {} {}", "Provides:".dimmed(), if provides.is_empty() { "(none)".dimmed().to_string() } else { provides.join(", ") });
    eprintln!("  {}", "───────────────────────────────────────────".dimmed());
    eprintln!();

    eprintln!(
        "  {} Created {}",
        "✓".green().bold(),
        manifest_path.display().to_string().cyan()
    );
    for filename in ["CLAUDE.md", "AGENTS.md"] {
        if path.join(filename).exists() {
            eprintln!(
                "  {} Added context markers to {}",
                "✓".green().bold(),
                filename.cyan(),
            );
        }
    }

    eprintln!();
    eprintln!("  {}", "Next steps:".bold());
    eprintln!("    {} {}  — extract public API and generate context", "archon".cyan(), "scan".bold());
    eprintln!("    {} {} — run conformance rules", "archon".cyan(), "verify".bold());
    if registry_dir.is_some() {
        eprintln!("    {} {} — rebuild the ecosystem graph", "archon".cyan(), "assemble".bold());
    } else {
        eprintln!("    {} {} — rebuild the ecosystem graph (needs {})", "archon".cyan(), "assemble".bold(), "--registry".dimmed());
    }
    eprintln!();

    Ok(())
}

fn cmd_describe(description: &str, root: &Path, dry_run: bool, verbose: bool) -> Result<()> {
    if !has_claude_cli() {
        anyhow::bail!(
            "describe requires the claude CLI. Install Claude Code: https://claude.ai/claude-code"
        );
    }

    eprintln!();
    eprintln!(
        "  {} — {}",
        "archon describe".cyan().bold(),
        "Natural language ecosystem configuration"
    );
    eprintln!();

    // Collect existing manifests.
    let existing = collect_manifests(root).unwrap_or_default();
    let existing_summary: String = if existing.is_empty() {
        "(no existing manifests found)".into()
    } else {
        existing
            .iter()
            .map(|(_, m)| {
                format!(
                    "- {} ({}): deps=[{}], provides=[{}] — {}",
                    m.name,
                    m.role,
                    m.depends_on.join(", "),
                    m.provides.join(", "),
                    m.description,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Discover repos with Cargo.toml but no manifest.
    let mut uninitialized = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path.join("Cargo.toml").exists()
                && !path.join("archon.yaml").exists()
            {
                let cargo = std::fs::read_to_string(path.join("Cargo.toml")).unwrap_or_default();
                let name =
                    extract_cargo_name(&cargo).unwrap_or_else(|| dir_name(&path));
                uninitialized.push(name);
            }
        }
    }

    let uninitialized_str = if uninitialized.is_empty() {
        "(none)".to_string()
    } else {
        uninitialized.join(", ")
    };

    eprintln!(
        "  {} {} existing manifest(s), {} uninitialized repo(s)",
        "·".dimmed(),
        existing.len(),
        uninitialized.len(),
    );
    eprintln!(
        "  {} \"{}\"",
        "▸".cyan(),
        description.dimmed()
    );
    eprintln!();

    let prompt = format!(
        r#"You are configuring an archon ecosystem (architecture governance for multi-repo Rust projects).

## Current state

Existing manifests:
{existing_summary}

Repos without manifests yet: {uninitialized_str}

## User's description

"{description}"

## Task

Based on the user's description, output a YAML document that specifies updates to apply. For each repo mentioned, output the fields that should change. Only include repos that the user's description is relevant to. For new repos (not yet initialized), include all fields.

Format — output ONLY valid YAML, no explanation:

```yaml
repos:
  - name: <repo-name>
    description: <one-line description>  # optional, omit if no change
    role: <core|extension|tool|service|library>  # optional, omit if no change
    depends_on:  # optional, omit if no change
      - <repo-name>
    provides:  # optional, omit if no change
      - <capability-name>
```

Rules:
- Only reference repos that exist in the current state or uninitialized list
- `depends_on` entries must reference other repos by name
- Roles: core (runtime/compiler/engine), extension (extends core), tool (CLI/dev tool), service (deployed), library (shared lib)
- If the user's description doesn't mention a repo, don't include it"#
    );

    eprintln!(
        "  {} Thinking...",
        "🤖".to_string().cyan()
    );

    let response = claude_prompt(&prompt)
        .ok_or_else(|| anyhow::anyhow!("AI inference failed — claude returned no response"))?;

    if verbose {
        eprintln!();
        eprintln!("  {} Raw AI response:", "debug:".yellow().bold());
        for line in response.lines() {
            eprintln!("    {}", line.dimmed());
        }
    }

    // Parse the YAML from the response (strip markdown fences if present).
    let yaml_str = extract_yaml_block(&response);

    if verbose {
        eprintln!();
        eprintln!("  {} Extracted YAML:", "debug:".yellow().bold());
        for line in yaml_str.lines() {
            eprintln!("    {}", line.dimmed());
        }
    }

    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&yaml_str).context("failed to parse AI response as YAML")?;

    let repos = parsed
        .get("repos")
        .and_then(|r| r.as_sequence());

    let repos = match repos {
        Some(r) if !r.is_empty() => r,
        _ => {
            eprintln!();
            eprintln!("  {}", "─── Changes ──────────────────────────────".dimmed());
            eprintln!("  {}", "───────────────────────────────────────────".dimmed());
            eprintln!();
            eprintln!(
                "  {} The described state already matches the current configuration",
                "✓".green(),
            );
            eprintln!();
            return Ok(());
        }
    };

    // Apply updates.
    eprintln!();
    eprintln!("  {}", "─── Changes ──────────────────────────────".dimmed());

    let mut updates = 0u32;
    let mut creates = 0u32;
    let mut already_satisfied: Vec<String> = Vec::new();

    for repo_value in repos {
        let name = repo_value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("repo entry missing 'name'"))?;

        // Find existing manifest or create new one.
        let existing_entry = existing.iter().find(|(_, m)| m.name == name);

        if let Some((repo_path, existing_manifest)) = existing_entry {
            // Update existing manifest.
            let mut manifest = existing_manifest.clone();
            let mut changed = false;

            if let Some(desc) = repo_value.get("description").and_then(|v| v.as_str()) {
                if desc != manifest.description {
                    eprintln!(
                        "  {} {} description: {} → {}",
                        "~".yellow().bold(),
                        name.bold(),
                        manifest.description.dimmed(),
                        desc.green(),
                    );
                    manifest.description = desc.to_string();
                    changed = true;
                }
            }

            if let Some(role_str) = repo_value.get("role").and_then(|v| v.as_str()) {
                let new_role = match role_str {
                    "core" => Role::Core,
                    "extension" => Role::Extension,
                    "tool" => Role::Tool,
                    "service" => Role::Service,
                    _ => Role::Library,
                };
                if format!("{}", new_role) != format!("{}", manifest.role) {
                    eprintln!(
                        "  {} {} role: {} → {}",
                        "~".yellow().bold(),
                        name.bold(),
                        format!("{}", manifest.role).dimmed(),
                        format!("{}", new_role).green(),
                    );
                    manifest.role = new_role;
                    changed = true;
                }
            }

            if let Some(deps) = repo_value.get("depends_on").and_then(|v| v.as_sequence()) {
                let new_deps: Vec<String> = deps
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if new_deps != manifest.depends_on {
                    eprintln!(
                        "  {} {} depends_on: [{}] → [{}]",
                        "~".yellow().bold(),
                        name.bold(),
                        manifest.depends_on.join(", ").dimmed(),
                        new_deps.join(", ").green(),
                    );
                    manifest.depends_on = new_deps;
                    changed = true;
                }
            }

            if let Some(provides) = repo_value.get("provides").and_then(|v| v.as_sequence()) {
                let new_provides: Vec<String> = provides
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if new_provides != manifest.provides {
                    eprintln!(
                        "  {} {} provides: [{}] → [{}]",
                        "~".yellow().bold(),
                        name.bold(),
                        manifest.provides.join(", ").dimmed(),
                        new_provides.join(", ").green(),
                    );
                    manifest.provides = new_provides;
                    changed = true;
                }
            }

            if changed {
                if dry_run {
                    eprintln!(
                        "  {} would update {}",
                        "dry-run:".yellow().bold(),
                        repo_path.join("archon.yaml").display()
                    );
                } else {
                    manifest.save(repo_path)?;
                    eprintln!(
                        "  {} {}",
                        "✓".green().bold(),
                        repo_path.join("archon.yaml").display().to_string().cyan(),
                    );
                }
                updates += 1;
            } else {
                already_satisfied.push(name.to_string());
            }
        } else {
            // Create new manifest.
            let desc = repo_value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("TODO")
                .to_string();
            let role = match repo_value.get("role").and_then(|v| v.as_str()).unwrap_or("library") {
                "core" => Role::Core,
                "extension" => Role::Extension,
                "tool" => Role::Tool,
                "service" => Role::Service,
                _ => Role::Library,
            };
            let depends_on: Vec<String> = repo_value
                .get("depends_on")
                .and_then(|v| v.as_sequence())
                .map(|deps| {
                    deps.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let provides: Vec<String> = repo_value
                .get("provides")
                .and_then(|v| v.as_sequence())
                .map(|p| {
                    p.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            // Find the repo directory.
            let repo_path = root.join(name);
            let crates = if repo_path.exists() {
                detect_workspace_crates(&repo_path)
            } else {
                None
            };

            eprintln!(
                "  {} {} ({}) deps=[{}] provides=[{}]",
                "+".green().bold(),
                name.bold(),
                format!("{role}").cyan(),
                depends_on.join(", "),
                provides.join(", "),
            );
            eprintln!("    {}", desc.dimmed());

            let manifest = Manifest {
                name: name.to_string(),
                description: desc,
                role,
                depends_on,
                provides,
                crates,
                auto_update: None,
                registry: None,
                rules: default_rules(),
            };

            if dry_run {
                eprintln!(
                    "  {} would create {}",
                    "dry-run:".yellow().bold(),
                    repo_path.join("archon.yaml").display()
                );
            } else if repo_path.exists() {
                manifest.save(&repo_path)?;
                eprintln!(
                    "  {} {}",
                    "✓".green().bold(),
                    repo_path.join("archon.yaml").display().to_string().cyan(),
                );
            } else {
                eprintln!(
                    "  {} directory {} does not exist — skipping",
                    "⚠".yellow(),
                    repo_path.display(),
                );
            }
            creates += 1;
        }
    }

    eprintln!("  {}", "───────────────────────────────────────────".dimmed());
    eprintln!();

    if updates == 0 && creates == 0 {
        if already_satisfied.is_empty() {
            eprintln!("  {} No changes to apply", "·".dimmed());
        } else {
            eprintln!(
                "  {} Already up to date: {}",
                "✓".green(),
                already_satisfied.join(", ").cyan(),
            );
            eprintln!(
                "  {} The described relationship already exists in the manifest(s)",
                "·".dimmed(),
            );
        }
    } else {
        let action = if dry_run { "would apply" } else { "applied" };
        eprintln!(
            "  {} {} {} update(s), {} new manifest(s)",
            "ok:".green().bold(),
            action,
            updates,
            creates,
        );
    }

    if dry_run {
        eprintln!();
        eprintln!(
            "  Run without {} to apply changes.",
            "--dry-run".bold()
        );
    }

    eprintln!();
    Ok(())
}

/// Extract YAML content from a response that might be wrapped in markdown fences.
fn extract_yaml_block(response: &str) -> String {
    // Try to find ```yaml ... ``` block.
    if let Some(start) = response.find("```yaml") {
        let after_fence = &response[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim().to_string();
        }
    }
    // Try plain ``` block.
    if let Some(start) = response.find("```") {
        let after_fence = &response[start + 3..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim().to_string();
        }
    }
    // Return as-is.
    response.trim().to_string()
}

fn cmd_scan(path: &Path, registry: Option<&Path>) -> Result<()> {
    let manifest = Manifest::load(path).context("loading manifest")?;
    println!("Scanning {}...", manifest.name.bold());

    // Determine which crates to scan.
    let crate_entries = manifest
        .crates
        .clone()
        .unwrap_or_else(|| detect_workspace_crates(path).unwrap_or_default());

    let public_crates: Vec<&CrateEntry> = crate_entries.iter().filter(|c| c.public).collect();

    // Extract public API surfaces.
    let mut crate_surfaces = Vec::new();
    for entry in &public_crates {
        let src_dir = find_crate_src(path, &entry.name);
        if let Some(src) = src_dir {
            match extract_crate_surface_with_contracts(&entry.name, &src) {
                Ok(surface) => {
                    println!(
                        "  {} {} types, {} traits, {} functions",
                        entry.name.cyan(),
                        surface.types.len(),
                        surface.traits.len(),
                        surface.functions.len()
                    );
                    crate_surfaces.push(surface);
                }
                Err(e) => {
                    eprintln!("  {} {}: {}", "warn:".yellow().bold(), entry.name, e);
                }
            }
        } else {
            eprintln!(
                "  {} could not find src/ for {}",
                "warn:".yellow().bold(),
                entry.name
            );
        }
    }

    // Build broadcast.
    let broadcast = Broadcast {
        repo: manifest.name.clone(),
        version: "0.1.0".into(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        crates: crate_surfaces,
    };

    // Write broadcast YAML.
    let broadcast_dir = path.join(".archon");
    std::fs::create_dir_all(&broadcast_dir)?;
    let broadcast_path = broadcast_dir.join("broadcast.yaml");
    let yaml = serde_yaml::to_string(&broadcast)?;
    std::fs::write(&broadcast_path, &yaml)?;
    println!(
        "{} Wrote {}",
        "ok:".green().bold(),
        broadcast_path.display()
    );

    // If registry is accessible, copy broadcast and generate full context.
    // Resolution: --registry flag > manifest.registry > sibling auto-detect.
    let manifest_registry = manifest.registry.as_ref().map(|r| path.join(r));
    let effective_registry = registry
        .map(|p| p.to_path_buf())
        .or(manifest_registry)
        .or_else(|| find_sibling_registry(path).map(|p| p.to_path_buf()));

    let mut context_written = false;

    if let Some(reg_path) = effective_registry {
        let reg_path = reg_path.to_path_buf();
        // Copy broadcast to registry.
        let reg_broadcast_dir = reg_path.join("broadcasts");
        std::fs::create_dir_all(&reg_broadcast_dir)?;
        let reg_broadcast_path = reg_broadcast_dir.join(format!("{}.yaml", manifest.name));
        std::fs::write(&reg_broadcast_path, &yaml)?;

        // Load graph + broadcasts for context generation.
        let graph_path = reg_path.join("graph.yaml");
        if graph_path.exists() {
            let graph = Graph::load(&graph_path)?;
            let broadcasts = collect_broadcasts(&reg_broadcast_dir)?;

            if let Some(node) = graph.find_node(&manifest.name) {
                let ctx = generate_context(node, &graph, &broadcasts);
                inject_context(path, &ctx)?;
                context_written = true;
                println!("{} Wrote .archon/context.md", "ok:".green().bold());
            }
        }
    }

    // If no registry/graph was available, still generate a standalone context
    // from just this repo's own broadcast data.
    if !context_written {
        use std::collections::HashMap;
        let node = crate::graph::GraphNode {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            role: manifest.role.clone(),
            depends_on: manifest.depends_on.clone(),
            provides: manifest.provides.clone(),
            dependents: vec![],
        };
        let graph = Graph {
            generated_at: chrono::Utc::now().to_rfc3339(),
            nodes: vec![node.clone()],
        };
        let mut broadcasts = HashMap::new();
        broadcasts.insert(manifest.name.clone(), broadcast);
        let ctx = generate_context(&node, &graph, &broadcasts);
        inject_context(path, &ctx)?;
        println!("{} Wrote .archon/context.md (standalone — run assemble for full graph)", "ok:".green().bold());
    }

    Ok(())
}

fn cmd_assemble(root: &Path, registry: &Path, distribute: bool, bootstrap: bool, no_ai: bool) -> Result<()> {
    if bootstrap {
        bootstrap_siblings(root, no_ai)?;
    }

    println!("Assembling graph from {}...", root.display());

    let manifest_pairs = collect_manifests(root)?;
    if manifest_pairs.is_empty() {
        anyhow::bail!("no archon.yaml manifests found in {}", root.display());
    }

    let manifests: Vec<Manifest> = manifest_pairs.iter().map(|(_, m)| m.clone()).collect();
    let graph = Graph::assemble(manifests);

    println!("Found {} repos:", graph.nodes.len());
    for node in &graph.nodes {
        println!(
            "  {} ({}) deps=[{}] dependents=[{}]",
            node.name.bold(),
            node.role,
            node.depends_on.join(", "),
            node.dependents.join(", "),
        );
    }

    // Validate.
    let violations = graph.check();
    for v in &violations {
        eprintln!("  {} {}", "violation:".red().bold(), v);
    }

    // Save graph.
    std::fs::create_dir_all(registry)?;
    let graph_path = registry.join("graph.yaml");
    graph.save(&graph_path)?;
    println!("{} Wrote {}", "ok:".green().bold(), graph_path.display());

    // Collect broadcasts and generate context.
    let broadcasts_dir = registry.join("broadcasts");
    let broadcasts = collect_broadcasts(&broadcasts_dir).unwrap_or_default();
    let context_dir = registry.join("context");
    std::fs::create_dir_all(&context_dir)?;

    for node in &graph.nodes {
        let ctx = generate_context(node, &graph, &broadcasts);
        let context_path = context_dir.join(format!("{}.context.md", node.name));
        std::fs::write(&context_path, &ctx)?;
    }
    println!(
        "{} Generated context for {} repos",
        "ok:".green().bold(),
        graph.nodes.len()
    );

    // Distribute to sibling repos.
    if distribute {
        println!("Distributing context to repos...");
        for (repo_path, manifest) in &manifest_pairs {
            if let Some(node) = graph.find_node(&manifest.name) {
                let ctx = generate_context(node, &graph, &broadcasts);
                match inject_context(repo_path, &ctx) {
                    Ok(true) => println!("  {} {}", "updated:".green(), manifest.name),
                    Ok(false) => println!("  {} {} (no markers)", "skipped:".yellow(), manifest.name),
                    Err(e) => eprintln!("  {} {}: {}", "error:".red(), manifest.name, e),
                }
            }
        }
    }

    if !violations.is_empty() {
        anyhow::bail!("{} violation(s) found", violations.len());
    }

    Ok(())
}

fn cmd_check(graph_path: Option<&Path>, root: &Path, format: &OutputFormat) -> Result<()> {
    let graph = if let Some(path) = graph_path {
        Graph::load(path)?
    } else {
        let manifest_pairs = collect_manifests(root)?;
        let manifests: Vec<Manifest> = manifest_pairs.into_iter().map(|(_, m)| m).collect();
        if manifests.is_empty() {
            anyhow::bail!("no archon.yaml manifests found");
        }
        Graph::assemble(manifests)
    };

    let violations = graph.check();

    match format {
        OutputFormat::Text => {
            if violations.is_empty() {
                println!(
                    "{} Graph is consistent ({} nodes)",
                    "ok:".green().bold(),
                    graph.nodes.len()
                );
            } else {
                for v in &violations {
                    eprintln!("{} {}", "violation:".red().bold(), v);
                }
                eprintln!(
                    "\n{} {} violation(s) in {} nodes",
                    "fail:".red().bold(),
                    violations.len(),
                    graph.nodes.len()
                );
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "nodes": graph.nodes.len(),
                "violations": violations.iter().map(|v| {
                    serde_json::json!({
                        "node": v.node,
                        "message": v.message,
                    })
                }).collect::<Vec<_>>(),
                "ok": violations.is_empty(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    if !violations.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_verify(path: &Path, format: &OutputFormat) -> Result<()> {
    let manifest = Manifest::load(path).context("loading manifest")?;

    if manifest.rules.is_empty() {
        println!(
            "{} No rules defined in {}",
            "ok:".green().bold(),
            manifest.name
        );
        return Ok(());
    }

    println!("Verifying {} ({} rules)...", manifest.name.bold(), manifest.rules.len());

    let mut failures = Vec::new();

    for rule in &manifest.rules {
        let desc = rule.description.as_deref().unwrap_or(&rule.id);
        print!("  {} {} ... ", "rule:".cyan().bold(), desc);

        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&rule.run)
            .current_dir(path)
            .output()
            .with_context(|| format!("running rule '{}'", rule.id))?;

        if output.status.success() {
            println!("{}", "pass".green());
        } else {
            println!("{}", "FAIL".red().bold());
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            failures.push((rule.id.clone(), desc.to_string(), stdout.to_string(), stderr.to_string()));
        }
    }

    match format {
        OutputFormat::Text => {
            if failures.is_empty() {
                println!(
                    "\n{} All {} rules passed for {}",
                    "ok:".green().bold(),
                    manifest.rules.len(),
                    manifest.name
                );
            } else {
                eprintln!();
                for (id, desc, stdout, stderr) in &failures {
                    eprintln!("{} {} ({})", "FAIL:".red().bold(), desc, id);
                    if !stdout.is_empty() {
                        let truncated: String = stdout.lines().take(20).collect::<Vec<_>>().join("\n");
                        eprintln!("{}", truncated);
                    }
                    if !stderr.is_empty() {
                        let truncated: String = stderr.lines().take(20).collect::<Vec<_>>().join("\n");
                        eprintln!("{}", truncated);
                    }
                }
                eprintln!(
                    "\n{} {}/{} rules failed for {}",
                    "fail:".red().bold(),
                    failures.len(),
                    manifest.rules.len(),
                    manifest.name
                );
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "repo": manifest.name,
                "total": manifest.rules.len(),
                "passed": manifest.rules.len() - failures.len(),
                "failed": failures.len(),
                "failures": failures.iter().map(|(id, desc, stdout, stderr)| {
                    serde_json::json!({
                        "id": id,
                        "description": desc,
                        "stdout": stdout,
                        "stderr": stderr,
                    })
                }).collect::<Vec<_>>(),
                "ok": failures.is_empty(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    if !failures.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

// ── Graph query commands ────────────────────────────────────────────────────

/// Load or assemble a graph from the given arguments.
fn load_or_assemble_graph(graph_path: Option<&Path>, root: &Path) -> Result<Graph> {
    if let Some(path) = graph_path {
        // If it's a directory (registry), look for graph.yaml inside it.
        let actual = if path.is_dir() {
            path.join("graph.yaml")
        } else {
            path.to_path_buf()
        };
        Graph::load(&actual)
    } else {
        let manifest_pairs = collect_manifests(root)?;
        let manifests: Vec<Manifest> = manifest_pairs.into_iter().map(|(_, m)| m).collect();
        if manifests.is_empty() {
            anyhow::bail!("no archon.yaml manifests found in {}", root.display());
        }
        Ok(Graph::assemble(manifests))
    }
}

fn cmd_graph(sub: GraphCommands) -> Result<()> {
    match sub {
        GraphCommands::Show { graph, root, format, role } => {
            cmd_graph_show(graph.as_deref(), &root, &format, role.as_deref())
        }
        GraphCommands::Info { name, graph, root, format } => {
            cmd_graph_info(&name, graph.as_deref(), &root, &format)
        }
        GraphCommands::Deps { name, graph, root, direct } => {
            cmd_graph_deps(&name, graph.as_deref(), &root, direct)
        }
        GraphCommands::Rdeps { name, graph, root, direct } => {
            cmd_graph_rdeps(&name, graph.as_deref(), &root, direct)
        }
        GraphCommands::Path { from, to, graph, root } => {
            cmd_graph_path(&from, &to, graph.as_deref(), &root)
        }
    }
}

fn cmd_graph_show(graph_path: Option<&Path>, root: &Path, format: &OutputFormat, role_filter: Option<&str>) -> Result<()> {
    let graph = load_or_assemble_graph(graph_path, root)?;

    let nodes: Vec<&graph::GraphNode> = if let Some(role_str) = role_filter {
        let role_str = role_str.to_lowercase();
        graph.nodes.iter().filter(|n| n.role.to_string() == role_str).collect()
    } else {
        graph.nodes.iter().collect()
    };

    match format {
        OutputFormat::Text => {
            println!("{} {} repos in graph", "archon:".cyan().bold(), nodes.len());
            println!();

            // Table header.
            println!(
                "  {:<28} {:<12} {:<30} {}",
                "REPO".bold(),
                "ROLE".bold(),
                "DEPENDS ON".bold(),
                "DEPENDED BY".bold(),
            );
            println!("  {}", "─".repeat(90).dimmed());

            for node in &nodes {
                let deps = if node.depends_on.is_empty() {
                    "—".dimmed().to_string()
                } else {
                    node.depends_on.join(", ")
                };
                let rdeps = if node.dependents.is_empty() {
                    "—".dimmed().to_string()
                } else {
                    node.dependents.join(", ")
                };
                println!(
                    "  {:<28} {:<12} {:<30} {}",
                    node.name,
                    node.role.to_string().dimmed(),
                    deps,
                    rdeps,
                );
            }

            if let Some(r) = role_filter {
                println!("\n  {} filtered by role={}", "·".dimmed(), r);
            }
        }
        OutputFormat::Json => {
            let output: Vec<_> = nodes.iter().map(|n| {
                serde_json::json!({
                    "name": n.name,
                    "description": n.description,
                    "role": n.role.to_string(),
                    "depends_on": n.depends_on,
                    "provides": n.provides,
                    "dependents": n.dependents,
                })
            }).collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

fn cmd_graph_info(name: &str, graph_path: Option<&Path>, root: &Path, format: &OutputFormat) -> Result<()> {
    let graph = load_or_assemble_graph(graph_path, root)?;
    let node = graph.find_node(name)
        .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in graph", name))?;

    match format {
        OutputFormat::Text => {
            println!("{} {}", "repo:".cyan().bold(), node.name.bold());
            println!("  {} {}", "description:".dimmed(), node.description);
            println!("  {} {}", "role:".dimmed(), node.role);

            if !node.provides.is_empty() {
                println!("  {} {}", "provides:".dimmed(), node.provides.join(", "));
            }

            println!();

            if node.depends_on.is_empty() {
                println!("  {} none", "depends on:".dimmed());
            } else {
                println!("  {}", "depends on:".dimmed());
                for dep in &node.depends_on {
                    let dep_node = graph.find_node(dep);
                    let desc = dep_node.map(|n| n.description.as_str()).unwrap_or("?");
                    println!("    {} {} {}", "→".green(), dep.bold(), format!("({})", desc).dimmed());
                }
            }

            println!();

            if node.dependents.is_empty() {
                println!("  {} none", "depended by:".dimmed());
            } else {
                println!("  {}", "depended by:".dimmed());
                for dep in &node.dependents {
                    let dep_node = graph.find_node(dep);
                    let desc = dep_node.map(|n| n.description.as_str()).unwrap_or("?");
                    println!("    {} {} {}", "←".yellow(), dep.bold(), format!("({})", desc).dimmed());
                }
            }

            // Transitive stats.
            let trans_deps = graph.transitive_deps(name);
            let trans_rdeps = graph.transitive_rdeps(name);
            if trans_deps.len() != node.depends_on.len() || trans_rdeps.len() != node.dependents.len() {
                println!();
                println!("  {} {} direct deps, {} transitive | {} direct dependents, {} transitive",
                    "·".dimmed(),
                    node.depends_on.len(), trans_deps.len(),
                    node.dependents.len(), trans_rdeps.len(),
                );
            }
        }
        OutputFormat::Json => {
            let trans_deps = graph.transitive_deps(name);
            let trans_rdeps = graph.transitive_rdeps(name);
            let output = serde_json::json!({
                "name": node.name,
                "description": node.description,
                "role": node.role.to_string(),
                "provides": node.provides,
                "depends_on": node.depends_on,
                "dependents": node.dependents,
                "transitive_deps": trans_deps,
                "transitive_rdeps": trans_rdeps,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

fn cmd_graph_deps(name: &str, graph_path: Option<&Path>, root: &Path, direct: bool) -> Result<()> {
    let graph = load_or_assemble_graph(graph_path, root)?;
    let node = graph.find_node(name)
        .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in graph", name))?;

    if direct {
        if node.depends_on.is_empty() {
            println!("{} has no dependencies", name.bold());
        } else {
            println!("{} depends on ({} direct):", name.bold(), node.depends_on.len());
            for dep in &node.depends_on {
                println!("  {} {}", "→".green(), dep);
            }
        }
    } else {
        let deps = graph.transitive_deps(name);
        if deps.is_empty() {
            println!("{} has no dependencies", name.bold());
        } else {
            println!("{} depends on ({} transitive, {} direct):", name.bold(), deps.len(), node.depends_on.len());
            for dep in &deps {
                let marker = if node.depends_on.contains(dep) { "→" } else { "⤷" };
                let label = if node.depends_on.contains(dep) {
                    "".to_string()
                } else {
                    " (transitive)".dimmed().to_string()
                };
                println!("  {} {}{}", marker.green(), dep, label);
            }
        }
    }

    Ok(())
}

fn cmd_graph_rdeps(name: &str, graph_path: Option<&Path>, root: &Path, direct: bool) -> Result<()> {
    let graph = load_or_assemble_graph(graph_path, root)?;
    let node = graph.find_node(name)
        .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in graph", name))?;

    if direct {
        if node.dependents.is_empty() {
            println!("nothing depends on {}", name.bold());
        } else {
            println!("{} is depended on by ({} direct):", name.bold(), node.dependents.len());
            for dep in &node.dependents {
                println!("  {} {}", "←".yellow(), dep);
            }
        }
    } else {
        let rdeps = graph.transitive_rdeps(name);
        if rdeps.is_empty() {
            println!("nothing depends on {}", name.bold());
        } else {
            println!("{} is depended on by ({} transitive, {} direct):", name.bold(), rdeps.len(), node.dependents.len());
            println!("  {} changing {} may impact these repos:", "⚠".yellow(), name.bold());
            for dep in &rdeps {
                let marker = if node.dependents.contains(dep) { "←" } else { "⤷" };
                let label = if node.dependents.contains(dep) {
                    "".to_string()
                } else {
                    " (transitive)".dimmed().to_string()
                };
                println!("  {} {}{}", marker.yellow(), dep, label);
            }
        }
    }

    Ok(())
}

fn cmd_graph_path(from: &str, to: &str, graph_path: Option<&Path>, root: &Path) -> Result<()> {
    let graph = load_or_assemble_graph(graph_path, root)?;

    // Verify both nodes exist.
    graph.find_node(from)
        .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in graph", from))?;
    graph.find_node(to)
        .ok_or_else(|| anyhow::anyhow!("repo '{}' not found in graph", to))?;

    match graph.find_path(from, to) {
        Some(path) => {
            println!("{}", "dependency path:".cyan().bold());
            for (i, node) in path.iter().enumerate() {
                if i > 0 {
                    println!("    {} depends on", "↓".green());
                }
                let info = graph.find_node(node);
                let desc = info.map(|n| format!(" ({})", n.description)).unwrap_or_default();
                println!("  {}{}", node.bold(), desc.dimmed());
            }
            println!("\n  {} {} hops", "·".dimmed(), path.len() - 1);
        }
        None => {
            println!("no dependency path from {} to {}", from.bold(), to.bold());

            // Try reverse direction.
            if let Some(rev) = graph.find_path(to, from) {
                println!(
                    "\n  {} a reverse path exists ({} {} {})",
                    "hint:".yellow(),
                    to, "→".green(), from,
                );
                println!("  {} try: archon graph path {} {}", "·".dimmed(), to, from);
                let _ = rev; // suppress unused warning
            }
        }
    }

    Ok(())
}

/// Default rules for Rust projects.
fn default_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "tests-pass".into(),
            run: "cargo test --all".into(),
            description: Some("All tests pass across the workspace".into()),
        },
        Rule {
            id: "no-clippy-warnings".into(),
            run: "cargo clippy --all -- -D warnings".into(),
            description: Some("No clippy warnings or errors".into()),
        },
        Rule {
            id: "builds-clean".into(),
            run: "cargo build --all".into(),
            description: Some("All crates build successfully".into()),
        },
    ]
}

// --- Helpers ---

fn inject_context(repo_path: &Path, context: &str) -> Result<bool> {
    // Write the full context to .archon/context.md
    let archon_dir = repo_path.join(".archon");
    std::fs::create_dir_all(&archon_dir)?;
    std::fs::write(archon_dir.join("context.md"), context)?;

    // In CLAUDE.md and AGENTS.md, replace managed section with a reference.
    let reference = "## Ecosystem Context (auto-generated by archon)\n\n\
                     See [`.archon/context.md`](.archon/context.md) for full dependency graph, \
                     public API surface, and contract details for this repo.";

    let mut injected = false;
    for filename in ["CLAUDE.md", "AGENTS.md"] {
        let path = repo_path.join(filename);
        if !path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        if !has_managed_sections(&content) {
            continue;
        }
        if let Some(updated) = replace_managed_section(&content, reference) {
            std::fs::write(&path, updated)?;
            injected = true;
        }
    }

    // If no file had markers, create or append to CLAUDE.md with markers.
    if !injected {
        let claude_path = repo_path.join("CLAUDE.md");
        let markers = empty_section_markers();
        let begin = markers.lines().next().unwrap();
        let end = markers.lines().last().unwrap();
        let section = format!("{begin}\n{reference}\n{end}\n");

        let content = if claude_path.exists() {
            let existing = std::fs::read_to_string(&claude_path)?;
            format!("{}\n\n{section}", existing.trim_end())
        } else {
            format!("# CLAUDE.md\n\n{section}")
        };
        std::fs::write(&claude_path, content)?;
        injected = true;
    }

    Ok(injected)
}

fn find_sibling_registry(path: &Path) -> Option<&Path> {
    // Look for archon-registry as a sibling directory.
    let parent = path.parent()?;
    let reg = parent.join("archon-registry");
    if reg.exists() {
        // We can't return a reference to a local, so return None
        // and let the caller handle it.
        None
    } else {
        None
    }
}

fn dir_name(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my-project".into())
}

fn extract_cargo_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(field) {
            let rest = trimmed[field.len()..].trim();
            if rest.starts_with('=') {
                let val = rest[1..].trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn extract_cargo_name(content: &str) -> Option<String> {
    extract_cargo_field(content, "name")
}

fn extract_cargo_description(content: &str) -> Option<String> {
    extract_cargo_field(content, "description")
}

fn detect_workspace_crates(path: &Path) -> Option<Vec<CrateEntry>> {
    let cargo_path = path.join("Cargo.toml");
    if !cargo_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&cargo_path).ok()?;

    // Check if it's a workspace.
    if !content.contains("[workspace]") {
        // Single crate — use the crate name.
        let name = extract_cargo_name(&content)?;
        return Some(vec![CrateEntry {
            name,
            public: true,
        }]);
    }

    // Parse workspace members — handles both inline and multi-line arrays.
    let raw_members = parse_toml_string_array(&content, "members")?;

    let mut crates = Vec::new();
    for member in &raw_members {
        if member.contains('*') {
            // Glob pattern like "crates/*" — expand by reading the directory.
            let pattern_dir = member.trim_end_matches("/*").trim_end_matches("\\*");
            let dir = path.join(pattern_dir);
            if dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() && p.join("Cargo.toml").exists() {
                            let crate_name = p.join("Cargo.toml");
                            let name = std::fs::read_to_string(&crate_name)
                                .ok()
                                .and_then(|c| extract_cargo_name(&c))
                                .unwrap_or_else(|| {
                                    entry.file_name().to_string_lossy().to_string()
                                });
                            crates.push(CrateEntry {
                                name,
                                public: true,
                            });
                        }
                    }
                }
            }
        } else {
            // Literal path like "crates/my-crate".
            let crate_dir = path.join(member);
            let name = if crate_dir.join("Cargo.toml").exists() {
                std::fs::read_to_string(crate_dir.join("Cargo.toml"))
                    .ok()
                    .and_then(|c| extract_cargo_name(&c))
                    .unwrap_or_else(|| member.split('/').next_back().unwrap_or(member).to_string())
            } else {
                member.split('/').next_back().unwrap_or(member).to_string()
            };
            crates.push(CrateEntry {
                name,
                public: true,
            });
        }
    }

    crates.sort_by(|a, b| a.name.cmp(&b.name));

    if crates.is_empty() {
        None
    } else {
        Some(crates)
    }
}

/// Parse a TOML string array value, handling both inline `key = ["a", "b"]`
/// and multi-line formats.
fn parse_toml_string_array(content: &str, key: &str) -> Option<Vec<String>> {
    // Find the line starting with `key`.
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            continue;
        }
        let after_key = trimmed[key.len()..].trim();
        if !after_key.starts_with('=') {
            continue;
        }
        let value_part = after_key[1..].trim();

        // Inline array: members = ["crates/*", "other"]
        if value_part.starts_with('[') {
            let full = if value_part.contains(']') {
                value_part.to_string()
            } else {
                // Multi-line array that starts on the same line as the key.
                let mut buf = value_part.to_string();
                for next_line in lines.by_ref() {
                    buf.push_str(next_line.trim());
                    if next_line.contains(']') {
                        break;
                    }
                }
                buf
            };
            // Extract quoted strings from the array.
            let items: Vec<String> = full
                .split('"')
                .enumerate()
                .filter(|(i, _)| i % 2 == 1) // odd indices are inside quotes
                .map(|(_, s)| s.to_string())
                .collect();
            return Some(items);
        }
    }
    None
}

/// Scan workspace crates for @contract() annotations and return discovered contract names.
fn discover_contracts(path: &Path) -> Vec<String> {
    let crate_entries = detect_workspace_crates(path).unwrap_or_default();
    let mut contracts = Vec::new();

    for entry in &crate_entries {
        if let Some(src_dir) = find_crate_src(path, &entry.name) {
            if let Ok(surface) = extract_crate_surface_with_contracts(&entry.name, &src_dir) {
                for binding in &surface.contracts {
                    if !contracts.contains(&binding.contract) {
                        contracts.push(binding.contract.clone());
                    }
                }
            }
        }
    }

    contracts.sort();
    contracts
}

/// Scan `root` for directories that have a Cargo.toml but no archon.yaml,
/// and generate a minimal manifest for each one.
fn bootstrap_siblings(root: &Path, no_ai: bool) -> Result<()> {
    println!(
        "{} Bootstrapping repos in {}...",
        "bootstrap:".cyan().bold(),
        root.display()
    );

    // Phase 1: Collect repos that need manifests.
    struct BootstrapRepo {
        path: PathBuf,
        dir: String,
        name: String,
        description: String,
        cargo_content: String,
        crates: Option<Vec<CrateEntry>>,
    }

    let mut repos = Vec::new();
    for entry in std::fs::read_dir(root).with_context(|| format!("reading {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.join("archon.yaml").exists() {
            continue;
        }
        let cargo_path = path.join("Cargo.toml");
        if !cargo_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&cargo_path)?;
        let name = extract_cargo_name(&content).unwrap_or_else(|| dir_name(&path));
        let description = extract_cargo_description(&content)
            .unwrap_or_else(|| format!("Auto-bootstrapped from {}", dir_name(&path)));
        let crates = detect_workspace_crates(&path);

        repos.push(BootstrapRepo {
            dir: dir_name(&path),
            path,
            name,
            description,
            cargo_content: content,
            crates,
        });
    }

    if repos.is_empty() {
        println!(
            "  {} All sibling repos already have archon.yaml",
            "·".dimmed()
        );
        println!();
        return Ok(());
    }

    // Phase 2: AI inference for roles and dependencies.
    let use_ai = !no_ai && has_claude_cli();
    let ai_results = if use_ai {
        println!(
            "  {} Analyzing {} repos with AI...",
            "🤖".to_string().cyan(),
            repos.len()
        );
        let entries: Vec<(String, String)> = repos
            .iter()
            .map(|r| (r.name.clone(), r.cargo_content.clone()))
            .collect();
        ai_suggest_bootstrap(&entries)
    } else {
        vec![]
    };

    // Phase 3: Create manifests.
    for repo in &repos {
        let ai_entry = ai_results.iter().find(|e| e.name == repo.name);

        let role = if let Some(ai) = ai_entry {
            ai.role.clone()
        } else {
            // Heuristic fallback.
            let dir = &repo.dir;
            if dir.contains("sdk") || dir.contains("lib") {
                Role::Library
            } else if dir.contains("cli") || dir.contains("tool") {
                Role::Tool
            } else if dir.contains("service") || dir.contains("server") || dir.contains("api") {
                Role::Service
            } else {
                Role::Library
            }
        };

        let depends_on = ai_entry
            .map(|ai| ai.depends_on.clone())
            .unwrap_or_default();

        let manifest = Manifest {
            name: repo.name.clone(),
            description: repo.description.clone(),
            role: role.clone(),
            depends_on: depends_on.clone(),
            provides: vec![],
            crates: repo.crates.clone(),
            auto_update: None,
            registry: None,
            rules: default_rules(),
        };

        manifest.save(&repo.path)?;

        let deps_str = if depends_on.is_empty() {
            String::new()
        } else {
            format!(" deps=[{}]", depends_on.join(", "))
        };
        let ai_tag = if ai_entry.is_some() { " (AI)" } else { "" };
        println!(
            "  {} {} → {} ({}{}){}",
            "created:".green().bold(),
            repo.dir,
            repo.name.bold(),
            role,
            ai_tag.dimmed(),
            deps_str.dimmed(),
        );
    }

    println!(
        "{} Bootstrapped {} repo(s){}",
        "ok:".green().bold(),
        repos.len(),
        if use_ai { " with AI inference" } else { "" },
    );
    println!();

    Ok(())
}

fn find_crate_src(repo_root: &Path, crate_name: &str) -> Option<PathBuf> {
    // Try common locations.
    let candidates = [
        repo_root.join("src"),
        repo_root.join(format!("crates/{}/src", crate_name)),
        repo_root.join(format!("crates/{crate_name}/src")),
    ];

    for candidate in &candidates {
        if candidate.exists()
            && (candidate.join("lib.rs").exists() || candidate.join("main.rs").exists())
        {
            return Some(candidate.clone());
        }
    }

    // Walk subdirectories looking for a matching Cargo.toml name.
    // This handles workspaces where directory names differ from crate names
    // (e.g., directory "spec/" contains crate "uor-ontology").
    for search_dir in [repo_root.to_path_buf(), repo_root.join("crates")] {
        if !search_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let cargo = entry.path().join("Cargo.toml");
                if cargo.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cargo) {
                        if extract_cargo_name(&content).as_deref() == Some(crate_name) {
                            let src = entry.path().join("src");
                            if src.exists() {
                                return Some(src);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
