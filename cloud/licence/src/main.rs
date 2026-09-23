//! Vidya dev licence service (prompts/P03 Step 2, docs/00-SYSTEM-CONTEXT.md §10).
//!
//! A small, SEPARATE Rust binary — not a member of the app workspace, so it never
//! ships inside the apps. In this phase it runs in `--dev` mode only. Phase 10
//! turns it into the production service (purchase webhook, admin panel, …).
//!
//! Usage (run from `cloud/licence/`):
//!   cargo run -- gen-code        # mint + store a VIDYA-XXXX-XXXX-XXXX code, print it
//!   cargo run -- --dev           # start the dev service on 127.0.0.1:8787
//!   cargo run -- print-key       # print the dev ed25519 public key (base64)
//!
//! On first run a dev ed25519 keypair is generated into `.dev-keys/` (gitignored)
//! and its public key is printed — paste it into `src-tauri/build-config/dev.json`
//! as `licence_public_key`.
//!
//! The code store is a small JSON file in `.dev-keys/` so the `gen-code` CLI and
//! the running server (separate processes) share state. (§10 calls it an
//! "in-memory code store"; a dev file-backed store is the practical equivalent
//! that lets the CLI seed codes the server can activate.)

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// Crockford base32 alphabet (excludes I, L, O, U) — docs §9 / P03 Step 2.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const DEFAULT_PORT: u16 = 8787;

// ------------------------------------------------------------------ store ---

/// The signed licence JSON (docs §10). Field set is fixed by the contract; the
/// app verifies the signature over these exact bytes, then parses them.
#[derive(Debug, Clone, Serialize)]
struct LicenceJson {
    licence_id: String,
    school_id: String,
    plan: String,
    max_students: Option<u32>,
    max_devices: Option<u32>,
    issued_at: String,
    server_machine_id: String,
}

/// One code's activation record (persisted so activation is idempotent).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Activation {
    machine_id: String,
    licence_id: String,
    school_id: String,
    school_name: String,
    issued_at: String,
    status: String, // active | revoked | moved
    /// The exact base64 licence + signature returned at first activation, so a
    /// retry from the same machine gets byte-identical results (idempotent).
    licence_b64: String,
    signature_b64: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CodeEntry {
    #[serde(default)]
    activation: Option<Activation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    codes: BTreeMap<String, CodeEntry>,
}

#[derive(Clone)]
struct AppState {
    signing: Arc<SigningKey>,
    store_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl AppState {
    fn read_store(&self) -> Store {
        match std::fs::read_to_string(&self.store_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Store::default(),
        }
    }
    fn write_store(&self, store: &Store) {
        let json = serde_json::to_string_pretty(store).expect("serialize store");
        let _ = std::fs::write(&self.store_path, json);
    }
}

// ------------------------------------------------------------------- keys ---

fn keys_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".dev-keys")
}

/// Load the dev signing key, generating (and persisting) one on first run.
fn ensure_keys() -> SigningKey {
    let dir = keys_dir();
    std::fs::create_dir_all(&dir).expect("create .dev-keys");
    let key_path = dir.join("ed25519.key");
    if let Ok(seed_b64) = std::fs::read_to_string(&key_path) {
        if let Ok(seed) = STANDARD.decode(seed_b64.trim()) {
            if let Ok(seed32) = <[u8; 32]>::try_from(seed.as_slice()) {
                return SigningKey::from_bytes(&seed32);
            }
        }
    }
    // First run: generate a random 32-byte seed → deterministic keypair.
    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    std::fs::write(&key_path, STANDARD.encode(seed)).expect("write dev key");
    std::fs::write(
        dir.join("ed25519.pub"),
        STANDARD.encode(signing.verifying_key().to_bytes()),
    )
    .expect("write dev pub");
    signing
}

fn public_key_b64(signing: &SigningKey) -> String {
    STANDARD.encode(signing.verifying_key().to_bytes())
}

