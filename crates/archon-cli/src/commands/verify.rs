use anyhow::{Context, Result};
use archon_core::paths;
use archon_verify::runner::run_verify;

use crate::app::{OutputFormat, VerifyArgs};

pub fn run(args: VerifyArgs) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    let arch_root = paths::resolve_arch_root(args.arch_root.as_deref())?;

    let report = run_verify(&cwd, arch_root.as_deref())?;

    match args.format {
        OutputFormat::Text => report.print(),
        OutputFormat::Json => println!("{}", report.to_json()),
    }

    let exit_code = report.exit_code(args.strict);
    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
