use anyhow::{Context, Result};
use colored::Colorize;

use archon_ai::backend;
use archon_ai::merge;
use archon_core::paths;
use archon_sync::engine::{self, AiMergeFn, SyncOptions};

use crate::app::SyncArgs;

pub fn run(args: SyncArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    let arch_root = paths::resolve_arch_root(args.arch_root.as_deref())?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "architecture repo not found. Use --arch-root or set ARCHON_ROOT."
            )
        })?;

    eprintln!(
        "\n{} {}",
        "archon sync:".bold(),
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .cyan()
    );
    eprintln!(
        "  arch root: {}",
        arch_root.display().to_string().dimmed()
    );

    if args.dry_run {
        eprintln!("  {}", "(dry run)".yellow());
    }

    // Detect AI backend for merge support
    let ai_merge: Option<AiMergeFn> = match backend::detect() {
        Ok(ab) => {
            eprintln!("  {} AI merge available", "✓".green());
            let model = "claude-sonnet-4-5-20250514".to_string();
            Some(Box::new(move |filename: &str, arch_content: &str, local_content: &str| {
                merge::merge_docs(filename, arch_content, local_content, ab, &model)
            }))
        }
        Err(_) => None,
    };

    eprintln!();

    let options = SyncOptions {
        dry_run: args.dry_run,
        force: args.force,
        ai_merge,
    };

    let results = engine::run_sync(&cwd, &arch_root, &options)?;
    engine::print_results(&results);

    if args.dry_run {
        eprintln!("\n  (dry run — no files written)");
    }

    eprintln!();
    Ok(())
}
