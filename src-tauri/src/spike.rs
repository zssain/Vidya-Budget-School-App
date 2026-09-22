//! Size-spike command (Phase 1 only). Never called at runtime; it exists so the
//! linker keeps every native dependency from docs/00-SYSTEM-CONTEXT.md §13, so
//! the real installer/installed sizes are measured with everything linked in.
//! Phase 2 deletes this file once real code uses these crates.
#![allow(dead_code)]

use base64::Engine as _;
use chacha20poly1305::aead::{Aead as _, KeyInit as _};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use ed25519_dalek::{Signer as _, SigningKey, Verifier as _};
use hmac::Mac as _;
use sha2::Digest as _;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
enum SpikeError {
    #[error("spike: {0}")]
    Msg(String),
}

#[tauri::command]
pub fn size_spike() -> Result<String, String> {
    real_spike().map_err(|e| e.to_string())
}

fn real_spike() -> Result<String, SpikeError> {
    let m = |e: String| SpikeError::Msg(e);

    // --- SQLCipher (rusqlite): in-memory DB, PRAGMA key, SELECT version ---
    let conn = rusqlite::Connection::open_in_memory().map_err(|e| m(e.to_string()))?;
    conn.pragma_update(None, "key", "spike-passphrase")
        .map_err(|e| m(e.to_string()))?;
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|e| m(e.to_string()))?;

    // --- reqwest client (rustls/ring) ---
    let _client = reqwest::Client::builder()
        .build()
        .map_err(|e| m(e.to_string()))?;

    // --- rcgen self-signed cert ---
    let certified =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).map_err(|e| m(e.to_string()))?;
    let cert_der = certified.cert.der().to_vec();
    let key_der = certified.key_pair.serialize_der();

    // --- rustls ServerConfig with the ring provider + tokio_rustls acceptor ---
    let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
        key_der,
    ));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server_config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| m(e.to_string()))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| m(e.to_string()))?;
    let _acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

    // --- axum router ---
    let _app: axum::Router = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));

    // --- hyper / hyper-util ---
    let _hdr = hyper::header::HeaderName::from_static("x-vidya");
    let _exec = hyper_util::rt::TokioExecutor::new();

    // --- tokio-tungstenite client request (do NOT connect) ---
    use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
    let _ws_req = "wss://school.invalid/vidya"
        .into_client_request()
        .map_err(|e| m(e.to_string()))?;

    // --- argon2 (low-level, no SaltString) ---
    let mut argon_out = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(b"pin-1234", b"spike-salt-16byte", &mut argon_out)
        .map_err(|e| m(e.to_string()))?;

    // --- chacha20poly1305 seal ---
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&[7u8; 32]));
    let _sealed = cipher
        .encrypt(Nonce::from_slice(&[0u8; 12]), b"vidya".as_ref())
        .map_err(|e| m(e.to_string()))?;

    // --- ed25519-dalek sign + verify ---
    let signing = SigningKey::generate(&mut rand::rngs::OsRng);
    let msg = b"vidya-budget-school";
    let sig = signing.sign(msg);
    signing
        .verifying_key()
        .verify(msg, &sig)
        .map_err(|e| m(e.to_string()))?;

    // --- sha2 / hmac / base64 / uuid / time ---
    let digest = sha2::Sha256::digest(msg);
    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let mut mac = <HmacSha256 as hmac::Mac>::new_from_slice(&[0u8; 32]).map_err(|e| m(e.to_string()))?;
    mac.update(msg);
    let tag = mac.finalize().into_bytes();
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    let id = uuid::Uuid::now_v7();
    let now = time::OffsetDateTime::now_utc();

    // --- qrcode SVG ---
    let code = qrcode::QrCode::new(b"https://vidyabudget.in").map_err(|e| m(e.to_string()))?;
    let qr_svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(120, 120)
        .build();

    // --- tracing / tracing-subscriber ---
    let _builder = tracing_subscriber::fmt();
    tracing::info!(target: "spike", "size spike linked");

    // --- serde_json / tokio touch ---
    let _json = serde_json::json!({ "ok": true, "id": id.to_string() });
    let _dur = tokio::time::Duration::from_secs(1);

    // --- desktop-only: mdns-sd + keyring ---
    #[cfg(not(target_os = "android"))]
    {
        let _daemon = mdns_sd::ServiceDaemon::new().map_err(|e| m(e.to_string()))?;
        // ignore errors: no entry is expected to exist
        let _ = keyring::Entry::new("in.vidyabudget.app", "db-key").and_then(|e| e.get_password());
    }

    Ok(format!(
        "sqlite={} cert_tag={} hmac_len={} qr_len={} at={}",
        sqlite_version,
        &b64[..8.min(b64.len())],
        tag.len(),
        qr_svg.len(),
        now,
    ))
}
