//! Google OAuth 2.0 for the DESKTOP Drive backup (Phase A / "#1").
//!
//! Installed-app flow with a **loopback redirect + PKCE** — verified against Google's
//! official "OAuth 2.0 for Mobile & Desktop Apps" doc:
//! * auth endpoint  `https://accounts.google.com/o/oauth2/v2/auth`
//! * token endpoint `https://oauth2.googleapis.com/token`
//! * redirect `http://127.0.0.1:<random-port>` (loopback; no client secret — PKCE
//!   replaces it for Desktop clients)
//! * refresh tokens are always returned for installed apps.
//!
//! Scope is the minimal `drive.file` (the app only ever touches files it created), so
//! backing up to the owner's OWN Drive needs no shared-folder behaviour — it is NOT
//! blocked by the Phase-6 exchange spike (that spike is only about staff reading each
//! other's files, which backup never does).
//!
//! Tokens live in the encrypted `app_kv` table (SQLCipher at rest). The client id is
//! the non-secret build value `google_client_id_desktop` (config.rs).

use crate::kv;
use base64::Engine as _;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
/// app_kv key holding the persisted tokens.
pub const KV_TOKENS: &str = "drive_oauth_tokens";
/// Refresh this many seconds before the access token truly expires.
const EXPIRY_SKEW_SECS: u64 = 60;
/// Give the user this long to complete consent before we give up.
const CONSENT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error)]
pub enum OauthError {
    #[error("google sign-in was cancelled or timed out")]
    Timeout,
    #[error("google sign-in failed: {0}")]
    Denied(String),
    #[error("not connected to Google Drive")]
    NotConnected,
    #[error("google returned no refresh token (re-consent needed)")]
    NoRefreshToken,
    #[error("oauth transport: {0}")]
    Io(String),
}

pub type OauthResult<T> = Result<T, OauthError>;

/// Persisted Google tokens (stored encrypted in `app_kv`). `account_email` is shown
/// in the UI ("Connected as …"); it is best-effort and may be empty.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriveTokens {
    pub refresh_token: String,
    pub access_token: String,
    /// Unix seconds at which `access_token` should be considered expired.
    pub expires_at: u64,
    #[serde(default)]
    pub account_email: String,
}

