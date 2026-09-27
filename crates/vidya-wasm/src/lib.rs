//! Thin wasm-bindgen wrapper around `vidya-core` (Phase 19, Step 1).
//!
//! The iPhone PWA runs the SAME business rules, HLC ordering, sealing and Argon2id
//! as the Android/desktop app by calling `vidya-core` compiled to WebAssembly —
//! never a re-implemented rule. This module only marshals values across the JS↔WASM
//! boundary (primitives + JSON strings, so no `serde-wasm-bindgen` dependency).
//!
//! Data across the boundary:
//!   * `wall_ms` is passed as `f64` (a JS number is exact to 2^53; a millisecond
//!     wall clock ~1.8e12 is far below that) and cast to the `u64` vidya-core uses.
//!   * A validation result is `None` on OK, or `Some(code)` — the stable
//!     `CoreError` code string the UI already maps (e.g. `"p_or_a"` → VALIDATION).
//!
//! Decisions are added here as the Web backend needs them; sealing/unsealing and
//! Argon2id land once their pure logic is shared into `vidya-core` (Step 1b/1c).

use vidya_core::attendance;
use vidya_core::hlc::{self, Hlc};
use vidya_core::{pin, seal};
use wasm_bindgen::prelude::*;

/// The vidya-core version compiled into the WASM — a smoke test that the module
/// loaded and that it matches the app version.
#[wasm_bindgen]
pub fn vidya_core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Validate a NEW attendance mark. `"P"`/`"A"` are accepted; `"L"` (legacy Leave)
/// is rejected exactly as on Android (`VALIDATION{rule:"p_or_a"}`). Returns `null`
/// when the mark is valid, or the stable error code string when it is rejected.
#[wasm_bindgen]
pub fn validate_new_mark(mark: &str) -> Option<String> {
    match attendance::parse_mark(mark) {
        Err(e) => Some(e.code().to_string()),
        Ok(m) => attendance::validate_new_mark(m).err().map(|e| e.code().to_string()),
    }
}

/// Attendance percentage in **tenths of a percent** (e.g. 913 = 91.3%), identical
/// to the native `percent_present` (legacy `L` counts in the denominator).
#[wasm_bindgen]
pub fn percent_present(p: u32, a: u32, l: u32) -> u32 {
    attendance::percent_present(p, a, l)
}

/// Build the canonical HLC string form from parts (for tests / a fixed clock).
#[wasm_bindgen]
pub fn hlc_new(wall_ms: f64, counter: u32, device_id: &str) -> String {
    Hlc::new(wall_ms as u64, counter as u16, device_id).to_string_form()
}

/// Advance the HLC for a new local event: `now(prev, wall_ms, device_id)`. `prev`
/// is the last HLC string this device produced (or `null` for the first). Returns
/// the new HLC string; the `Err(code)` is thrown to JS if `prev` is malformed.
/// (`Result<_, String>` rather than `JsError` so the same code is testable on the
/// host too — `JsError` cannot be constructed on a non-wasm target.)
#[wasm_bindgen]
pub fn hlc_next(prev: Option<String>, wall_ms: f64, device_id: &str) -> Result<String, String> {
    let prev_hlc = match prev {
        Some(s) => Some(Hlc::parse(&s).map_err(|e| e.code().to_string())?),
        None => None,
    };
    Ok(hlc::now(prev_hlc.as_ref(), wall_ms as u64, device_id).to_string_form())
}

/// Parse an HLC string into `{wall_ms, counter, device_id}` JSON; throws the error
/// code on a malformed HLC. (String compare on the canonical form is the total
/// order, so JS orders ops by plain string comparison — no wrapper needed for that.)
#[wasm_bindgen]
pub fn hlc_parse(s: &str) -> Result<String, String> {
    let h = Hlc::parse(s).map_err(|e| e.code().to_string())?;
    Ok(serde_json::json!({
        "wall_ms": h.wall_ms,
        "counter": h.counter,
        "device_id": h.device_id,
    })
    .to_string())
}

/// True when the device and server wall clocks differ enough to flag (same rule as
/// the native check-in clock-skew warning).
#[wasm_bindgen]
pub fn hlc_skew(local_wall_ms: f64, remote_wall_ms: f64) -> bool {
    hlc::skew_flag(local_wall_ms as u64, remote_wall_ms as u64)
}

