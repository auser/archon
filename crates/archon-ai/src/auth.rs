use anyhow::{bail, Context, Result};
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write as IoWrite},
    net::TcpListener,
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

// ─── Claude / Anthropic OAuth (same client_id as Claude Code CLI) ─────────────
const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const CLAUDE_AUTH_URL: &str = "https://claude.ai/oauth/authorize";
const CLAUDE_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const CLAUDE_PORT: u16 = 54321;

// ─── OpenAI OAuth (openai-codex public client_id) ─────────────────────────────
const OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_PORT: u16 = 1455;

const CALLBACK_TIMEOUT_SECS: u64 = 180;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Credentials {
    pub provider: String,
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

impl Credentials {
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            exp <= now + 60
        })
    }
}

// ─── Storage ──────────────────────────────────────────────────────────────────

type CredMap = HashMap<String, Credentials>;

/// Find the nearest ancestor directory containing `hologram.repo.yaml`, or CWD.
fn project_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join("hologram.repo.yaml").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Credentials live in `<project_root>/.archon/credentials.json`.
fn creds_path() -> PathBuf {
    project_root().join(".archon").join("credentials.json")
}

pub fn load(provider: &str) -> Option<Credentials> {
    let text = std::fs::read_to_string(creds_path()).ok()?;
    let map: CredMap = serde_json::from_str(&text).ok()?;
    map.get(provider).cloned()
}

pub fn save(creds: &Credentials) -> Result<()> {
    let path = creds_path();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut map: CredMap = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    map.insert(creds.provider.clone(), creds.clone());
    let json = serde_json::to_string_pretty(&map).context("serializing credentials")?;
    std::fs::write(&path, json).context("writing credentials")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    // Ensure .archon/ is gitignored in the project root.
    archon_core::paths::ensure_gitignore_entry(&project_root(), ".archon/")?;
    Ok(())
}

pub fn remove(provider: &str) -> Result<()> {
    let path = creds_path();
    let mut map: CredMap = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    map.remove(provider);
    let json = serde_json::to_string_pretty(&map).context("serializing credentials")?;
    std::fs::write(&path, json).context("writing credentials")?;
    Ok(())
}

