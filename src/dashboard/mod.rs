pub mod app;
mod event_handler;
pub mod ui;
pub mod web;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::graph::{collect_manifests, Graph};
use crate::manifest::Manifest;

/// Entry point for the dashboard command.
pub fn run_dashboard(root: &Path, registry: &Path, web_mode: bool) -> Result<()> {
    let graph = load_or_assemble(root, registry)?;
    let manifests = collect_manifests(root).unwrap_or_default();

    if web_mode {
        let output_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("archon-dashboard.html");
        web::generate_web_dashboard(&graph, &output_path)?;
        println!("Wrote {}", output_path.display());
        return Ok(());
    }

    run_tui(graph, manifests, root.to_path_buf(), registry.to_path_buf())
}

fn load_or_assemble(root: &Path, registry: &Path) -> Result<Graph> {
    let graph_path = registry.join("graph.yaml");
    if graph_path.exists() {
        Graph::load(&graph_path)
    } else {
        let manifest_pairs = collect_manifests(root)?;
        let manifests: Vec<Manifest> = manifest_pairs.into_iter().map(|(_, m)| m).collect();
        if manifests.is_empty() {
            Ok(Graph {
                generated_at: chrono::Utc::now().to_rfc3339(),
                nodes: vec![],
            })
        } else {
            Ok(Graph::assemble(manifests))
        }
    }
}

fn run_tui(
    graph: Graph,
    manifests: Vec<(PathBuf, Manifest)>,
    root: PathBuf,
    registry: PathBuf,
) -> Result<()> {
    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode().context("enabling raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).context("entering alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("creating terminal")?;

    let mut app = app::App::new(graph, manifests, root, registry);

    loop {
        terminal
            .draw(|f| ui::draw(f, &mut app))
            .context("drawing frame")?;

        if let Event::Key(key) = event::read().context("reading event")? {
            event_handler::handle_key(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode().context("disabling raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("leaving alternate screen")?;
    terminal.show_cursor().context("showing cursor")?;

    Ok(())
}