// -------------------------------------------------------------- utilities ---

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn random_id(prefix: &str) -> String {
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    format!("{prefix}_{}", hex(&b))
}

/// Generate a `VIDYA-XXXX-XXXX-XXXX` code from Crockford base32 characters.
fn generate_code() -> String {
    let mut bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let groups: Vec<String> = (0..3)
        .map(|g| {
            (0..4)
                .map(|c| CROCKFORD[(bytes[g * 4 + c] % 32) as usize] as char)
                .collect::<String>()
        })
        .collect();
    format!("VIDYA-{}-{}-{}", groups[0], groups[1], groups[2])
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn sign_licence(signing: &SigningKey, lic: &LicenceJson) -> (String, String) {
    let raw = serde_json::to_vec(lic).expect("serialize licence");
    let sig = signing.sign(&raw);
    (STANDARD.encode(&raw), STANDARD.encode(sig.to_bytes()))
}

// ------------------------------------------------------------- API models ---

#[derive(Debug, Deserialize)]
struct ActivateReq {
    code: String,
    school_name: String,
    machine_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    app_version: String,
}

#[derive(Debug, Serialize)]
struct ActivateResp {
    licence: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct CheckReq {
    licence_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    machine_id: String,
}

#[derive(Debug, Serialize)]
struct CheckResp {
    status: String,
}

fn err(status: axum::http::StatusCode, code: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": code }))).into_response()
}

// --------------------------------------------------------------- handlers ---

/// `POST /v1/activate` — idempotent for the same machine_id; 409 for another.
async fn activate(State(state): State<AppState>, Json(req): Json<ActivateReq>) -> axum::response::Response {
    use axum::http::StatusCode;
    let _guard = state.lock.lock().unwrap();
    let mut store = state.read_store();

    let entry = match store.codes.get(&req.code) {
        Some(e) => e.clone(),
        None => return err(StatusCode::NOT_FOUND, "CODE_NOT_FOUND"),
    };

    if let Some(act) = entry.activation {
        // Same machine → return the exact same licence (idempotent).
        if act.machine_id == req.machine_id {
            return (
                StatusCode::OK,
                Json(ActivateResp { licence: act.licence_b64, signature: act.signature_b64 }),
            )
                .into_response();
        }
        // Used by another machine/school.
        return err(StatusCode::CONFLICT, "CODE_ALREADY_USED");
    }

    // First activation: mint + sign a perpetual, unlimited licence (dev default).
    let issued_at = now_iso();
    let lic = LicenceJson {
        licence_id: random_id("lic"),
        school_id: random_id("sch"),
        plan: "perpetual".to_string(),
        max_students: None,
        max_devices: None,
        issued_at: issued_at.clone(),
        server_machine_id: req.machine_id.clone(),
    };
    let (licence_b64, signature_b64) = sign_licence(&state.signing, &lic);

    store.codes.insert(
        req.code.clone(),
        CodeEntry {
            activation: Some(Activation {
                machine_id: req.machine_id,
                licence_id: lic.licence_id,
                school_id: lic.school_id,
                school_name: req.school_name,
                issued_at,
                status: "active".to_string(),
                licence_b64: licence_b64.clone(),
                signature_b64: signature_b64.clone(),
            }),
        },
    );
    state.write_store(&store);

    (StatusCode::OK, Json(ActivateResp { licence: licence_b64, signature: signature_b64 })).into_response()
}

/// `POST /v1/check` — report the licence status (active|revoked|moved).
async fn check(State(state): State<AppState>, Json(req): Json<CheckReq>) -> axum::response::Response {
    use axum::http::StatusCode;
    let _guard = state.lock.lock().unwrap();
    let store = state.read_store();
    for entry in store.codes.values() {
        if let Some(act) = &entry.activation {
            if act.licence_id == req.licence_id {
                return (StatusCode::OK, Json(CheckResp { status: act.status.clone() })).into_response();
            }
        }
    }
    err(StatusCode::NOT_FOUND, "CODE_NOT_FOUND")
}

/// `POST /v1/transfer` — not implemented until Phase 8 (licence transfer/restore).
async fn transfer() -> axum::response::Response {
    err(axum::http::StatusCode::NOT_IMPLEMENTED, "NOT_IMPLEMENTED")
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/activate", post(activate))
        .route("/v1/check", post(check))
        .route("/v1/transfer", post(transfer))
        .with_state(state)
}

// -------------------------------------------------------------------- main --

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let signing = ensure_keys();

    match arg.as_str() {
        "gen-code" => {
            let dir = keys_dir();
            let store_path = dir.join("store.json");
            let mut store: Store = std::fs::read_to_string(&store_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let code = generate_code();
            store.codes.insert(code.clone(), CodeEntry::default());
            std::fs::write(&store_path, serde_json::to_string_pretty(&store).unwrap())
                .expect("write store");
            println!("{code}");
        }
        "print-key" => {
            println!("{}", public_key_b64(&signing));
        }
        "--dev" | "serve" => {
            let port: u16 = std::env::var("LICENCE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(DEFAULT_PORT);
            let state = AppState {
                signing: Arc::new(signing),
                store_path: keys_dir().join("store.json"),
                lock: Arc::new(Mutex::new(())),
            };
            println!("Vidya dev licence service (dev mode)");
            println!("  listening on http://127.0.0.1:{port}");
            println!("  licence_public_key (paste into src-tauri/build-config/dev.json):");
            println!("    {}", public_key_b64(&state.signing));
            println!("  mint a code with:  cargo run -- gen-code");
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
                .await
                .expect("bind licence service");
            axum::serve(listener, router(state)).await.expect("serve licence service");
        }
        _ => {
            eprintln!(
                "usage:\n  cargo run -- gen-code     mint + store a VIDYA-XXXX-XXXX-XXXX code\n  \
                 cargo run -- --dev        start the dev service on 127.0.0.1:{DEFAULT_PORT}\n  \
                 cargo run -- print-key    print the dev ed25519 public key (base64)"
            );
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_shape_is_valid_crockford() {
        let code = generate_code();
        assert!(code.starts_with("VIDYA-"));
        let groups: Vec<&str> = code.split('-').collect();
        assert_eq!(groups.len(), 4); // VIDYA + 3 groups
        for g in &groups[1..] {
            assert_eq!(g.len(), 4);
            assert!(g.bytes().all(|b| CROCKFORD.contains(&b)), "only Crockford chars");
        }
        // Never contains the excluded letters.
        for bad in ['I', 'L', 'O', 'U'] {
            assert!(!code[6..].contains(bad), "must exclude {bad}");
        }
    }

    #[test]
    fn signed_licence_verifies_with_public_key() {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        let signing = SigningKey::from_bytes(&seed);
        let lic = LicenceJson {
            licence_id: "lic_1".into(),
            school_id: "sch_1".into(),
            plan: "perpetual".into(),
            max_students: None,
            max_devices: None,
            issued_at: "2026-09-23T00:00:00Z".into(),
            server_machine_id: "m1".into(),
        };
        let (lic_b64, sig_b64) = sign_licence(&signing, &lic);
        let raw = STANDARD.decode(lic_b64).unwrap();
        let sig = Signature::from_slice(&STANDARD.decode(sig_b64).unwrap()).unwrap();
        let vk = VerifyingKey::from_bytes(&signing.verifying_key().to_bytes()).unwrap();
        assert!(vk.verify(&raw, &sig).is_ok());
        // The signed bytes parse back to the same field values the app expects.
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(v["plan"], "perpetual");
        assert_eq!(v["max_students"], serde_json::Value::Null);
        assert_eq!(v["server_machine_id"], "m1");
    }
}