pub fn all() -> CredMap {
    std::fs::read_to_string(creds_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

// ─── Claude / Anthropic: PKCE OAuth via claude.ai subscription ───────────────

pub fn login_claude(port: Option<u16>) -> Result<Credentials> {
    let custom = port.unwrap_or(CLAUDE_PORT);
    match pkce_oauth_flow(
        "claude",
        "Anthropic / Claude",
        CLAUDE_AUTH_URL,
        CLAUDE_TOKEN_URL,
        CLAUDE_CLIENT_ID,
        custom,
        "user:inference user:profile",
    ) {
        Ok(creds) => Ok(creds),
        Err(e) if port.is_some() && custom != CLAUDE_PORT => {
            eprintln!(
                "  ⚠ port {custom} failed ({e:#}), retrying on default port {CLAUDE_PORT}..."
            );
            pkce_oauth_flow(
                "claude",
                "Anthropic / Claude",
                CLAUDE_AUTH_URL,
                CLAUDE_TOKEN_URL,
                CLAUDE_CLIENT_ID,
                CLAUDE_PORT,
                "user:inference user:profile",
            )
        }
        Err(e) => Err(e),
    }
}

// ─── OpenAI: PKCE OAuth with loopback callback ────────────────────────────────

pub fn login_openai(port: Option<u16>) -> Result<Credentials> {
    let custom = port.unwrap_or(OPENAI_PORT);
    match pkce_oauth_flow(
        "openai",
        "OpenAI",
        OPENAI_AUTH_URL,
        OPENAI_TOKEN_URL,
        OPENAI_CLIENT_ID,
        custom,
        "openid profile email offline_access",
    ) {
        Ok(creds) => Ok(creds),
        Err(e) if port.is_some() && custom != OPENAI_PORT => {
            eprintln!(
                "  ⚠ port {custom} failed ({e:#}), retrying on default port {OPENAI_PORT}..."
            );
            pkce_oauth_flow(
                "openai",
                "OpenAI",
                OPENAI_AUTH_URL,
                OPENAI_TOKEN_URL,
                OPENAI_CLIENT_ID,
                OPENAI_PORT,
                "openid profile email offline_access",
            )
        }
        Err(e) => Err(e),
    }
}

/// Generic PKCE OAuth flow: open browser → capture loopback callback → exchange code.
fn pkce_oauth_flow(
    provider: &str,
    display_name: &str,
    auth_url: &str,
    token_url: &str,
    client_id: &str,
    port: u16,
    scope: &str,
) -> Result<Credentials> {
    let (verifier, challenge) = pkce_pair();
    let state = random_state();
    let redirect_uri = format!("http://localhost:{port}/callback");
    let scope_enc = scope.replace(' ', "%20");

    let full_auth_url = format!(
        "{auth_url}\
         ?response_type=code\
         &client_id={client_id}\
         &redirect_uri={}\
         &scope={scope_enc}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &state={state}",
        percent_encode(&redirect_uri),
    );

    println!();
    println!("  {} {display_name} — OAuth authorization", "→".blue());
    println!("  Opening authorization page in your browser...");
    println!("  (waiting up to {CALLBACK_TIMEOUT_SECS}s for the callback)");
    println!();

    // Bind the listener BEFORE opening the browser so we don't miss the callback.
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .with_context(|| format!("could not bind port {port} — is another instance running?"))?;

    let (tx, rx) = mpsc::channel::<Result<(String, String)>>();
    let state_clone = state.clone();
    std::thread::spawn(move || {
        tx.send(accept_callback(&listener, &state_clone)).ok();
    });

    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&full_auth_url)
        .spawn()
        .ok();

    let (code, returned_state) = match rx.recv_timeout(Duration::from_secs(CALLBACK_TIMEOUT_SECS)) {
        Ok(Ok(pair)) => pair,
        Ok(Err(e)) => return Err(e),
        Err(_) => bail!("OAuth timed out after {CALLBACK_TIMEOUT_SECS} seconds"),
    };

    println!("  Exchanging authorization code...");
    let (token, refresh_token, expires_at) = exchange_code(
        token_url,
        client_id,
        &code,
        &verifier,
        &redirect_uri,
        &returned_state,
    )?;

    println!("  {} Logged in to {display_name}.", "✓".green().bold());

    Ok(Credentials {
        provider: provider.to_owned(),
        token,
        refresh_token,
        expires_at,
    })
}

/// Block until browser hits the callback URL, validate state, return (code, state).
fn accept_callback(listener: &TcpListener, expected_state: &str) -> Result<(String, String)> {
    let (mut stream, _) = listener.accept().context("accepting OAuth callback")?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).context("reading callback request")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // First line of HTTP request: "GET /callback?code=...&state=... HTTP/1.1"
    let first_line = request.lines().next().unwrap_or("");
    let query = first_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| path.split_once('?').map(|(_, q)| q))
        .unwrap_or("");

    let code = query_param(query, "code");
    let state = query_param(query, "state");
    let ok = code.is_some() && state.as_deref() == Some(expected_state);

    let html = if ok {
        "<html><body style='font-family:sans-serif;padding:2em'>\
         <h2>Authorization successful</h2>\
         <p>You can close this tab and return to the terminal.</p>\
         </body></html>"
    } else {
        "<html><body style='font-family:sans-serif;padding:2em'>\
         <h2>Authorization failed</h2>\
         <p>State mismatch or missing code. Please try again.</p>\
         </body></html>"
    };

    let status = if ok { "200 OK" } else { "400 Bad Request" };
    let response =
        format!("HTTP/1.1 {status}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{html}");
    stream.write_all(response.as_bytes()).ok();

    if ok {
        Ok((code.unwrap(), state.unwrap()))
    } else {
        bail!("OAuth callback: state mismatch or missing code")
    }
}

