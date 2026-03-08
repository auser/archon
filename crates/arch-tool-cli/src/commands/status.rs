use anyhow::{Context, Result};
use arch_tool_core::paths;
use arch_tool_verify::runner::run_verify;

use crate::app::{OutputFormat, StatusArgs};

pub fn run(args: StatusArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    let arch_root = paths::resolve_arch_root(None)?;

    let report = run_verify(&cwd, arch_root.as_deref())?;

    match args.format {
        OutputFormat::Text => report.print(),
        OutputFormat::Json => println!("{}", report.to_json()),
    }

    Ok(())
}
