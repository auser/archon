use anyhow::{Context, Result};
use colored::Colorize;

use arch_tool_adr::create;
use arch_tool_adr::model::AdrStatus;

use crate::app::AdrCommands;

pub fn run(cmd: AdrCommands) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    let adr_dir = cwd.join("specs/adrs");

    match cmd {
        AdrCommands::New { title, status } => {
            let status = match status.as_deref() {
                Some("accepted") => AdrStatus::Accepted,
                Some("deprecated") => AdrStatus::Deprecated,
                Some("superseded") => AdrStatus::Superseded,
                _ => AdrStatus::Proposed,
            };

            let filename = create::create_adr(&adr_dir, &title, status)?;
            eprintln!(
                "\n  {} specs/adrs/{}",
                "created".green(),
                filename
            );
            eprintln!();
        }
        AdrCommands::List => {
            let adrs = create::list_adrs(&adr_dir)?;
            if adrs.is_empty() {
                eprintln!("\n  No ADRs found in specs/adrs/\n");
            } else {
                eprintln!("\n{}", "ADRs:".bold());
                for adr in &adrs {
                    eprintln!("  {:04}  {}", adr.number, adr.title);
                }
                eprintln!();
            }
        }
    }

    Ok(())
}
