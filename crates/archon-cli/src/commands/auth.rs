use anyhow::{bail, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use archon_ai::auth;

use crate::app::AuthCommands;

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("  {spinner:.magenta} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
}

pub fn run(cmd: AuthCommands) -> Result<()> {
    match cmd {
        AuthCommands::Login { provider, port } => login(provider.as_deref(), port),
        AuthCommands::Refresh { provider } => refresh_token(provider.as_deref()),
        AuthCommands::Status => status(),
        AuthCommands::Logout { provider } => logout(provider.as_deref()),
    }
}

fn login(provider: Option<&str>, port: Option<u16>) -> Result<()> {
    let provider = match provider {
        Some(p) => p.to_owned(),
        None => inquire::Select::new(
            "Which AI provider?",
            vec!["claude  (browser OAuth)", "openai  (browser OAuth)"],
        )
        .prompt()?
        .split_whitespace()
        .next()
        .unwrap_or("claude")
        .to_owned(),
    };

    let creds = match provider.as_str() {
        "claude" => auth::login_claude(port)?,
        "openai" => auth::login_openai(port)?,
        other => bail!("unknown provider '{other}' — use 'claude' or 'openai'"),
    };

    auth::save(&creds)?;
    println!();
    println!(
        "  {} Credentials saved (.archon/credentials.json)",
        "✓".green().bold()
    );
    println!();
    Ok(())
}

fn refresh_token(provider: Option<&str>) -> Result<()> {
    let provider = match provider {
        Some(p) => p.to_owned(),
        None => {
            let all = auth::all();
            let choices: Vec<String> = all.keys().cloned().collect();
            if choices.is_empty() {
                bail!("no credentials stored — run: archon auth login");
            }
            inquire::Select::new("Which provider to refresh?", choices).prompt()?
        }
    };

    let creds = auth::load(&provider)
        .ok_or_else(|| anyhow::anyhow!("no stored credentials for '{provider}'"))?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(spinner_style());
    pb.set_message(format!("Refreshing {provider} token..."));
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    let refreshed = auth::refresh(&creds)?;
    auth::save(&refreshed)?;

    pb.finish_with_message(format!("{} Token refreshed and saved.", "✓".green().bold()));
    println!();
    Ok(())
}

fn status() -> Result<()> {
    let all = auth::all();
    println!();
    if all.is_empty() {
        println!("  No credentials stored.");
        println!("  Run: archon auth login");
    } else {
        println!("  Stored credentials:");
        for (provider, creds) in &all {
            let note = if creds.is_expired() {
                if creds.refresh_token.is_some() {
                    "(expired — run: archon auth refresh)".red().to_string()
                } else {
                    "(expired — run: archon auth login)".red().to_string()
                }
            } else if creds.expires_at.is_some() {
                "(OAuth token)".green().to_string()
            } else {
                "(API key)".green().to_string()
            };
            println!("    {} {provider}  {note}", "✓".green().bold());
        }
    }
    println!();
    Ok(())
}

fn logout(provider: Option<&str>) -> Result<()> {
    let provider = match provider {
        Some(p) => p.to_owned(),
        None => {
            let all = auth::all();
            let choices: Vec<String> = all.keys().cloned().collect();
            if choices.is_empty() {
                println!("  No credentials stored.");
                return Ok(());
            }
            inquire::Select::new("Which provider to log out?", choices).prompt()?
        }
    };

    auth::remove(&provider)?;
    println!("  {} Logged out of {provider}.", "✓".green().bold());
    Ok(())
}
