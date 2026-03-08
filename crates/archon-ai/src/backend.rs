use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::json;

use crate::auth::{self, Credentials};

/// Which AI backend to use for automated content generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// ANTHROPIC_API_KEY environment variable
    AnthropicApi,
    /// `claude` CLI binary in PATH
    ClaudeCli,
    /// Stored OAuth credentials for Claude (`.archon/credentials.json`)
    StoredClaude,
    /// Stored OAuth credentials for OpenAI (`.archon/credentials.json`)
    OpenAI,
}

/// Resolve which backend to use.
///
/// Priority for `"auto"` / unknown provider:
///   `ANTHROPIC_API_KEY` → `claude` CLI → stored claude creds → stored openai creds
///
/// For named providers (`"claude"`, `"openai"`, `"api"`, `"cli"`), the matching
/// backend is returned or an error if it is unavailable.
pub fn check(provider: &str) -> Result<Backend> {
    match provider {
        "api" => {
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                Ok(Backend::AnthropicApi)
            } else {
                bail!("provider=\"api\" requires ANTHROPIC_API_KEY to be set")
            }
        }
        "cli" => {
            if which_claude() {
                Ok(Backend::ClaudeCli)
            } else {
                bail!("provider=\"cli\" requires 'claude' in PATH")
            }
        }
        "openai" => match auth::load("openai") {
            Some(c) if !c.is_expired() => Ok(Backend::OpenAI),
            Some(c) => try_auto_refresh(c).map(|_| Backend::OpenAI),
            None => {
                bail!("no OpenAI credentials found.\n  Run: archon auth login --provider openai")
            }
        },
        _ => {
            // "claude", "auto", or unrecognised — prefer automated, fall back to stored creds
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                return Ok(Backend::AnthropicApi);
            }
            if which_claude() {
                return Ok(Backend::ClaudeCli);
            }
            if let Some(c) = auth::load("claude") {
                if !c.is_expired() {
                    return Ok(Backend::StoredClaude);
                }
                if try_auto_refresh(c).is_ok() {
                    return Ok(Backend::StoredClaude);
                }
            }
            if let Some(c) = auth::load("openai") {
                if !c.is_expired() {
                    return Ok(Backend::OpenAI);
                }
                if try_auto_refresh(c).is_ok() {
                    return Ok(Backend::OpenAI);
                }
            }
            bail!(
                "no AI credentials found.\n  \
                 Run: archon auth login\n  \
                 Or set ANTHROPIC_API_KEY."
            )
        }
    }
}

/// Resolve which backend to use (auto-detect only).
///
/// Priority: `ANTHROPIC_API_KEY` → `claude` CLI → stored creds
pub fn detect() -> Result<Backend> {
    check("auto")
}

/// Check if any backend is available without error.
pub fn is_available() -> bool {
    detect().is_ok()
}

/// Attempt to refresh an expired token. Returns `Ok(())` on success.
fn try_auto_refresh(creds: Credentials) -> Result<()> {
    if creds.refresh_token.is_none() {
        bail!(
            "{} credentials have expired and no refresh token is stored.\n  \
             Run: archon auth login --provider {}",
            creds.provider,
            creds.provider,
        );
    }
    eprintln!(
        "  {} {} token expired — refreshing...",
        "→".blue(),
        creds.provider,
    );
    let refreshed = auth::refresh(&creds)?;
    auth::save(&refreshed)?;
    eprintln!(
        "  {} {} token refreshed.",
        "✓".green().bold(),
        refreshed.provider,
    );
    Ok(())
}

/// Dispatch a prompt to the chosen backend, return the model's response text.
pub fn call(prompt: &str, model: &str, backend: Backend) -> Result<String> {
    match backend {
        Backend::AnthropicApi => call_anthropic_env(prompt, model),
        Backend::ClaudeCli => call_claude_cli(prompt),
        Backend::StoredClaude => {
            let creds = auth::load("claude").context("stored claude credentials not found")?;
            call_anthropic_key(prompt, model, &creds.token)
        }
        Backend::OpenAI => {
            let creds = auth::load("openai")
                .filter(|c| !c.is_expired())
                .context("stored OpenAI credentials not found or expired")?;
            call_openai(prompt, &creds.token)
        }
    }
}

fn call_anthropic_env(prompt: &str, model: &str) -> Result<String> {
    let key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;
    call_anthropic_key(prompt, model, &key)
}

fn call_anthropic_key(prompt: &str, model: &str, key: &str) -> Result<String> {
    let body = json!({
        "model": model,
        "max_tokens": 16384,
        "messages": [{"role": "user", "content": prompt}]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .context("calling Anthropic API")?;

    if !resp.status().is_success() {
        bail!("Anthropic API error {}: {}", resp.status(), resp.text()?);
    }

    let json: serde_json::Value = resp.json().context("parsing API response")?;
    json["content"][0]["text"]
        .as_str()
        .map(ToOwned::to_owned)
        .context("unexpected Anthropic response shape")
}

fn call_openai(prompt: &str, token: &str) -> Result<String> {
    let body = json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": prompt}]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(token)
        .json(&body)
        .send()
        .context("calling OpenAI API")?;

    if !resp.status().is_success() {
        bail!("OpenAI API error {}: {}", resp.status(), resp.text()?);
    }

    let json: serde_json::Value = resp.json().context("parsing OpenAI response")?;
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(ToOwned::to_owned)
        .context("unexpected OpenAI response shape")
}

fn call_claude_cli(prompt: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    let mut child = Command::new("claude")
        .args(["--print"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawning claude CLI")?;

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(prompt.as_bytes())
            .context("writing to claude stdin")?;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(Duration::from_secs(300)) {
        Ok(Ok(output)) => {
            if !output.status.success() {
                bail!("claude CLI exited with {}", output.status);
            }
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Err(e)) => Err(e).context("waiting for claude"),
        Err(_) => bail!("claude CLI timed out after 300 seconds"),
    }
}

fn which_claude() -> bool {
    std::process::Command::new("which")
        .arg("claude")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