fn exchange_code(
    token_url: &str,
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
    state: &str,
) -> Result<(String, Option<String>, Option<u64>)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Claude (platform.claude.com) requires JSON body with state.
    // OpenAI uses application/x-www-form-urlencoded without state.
    let resp = if token_url.contains("claude.com") {
        client
            .post(token_url)
            .json(&serde_json::json!({
                "grant_type":    "authorization_code",
                "client_id":     client_id,
                "code":          code,
                "redirect_uri":  redirect_uri,
                "code_verifier": verifier,
                "state":         state,
            }))
            .send()
            .context("token exchange request")?
    } else {
        client
            .post(token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", verifier),
            ])
            .send()
            .context("token exchange request")?
    };

    if !resp.status().is_success() {
        bail!(
            "token exchange failed ({}): {}",
            resp.status(),
            resp.text()?
        );
    }

    let json: serde_json::Value = resp.json().context("parsing token response")?;
    let access_token = json["access_token"]
        .as_str()
        .context("missing access_token")?
        .to_owned();
    let refresh_token = json["refresh_token"].as_str().map(ToOwned::to_owned);
    let expires_at = json["expires_in"].as_u64().map(|secs| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + secs
    });

    Ok((access_token, refresh_token, expires_at))
}

// ─── Refresh token flow ───────────────────────────────────────────────────────

/// Exchange a refresh token for a new access token.
///
/// Returns updated `Credentials` with the new access/refresh tokens.
/// The caller is responsible for saving the result via `auth::save()`.
pub fn refresh(creds: &Credentials) -> Result<Credentials> {
    let refresh_token = creds
        .refresh_token
        .as_deref()
        .context("no refresh_token stored — re-run: archon auth login")?;

    let (token_url, client_id) = match creds.provider.as_str() {
        "claude" => (CLAUDE_TOKEN_URL, CLAUDE_CLIENT_ID),
        "openai" => (OPENAI_TOKEN_URL, OPENAI_CLIENT_ID),
        other => bail!("unsupported provider for refresh: '{other}'"),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resp = if token_url.contains("claude.com") {
        client
            .post(token_url)
            .json(&serde_json::json!({
                "grant_type":    "refresh_token",
                "client_id":     client_id,
                "refresh_token": refresh_token,
            }))
            .send()
            .context("refresh token request")?
    } else {
        client
            .post(token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("refresh_token", refresh_token),
            ])
            .send()
            .context("refresh token request")?
    };

    if !resp.status().is_success() {
        bail!(
            "token refresh failed ({}): {}\n  Re-run: archon auth login --provider {}",
            resp.status(),
            resp.text()?,
            creds.provider,
        );
    }

    let json: serde_json::Value = resp.json().context("parsing refresh response")?;
    let access_token = json["access_token"]
        .as_str()
        .context("missing access_token in refresh response")?
        .to_owned();
    let new_refresh = json["refresh_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| creds.refresh_token.clone());
    let expires_at = json["expires_in"].as_u64().map(|secs| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + secs
    });

    Ok(Credentials {
        provider: creds.provider.clone(),
        token: access_token,
        refresh_token: new_refresh,
        expires_at,
    })
}

// ─── PKCE helpers ─────────────────────────────────────────────────────────────

fn pkce_pair() -> (String, String) {
    let mut raw = [0u8; 32];
    fill_random(&mut raw);
    let verifier = base64url(&raw);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64url(&digest);
    (verifier, challenge)
}

fn random_state() -> String {
    let mut raw = [0u8; 32];
    fill_random(&mut raw);
    base64url(&raw)
}

fn fill_random(buf: &mut [u8]) {
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(buf);
    } else {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        for (i, b) in buf.iter_mut().enumerate() {
            *b = ((t.wrapping_shr(i as u32 % 32)) ^ i as u32) as u8;
        }
    }
}

/// Base64url encoding without padding (RFC 4648 §5).
fn base64url(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        buf = (buf << 8) | u32::from(byte);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            out.push(CHARS[((buf >> bits) & 0x3f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(CHARS[((buf << (6 - bits)) & 0x3f) as usize] as char);
    }
    out
}

/// Percent-encode a string for use as a query parameter value (RFC 3986).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{b:02X}"));
            }
        }
    }
    out
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            Some(v.to_owned())
        } else {
            None
        }
    })
}
