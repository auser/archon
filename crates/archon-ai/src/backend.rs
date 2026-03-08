use anyhow::{bail, Context, Result};
use serde_json::json;

/// Which AI backend to use for automated content generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// ANTHROPIC_API_KEY environment variable
    AnthropicApi,
    /// `claude` CLI binary in PATH
    ClaudeCli,
}

/// Resolve which backend to use.
///
/// Priority: `ANTHROPIC_API_KEY` env var → `claude` CLI in PATH
pub fn detect() -> Result<Backend> {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        return Ok(Backend::AnthropicApi);
    }
    if which_claude() {
        return Ok(Backend::ClaudeCli);
    }
    bail!(
        "no AI backend found.\n  \
         Set ANTHROPIC_API_KEY or install the claude CLI."
    )
}

/// Check if a specific backend is available without error.
pub fn is_available() -> bool {
    detect().is_ok()
}

/// Dispatch a prompt to the chosen backend, return the model's response text.
pub fn call(prompt: &str, model: &str, backend: Backend) -> Result<String> {
    match backend {
        Backend::AnthropicApi => call_anthropic(prompt, model),
        Backend::ClaudeCli => call_claude_cli(prompt),
    }
}

fn call_anthropic(prompt: &str, model: &str) -> Result<String> {
    let key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;

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
        .header("x-api-key", &key)
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
