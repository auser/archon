mod app;
mod commands;
mod output;

use clap::Parser;
use colored::Colorize;

fn main() {
    let cli = app::Cli::parse();

    let result = match cli.command {
        app::Commands::Init(args) => commands::init::run(args),
        app::Commands::Verify(args) => commands::verify::run(args),
        app::Commands::Status(args) => commands::status::run(args),
        app::Commands::Sync(args) => commands::sync::run(args),
        app::Commands::Adr(cmd) => commands::adr::run(cmd),
        app::Commands::Exception(cmd) => commands::exception::run(cmd),
        app::Commands::Bootstrap(args) => commands::bootstrap::run(args),
        app::Commands::Decide(args) => commands::decide::run(args),
        app::Commands::Auth(cmd) => commands::auth::run(cmd),
        app::Commands::Generate(args) => commands::generate::run(args),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}