// ---- Sealing (identical .vop bundle / relay envelope crypto as the app) --------
// The browser supplies the 12-byte nonce (crypto.getRandomValues) and does the
// ops→JSON itself; these wrap the pure `vidya_core::seal` primitives so a bundle
// sealed here is byte-compatible with one sealed by the Android/desktop app.

fn key32(k: &[u8]) -> Result<[u8; 32], String> {
    k.try_into().map_err(|_| "key must be 32 bytes".to_string())
}

/// Seal `plaintext` under a 32-byte `key` with `aad` and a caller-supplied 12-byte
/// `nonce` → `nonce ‖ ciphertext+tag`. Throws on a bad key/nonce length.
#[wasm_bindgen]
pub fn seal_with_nonce(key: &[u8], aad: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    seal::seal_with_nonce(&key32(key)?, aad, nonce, plaintext).map_err(|e| format!("{e:?}"))
}

/// Open a `nonce ‖ ciphertext+tag` blob under a 32-byte `key` with `aad`. Throws on
/// a tampered/short blob, wrong key, or wrong associated data.
#[wasm_bindgen]
pub fn seal_open(key: &[u8], aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, String> {
    seal::open(&key32(key)?, aad, sealed).map_err(|e| format!("{e:?}"))
}

/// Associated data for a Drive `.vop` bundle (audience + key version).
#[wasm_bindgen]
pub fn bundle_aad(audience: &str, key_version: f64) -> Vec<u8> {
    seal::bundle_aad(audience, key_version as i64)
}

/// Associated data for the relay route (method/path/device/epoch).
#[wasm_bindgen]
pub fn relay_aad(method: &str, path: &str, device_id: &str, server_epoch: f64) -> Vec<u8> {
    seal::associated_data(method, path, device_id, server_epoch as i64)
}

/// Derive the two per-direction relay keys from a base64 session key, returned as
/// `c2s(32) ‖ s2c(32)` (64 bytes). Throws if the session key is not 32 bytes.
#[wasm_bindgen]
pub fn derive_direction_keys(session_key_b64: &str) -> Result<Vec<u8>, String> {
    let k = seal::derive_keys(session_key_b64).map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&k.c2s);
    out.extend_from_slice(&k.s2c);
    Ok(out)
}

/// The 32-byte symmetric key that seals a Drive join response, derived from the
/// single-use invite code the PWA scanned (identical to the school PC's key).
#[wasm_bindgen]
pub fn join_key(invite_code: &str) -> Vec<u8> {
    seal::join_key(invite_code).to_vec()
}

/// Associated data binding a join response to its request id.
#[wasm_bindgen]
pub fn join_aad(request_id: &str) -> Vec<u8> {
    seal::join_aad(request_id)
}

// ---- PIN (Argon2id, identical parameters as the app) --------------------------

/// Hash a PIN with a caller-supplied 16-byte random salt → a PHC string. Throws on
/// a bad salt length. (The browser provides the salt via crypto.getRandomValues.)
#[wasm_bindgen]
pub fn pin_hash_with_salt(pin: &str, salt: &[u8]) -> Result<String, String> {
    let salt: [u8; 16] = salt.try_into().map_err(|_| "salt must be 16 bytes".to_string())?;
    pin::hash_pin_with_salt(pin, &salt)
}

/// Verify a PIN against a stored PHC string (identical Argon2id verify as the app).
#[wasm_bindgen]
pub fn pin_verify(pin_input: &str, phc: &str) -> bool {
    pin::verify_pin(pin_input, phc)
}

/// Lockout wait (seconds) after `fail_count` wrong PINs, or `undefined` before the
/// 5th failure. Same curve as the app (30 s doubling, capped at 1 h).
#[wasm_bindgen]
pub fn pin_lockout_seconds(fail_count: u32) -> Option<u32> {
    pin::lockout_seconds(fail_count).map(|s| s as u32)
}

/// Tries remaining before the account locks.
#[wasm_bindgen]
pub fn pin_remaining(fail_count: u32) -> u32 {
    pin::remaining_before_lock(fail_count)
}

