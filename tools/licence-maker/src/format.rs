//! Licence v2 wire format — a faithful mirror of `vidya-core::licence` (docs §10,
//! prompts/P12 Step 6/7).
//!
//! `licence-maker` cannot depend on `vidya-core` (Step 7 limits its dependencies
//! and keeps it out of the app workspace), so the machine-code and licence-key
//! encoding are re-implemented here. They MUST stay byte-for-byte identical to
//! `vidya-core`; the `golden_vector_matches_vidya_core` test in `main.rs` pins a
//! key that `vidya-core`'s own test also verifies.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
// `machine_code` (the deriver) is only needed by the golden-vector tests — the
// CLI is given the buyer's code and only validates it (see `canonical_machine_code`).
#[cfg(test)]
use sha2::{Digest, Sha256};

/// Crockford base32 alphabet (no I, L, O, U).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The signed payload of a v2 offline licence. Field order matches
/// `vidya-core::licence::LicenceV2` exactly (serde serialises in declaration
/// order, so the signed bytes are identical).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceV2 {
    pub v: u8,
    pub licence_id: String,
    pub school_name: String,
    pub machine_code: String,
    pub issued_at: String,
    pub plan: String,
    pub modules: Vec<String>,
}

/// Derive the display machine code from a `machine_id` (mirror of vidya-core).
#[cfg(test)]
pub fn machine_code(machine_id: &str) -> String {
    let digest = Sha256::digest(machine_id.as_bytes());
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&digest[0..8]);
    let bits60 = u64::from_be_bytes(first8) >> 4;
    let mut idx = [0usize; 12];
    let mut sum = 0usize;
    for (i, slot) in idx.iter_mut().enumerate() {
        let v = ((bits60 >> (5 * (11 - i))) & 0x1f) as usize;
        *slot = v;
        sum += v;
    }
    format_machine_code(&idx, sum % 32)
}

/// Format 12 data indices + a checksum index into `XXXX-XXXX-XXXX-C`.
fn format_machine_code(idx: &[usize; 12], check: usize) -> String {
    let c = |g: usize, k: usize| CROCKFORD[idx[g * 4 + k]] as char;
    format!(
        "{}{}{}{}-{}{}{}{}-{}{}{}{}-{}",
        c(0, 0), c(0, 1), c(0, 2), c(0, 3),
        c(1, 0), c(1, 1), c(1, 2), c(1, 3),
        c(2, 0), c(2, 1), c(2, 2), c(2, 3),
        CROCKFORD[check] as char,
    )
}

fn crockford_value(ch: char) -> Option<usize> {
    let mapped = match ch.to_ascii_uppercase() {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    };
    CROCKFORD.iter().position(|&a| a as char == mapped)
}

/// Validate a typed machine code; return the 13 data+checksum indices on success.
fn machine_code_indices(text: &str) -> Option<[usize; 13]> {
    let mut vals = Vec::with_capacity(13);
    for ch in text.chars() {
        if ch == '-' || ch.is_whitespace() {
            continue;
        }
        vals.push(crockford_value(ch)?);
    }
    if vals.len() != 13 {
        return None;
    }
    let sum: usize = vals[..12].iter().sum();
    if sum % 32 != vals[12] {
        return None;
    }
    let mut out = [0usize; 13];
    out.copy_from_slice(&vals);
    Some(out)
}

/// Canonical `XXXX-XXXX-XXXX-C` display form of a valid typed machine code
/// (uppercase, re-dashed), or `None` if it is invalid.
pub fn canonical_machine_code(text: &str) -> Option<String> {
    let all = machine_code_indices(text)?;
    let mut data = [0usize; 12];
    data.copy_from_slice(&all[..12]);
    Some(format_machine_code(&data, all[12]))
}

/// Encode `base64url(payload ‖ signature)`, no padding (mirror of vidya-core).
pub fn encode_licence_key(payload: &[u8], signature: &[u8]) -> String {
    let mut buf = Vec::with_capacity(payload.len() + signature.len());
    buf.extend_from_slice(payload);
    buf.extend_from_slice(signature);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Split a pasted key back into `(payload, signature)` (mirror of vidya-core;
/// used by the `verify` command).
pub fn parse_licence_key(text: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace() && *c != '=').collect();
    let bytes = URL_SAFE_NO_PAD.decode(compact.as_bytes()).map_err(|_| "not a valid licence key".to_string())?;
    if bytes.len() <= 64 {
        return Err("licence key too short".into());
    }
    let split = bytes.len() - 64;
    Ok((bytes[..split].to_vec(), bytes[split..].to_vec()))
}

/// Show a compact licence key in groups of 5 for pasting.
pub fn group5(key: &str) -> String {
    key.as_bytes()
        .chunks(5)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ")
}
