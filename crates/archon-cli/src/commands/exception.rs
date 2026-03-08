use anyhow::{Context, Result};
use colored::Colorize;

use archon_adr::exception;

use crate::app::ExceptionCommands;

pub fn run(cmd: ExceptionCommands) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    match cmd {
        ExceptionCommands::New {
            rule,
            reason,
            expires,
        } => {
            let id = exception::add_exception(
                &cwd,
                &rule,
                &reason,
                expires.as_deref(),
            )?;
            eprintln!("\n  {} exception {}", "created".green(), id);
            eprintln!("    rule:    {rule}");
            eprintln!("    reason:  {reason}");
            if let Some(exp) = &expires {
                eprintln!("    expires: {exp}");
            }
            eprintln!();
        }
        ExceptionCommands::List => {
            let exceptions = exception::list_exceptions(&cwd)?;
            if exceptions.is_empty() {
                eprintln!("\n  No exceptions declared.\n");
            } else {
                eprintln!("\n{}", "Exceptions:".bold());
                for exc in &exceptions {
                    let expires = exc
                        .expires
                        .as_deref()
                        .unwrap_or("never");
                    eprintln!(
                        "  {}  rule: {}  expires: {}",
                        exc.id, exc.rule, expires
                    );
                }
                eprintln!();
            }
        }
    }

    Ok(())
}