/// Whether the account is currently locked (times in epoch ms).
#[wasm_bindgen]
pub fn pin_is_locked(locked_until_ms: Option<f64>, now_ms: f64) -> bool {
    pin::is_locked(locked_until_ms.map(|v| v as i64), now_ms as i64)
}

// ---- Parity tests: the SAME vectors run natively (`cargo test`) and in WASM
// (`wasm-pack test --node`), proving the wrapper is faithful to vidya-core. ----
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn version_matches_crate() {
        assert_eq!(vidya_core_version(), env!("CARGO_PKG_VERSION"));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn attendance_mark_and_percent_match_core() {
        // P/A accepted; legacy L rejected with the same code as Android.
        assert_eq!(validate_new_mark("P"), None);
        assert_eq!(validate_new_mark("A"), None);
        assert_eq!(validate_new_mark("L").as_deref(), Some("VALIDATION"));
        assert_eq!(validate_new_mark("Z").as_deref(), Some("VALIDATION"));
        // Same vector as the vidya-core doctest: 612 present of 670 → 913 tenths.
        assert_eq!(percent_present(612, 58, 0), 913);
        assert_eq!(percent_present(0, 0, 0), 0);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn hlc_round_trips_and_orders() {
        let a = hlc_new(1_800_000_000_000.0, 0, "dev-a1");
        let b = hlc_new(1_800_000_000_000.0, 1, "dev-a1");
        assert!(a < b, "string compare is the total order");
        // parse round-trips the parts.
        let parsed = hlc_parse(&a).unwrap();
        assert!(parsed.contains("\"device_id\":\"dev-a1\""));
        assert!(parsed.contains("\"counter\":0"));
        // next() advances monotonically even when the wall clock does not move.
        let n1 = hlc_next(Some(a.clone()), 1_800_000_000_000.0, "dev-a1").unwrap();
        assert!(n1 > a, "counter advances when wall_ms is unchanged");
        // a malformed HLC throws.
        assert!(hlc_parse("not-an-hlc").is_err());
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn sealing_round_trips_and_is_deterministic() {
        // Interop: a bundle sealed here is byte-identical to one the app seals with
        // the same key+aad+nonce+plaintext, and each side can open the other's.
        let key = [7u8; 32];
        let nonce = [9u8; 12];
        let aad = bundle_aad("class:c1", 3.0);
        let sealed = seal_with_nonce(&key, &aad, &nonce, b"ops").unwrap();
        assert_eq!(sealed, seal_with_nonce(&key, &aad, &nonce, b"ops").unwrap());
        assert_eq!(seal_open(&key, &aad, &sealed).unwrap(), b"ops");
        assert!(seal_open(&key, &bundle_aad("finance", 3.0), &sealed).is_err(), "wrong audience can't open");
        assert!(seal_with_nonce(&[0u8; 8], &aad, &nonce, b"x").is_err(), "bad key length throws");
        // Relay direction keys derive to 64 bytes (c2s ‖ s2c), and the two differ.
        let dk = derive_direction_keys(&base64_std([7u8; 32])).unwrap();
        assert_eq!(dk.len(), 64);
        assert_ne!(&dk[..32], &dk[32..]);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn pin_hash_verify_and_lockout_match_core() {
        let salt = [3u8; 16];
        let phc = pin_hash_with_salt("1234", &salt).unwrap();
        assert!(pin_verify("1234", &phc));
        assert!(!pin_verify("4321", &phc));
        assert!(phc.contains("m=19456,t=2,p=1"), "identical Argon2id params");
        assert!(pin_hash_with_salt("1234", &[0u8; 8]).is_err(), "bad salt length throws");
        assert_eq!(pin_lockout_seconds(4), None);
        assert_eq!(pin_lockout_seconds(5), Some(30));
        assert_eq!(pin_remaining(4), 1);
        assert!(pin_is_locked(Some(2000.0), 1000.0));
        assert!(!pin_is_locked(None, 1000.0));
    }

    /// Base64 (standard) of a 32-byte array — a tiny helper for the relay-key test
    /// (the app passes the session key as base64).
    fn base64_std(bytes: [u8; 32]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }
}