impl DriveTokens {
    fn is_access_fresh(&self) -> bool {
        !self.access_token.is_empty() && now_unix() + EXPIRY_SKEW_SECS < self.expires_at
    }
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// ---- PKCE ------------------------------------------------------------------

/// A PKCE pair: the secret `verifier` (kept in memory) and the `challenge` sent in
/// the authorization request (`S256`). `challenge == base64url_nopad(sha256(verifier))`.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// A random URL-safe token of `n` base64url chars' worth of entropy.
fn random_b64url(n_bytes: usize) -> String {
    use rand::RngCore;
    let mut bytes = vec![0u8; n_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn pkce() -> Pkce {
    let verifier = random_b64url(32); // 43 chars — within the 43..128 spec range
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    Pkce { verifier, challenge }
}

/// Build the authorization URL (properly percent-encoded via `reqwest::Url`).
pub fn auth_url(client_id: &str, redirect_uri: &str, challenge: &str, state: &str) -> String {
    reqwest::Url::parse_with_params(
        AUTH_ENDPOINT,
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

// ---- token persistence -----------------------------------------------------

pub fn load_tokens(conn: &Connection) -> Option<DriveTokens> {
    kv::get::<DriveTokens>(conn, KV_TOKENS).ok().flatten()
}

pub fn store_tokens(conn: &Connection, tokens: &DriveTokens) -> rusqlite::Result<()> {
    kv::set(conn, KV_TOKENS, tokens)
}

pub fn clear_tokens(conn: &Connection) -> rusqlite::Result<()> {
    kv::delete(conn, KV_TOKENS)
}

pub fn is_connected(conn: &Connection) -> bool {
    load_tokens(conn).map(|t| !t.refresh_token.is_empty()).unwrap_or(false)
}

// ---- HTTP (async; callers own the runtime) ---------------------------------

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

async fn post_token_form(client: &reqwest::Client, form: &[(&str, &str)]) -> OauthResult<TokenResponse> {
    let resp = client
        .post(TOKEN_ENDPOINT)
        .form(form)
        .send()
        .await
        .map_err(|e| OauthError::Io(e.to_string()))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| OauthError::Io(e.to_string()))?;
    if !status.is_success() {
        let msg = serde_json::from_str::<ErrorResponse>(&body)
            .map(|e| if e.error_description.is_empty() { e.error } else { e.error_description })
            .unwrap_or_else(|_| format!("HTTP {status}"));
        return Err(OauthError::Denied(msg));
    }
    serde_json::from_str::<TokenResponse>(&body).map_err(|e| OauthError::Io(e.to_string()))
}

/// Exchange an authorization `code` (+ PKCE verifier) for tokens.
pub async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> OauthResult<DriveTokens> {
    let r = post_token_form(
        client,
        &[
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ],
    )
    .await?;
    let refresh_token = r.refresh_token.ok_or(OauthError::NoRefreshToken)?;
    Ok(DriveTokens {
        refresh_token,
        access_token: r.access_token,
        expires_at: now_unix() + r.expires_in,
        account_email: String::new(),
    })
}

/// Use the refresh token to get a fresh access token. Returns the new access token
/// and its expiry (the refresh token itself does not change).
pub async fn refresh_access(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> OauthResult<(String, u64)> {
    let r = post_token_form(
        client,
        &[
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ],
    )
    .await?;
    Ok((r.access_token, now_unix() + r.expires_in))
}

/// Return a valid access token, transparently refreshing via the stored refresh
/// token when the cached access token is stale (within [`EXPIRY_SKEW_SECS`] of
/// expiry). Persists the refreshed token. This is the token provider the Drive REST
/// client (A2) uses. **Sync** — it owns a short-lived runtime, so call it from a
/// blocking context (not from inside another Tokio `block_on`).
pub fn valid_access_token(conn: &Connection, client_id: &str) -> OauthResult<String> {
    let mut tokens = load_tokens(conn).ok_or(OauthError::NotConnected)?;
    if tokens.refresh_token.is_empty() {
        return Err(OauthError::NotConnected);
    }
    if tokens.is_access_fresh() {
        return Ok(tokens.access_token);
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| OauthError::Io(e.to_string()))?;
    let client = reqwest::Client::new();
    let (access, expires_at) = rt.block_on(refresh_access(&client, client_id, &tokens.refresh_token))?;
    tokens.access_token = access.clone();
    tokens.expires_at = expires_at;
    let _ = store_tokens(conn, &tokens); // best-effort cache; a failed write just re-refreshes next time
    Ok(access)
}

// ---- interactive connect (mobile: custom-scheme redirect via deep link) -----
//
// Desktop uses a loopback redirect (below). Android cannot bind a loopback the system
// browser can reach, so it uses Google's "installed app" **custom-scheme** redirect —
// the reversed client id — captured by tauri-plugin-deep-link. The flow is split in
// two (the deep-link callback is event-driven, not a blocking server): the app calls
// [`begin_mobile_connect`] to get the auth URL (opened in a Custom Tab), then feeds the
// deep-link callback URL to [`finish_mobile_connect`]. The REST client + token
// exchange + storage are shared with desktop. Device-verified (the deep-link routing
// itself only works on a real Android build).

/// The redirect URI for an Android OAuth client: `…-xyz.apps.googleusercontent.com`
/// → `com.googleusercontent.apps.…-xyz:/oauth2redirect` (Google's documented reversed
/// client id scheme for installed apps).
pub fn android_redirect_uri(client_id: &str) -> String {
    let id = client_id.strip_suffix(".apps.googleusercontent.com").unwrap_or(client_id);
    format!("com.googleusercontent.apps.{id}:/oauth2redirect")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PendingConnect {
    verifier: String,
    state: String,
    client_id: String,
    redirect_uri: String,
}
const KV_PENDING: &str = "drive_oauth_pending";

/// Begin a mobile connect: stash PKCE + anti-CSRF `state`, return the authorization URL
/// for the app to open in a Custom Tab. Finished by [`finish_mobile_connect`].
pub fn begin_mobile_connect(conn: &Connection, client_id: &str) -> OauthResult<String> {
    let p = pkce();
    let state = random_b64url(16);
    let redirect_uri = android_redirect_uri(client_id);
    let url = auth_url(client_id, &redirect_uri, &p.challenge, &state);
    let pending = PendingConnect { verifier: p.verifier, state, client_id: client_id.to_string(), redirect_uri };
    kv::set(conn, KV_PENDING, &pending).map_err(|e| OauthError::Io(e.to_string()))?;
    Ok(url)
}

/// Finish a mobile connect from the deep-link callback URL: verify `state`, exchange the
/// code (PKCE), store the tokens. Sync — owns a short-lived runtime (call off any
/// ambient Tokio runtime).
pub fn finish_mobile_connect(conn: &Connection, callback_url: &str) -> OauthResult<DriveTokens> {
    let pending: PendingConnect =
        kv::get(conn, KV_PENDING).ok().flatten().ok_or_else(|| OauthError::Denied("no pending connect".into()))?;
    let url = reqwest::Url::parse(callback_url).map_err(|_| OauthError::Denied("bad callback url".into()))?;
    let (mut code, mut state) = (None, None);
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            "error" => return Err(OauthError::Denied(v.into_owned())),
            _ => {}
        }
    }
    let code = code.ok_or_else(|| OauthError::Denied("no code in callback".into()))?;
    if state.as_deref() != Some(pending.state.as_str()) {
        return Err(OauthError::Denied("state mismatch".into()));
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| OauthError::Io(e.to_string()))?;
    let client = reqwest::Client::new();
    let tokens = rt.block_on(exchange_code(&client, &pending.client_id, &code, &pending.verifier, &pending.redirect_uri))?;
    let _ = kv::delete(conn, KV_PENDING);
    store_tokens(conn, &tokens).map_err(|e| OauthError::Io(e.to_string()))?;
    Ok(tokens)
}

// ---- interactive connect (loopback) ----------------------------------------

/// Parse the HTTP request line of the loopback redirect, returning `(code, state)`
/// or an error carried in the query (`error=access_denied`). Expects a line like
/// `GET /?code=...&state=... HTTP/1.1`.
fn parse_redirect(request_line: &str) -> OauthResult<(String, String)> {
    let path = request_line.split_whitespace().nth(1).unwrap_or("");
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{path}"))
        .map_err(|_| OauthError::Denied("bad redirect".into()))?;
    let mut code = None;
    let mut state = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            "error" => return Err(OauthError::Denied(v.into_owned())),
            _ => {}
        }
    }
    match (code, state) {
        (Some(c), Some(s)) => Ok((c, s)),
        _ => Err(OauthError::Denied("no code in redirect".into())),
    }
}

/// Best-effort: open `url` in the user's default browser.
fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// Run the full interactive connect flow (blocking): bind a loopback port, open the
/// browser to Google, wait for the redirect, exchange the code, and return tokens.
/// Call from a blocking context (e.g. `tokio::task::spawn_blocking`).
pub fn run_connect(client_id: &str) -> OauthResult<DriveTokens> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| OauthError::Io(e.to_string()))?;
    rt.block_on(connect_flow(client_id))
}

async fn connect_flow(client_id: &str) -> OauthResult<DriveTokens> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| OauthError::Io(e.to_string()))?;
    let port = listener.local_addr().map_err(|e| OauthError::Io(e.to_string()))?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let pkce = pkce();
    let state = random_b64url(16);
    open_browser(&auth_url(client_id, &redirect_uri, &pkce.challenge, &state));

    // Wait (with a timeout) for Google to redirect back to our loopback.
    let (mut stream, _) = tokio::time::timeout(CONSENT_TIMEOUT, listener.accept())
        .await
        .map_err(|_| OauthError::Timeout)?
        .map_err(|e| OauthError::Io(e.to_string()))?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await.map_err(|e| OauthError::Io(e.to_string()))?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let parsed = parse_redirect(first_line);

    // Always answer the browser so the tab shows a friendly message.
    let (status, body) = match &parsed {
        Ok(_) => ("200 OK", "Vidya is connected to Google Drive. You can close this tab."),
        Err(_) => ("400 Bad Request", "Google sign-in did not complete. You can close this tab and try again."),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n<!doctype html><meta charset=utf-8><title>Vidya</title><body style=\"font:16px system-ui;padding:40px\">{}</body>",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;

    let (code, got_state) = parsed?;
    if got_state != state {
        return Err(OauthError::Denied("state mismatch (possible CSRF)".into()));
    }

    let client = reqwest::Client::new();
    exchange_code(&client, client_id, &code, &pkce.verifier, &redirect_uri).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let p = pkce();
        // verifier is base64url(32 bytes) → 43 chars, no padding.
        assert_eq!(p.verifier.len(), 43);
        assert!(!p.verifier.contains('='));
        let expect = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.as_bytes()));
        assert_eq!(p.challenge, expect);
        assert!(!p.challenge.contains('=') && !p.challenge.contains('+') && !p.challenge.contains('/'));
    }

    #[test]
    fn android_redirect_is_reversed_client_id() {
        assert_eq!(
            android_redirect_uri("665784308071-abc.apps.googleusercontent.com"),
            "com.googleusercontent.apps.665784308071-abc:/oauth2redirect"
        );
        // A bare id (already reversed / unexpected) is passed through, not corrupted.
        assert_eq!(android_redirect_uri("plain-id"), "com.googleusercontent.apps.plain-id:/oauth2redirect");
    }

    #[test]
    fn mobile_connect_roundtrip_state_is_enforced() {
        let mut c = crate::db::open_in_memory("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        crate::db::run_migrations(&mut c).unwrap();
        let url = begin_mobile_connect(&c, "665784308071-abc.apps.googleusercontent.com").unwrap();
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("com.googleusercontent.apps.665784308071-abc"));
        // A callback with the wrong state is rejected (anti-CSRF), before any network.
        let bad = finish_mobile_connect(&c, "com.googleusercontent.apps.x:/oauth2redirect?code=C&state=WRONG");
        assert!(matches!(bad, Err(OauthError::Denied(_))));
    }

    #[test]
    fn auth_url_has_required_params() {
        let u = auth_url("CID", "http://127.0.0.1:12345", "CHAL", "STATE");
        assert!(u.starts_with(AUTH_ENDPOINT));
        assert!(u.contains("client_id=CID"));
        assert!(u.contains("response_type=code"));
        assert!(u.contains("code_challenge=CHAL"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("state=STATE"));
        assert!(u.contains("access_type=offline"));
        // scope is percent-encoded
        assert!(u.contains("drive.file"));
        // loopback redirect is percent-encoded
        assert!(u.contains("127.0.0.1"));
    }

    #[test]
    fn parse_redirect_extracts_code_and_state() {
        let (c, s) = parse_redirect("GET /?code=abc123&state=xyz HTTP/1.1").unwrap();
        assert_eq!(c, "abc123");
        assert_eq!(s, "xyz");
    }

    #[test]
    fn parse_redirect_surfaces_denial() {
        let e = parse_redirect("GET /?error=access_denied HTTP/1.1").unwrap_err();
        assert!(matches!(e, OauthError::Denied(_)));
    }

    #[test]
    fn valid_access_token_errs_when_not_connected() {
        use crate::db;
        const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        // No tokens stored → NotConnected (no network hit).
        assert!(matches!(valid_access_token(&c, "CID").unwrap_err(), OauthError::NotConnected));
        // A fresh cached access token is returned without any refresh/network.
        let fresh = DriveTokens {
            refresh_token: "r".into(),
            access_token: "cached".into(),
            expires_at: now_unix() + 3600,
            account_email: String::new(),
        };
        store_tokens(&c, &fresh).unwrap();
        assert_eq!(valid_access_token(&c, "CID").unwrap(), "cached");
    }

    #[test]
    fn freshness_respects_skew() {
        let mut t = DriveTokens { access_token: "a".into(), expires_at: now_unix() + 10, ..Default::default() };
        assert!(!t.is_access_fresh()); // within the 60s skew → treated as stale
        t.expires_at = now_unix() + 3600;
        assert!(t.is_access_fresh());
        t.access_token = String::new();
        assert!(!t.is_access_fresh());
    }
}
