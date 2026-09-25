//! Licence verification and status (prompts/P02 licence.rs, docs §10; v2 offline
//! files in prompts/P12 Step 6).
//!
//! Business model: **one-time purchase, perpetual licence, no expiry, no online
//! check, no remote revoke.** The app verifies an ed25519 signature offline
//! against the public key from build config.
//!
//! Two licence formats coexist:
//! - **v1** (already-activated installs): the [`Licence`] JSON the retired
//!   `cloud/licence` service signed. [`verify`] still accepts it and it is now
//!   treated as perpetual (no 30-day re-check, no grace — that logic is removed).
//! - **v2** (offline files, from Phase 12): a machine-bound [`LicenceV2`] payload
//!   delivered as a **licence key** (base64url of `payload ‖ signature`) or a
//!   `.vlic` file, minted by `tools/licence-maker`. See [`machine_code`],
//!   [`parse_licence_key`] and [`verify_v2`].
//!
//! Pure: no IO, no clock, no randomness (all inputs passed in), no floats.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::{CoreError, CoreResult};

/// Crockford base32 alphabet (no I, L, O, U) — used for the machine code (§10).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// ed25519 signatures are always exactly 64 bytes.
const SIG_LEN: usize = 64;

/// A signed, server-issued licence (docs §10). The JSON of this struct is what
/// the `cloud/licence` service signs with its ed25519 private key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Licence {
    pub licence_id: String,
    pub school_id: String,
    pub plan: String,
    /// `null` = unlimited students.
    pub max_students: Option<u32>,
    /// `null` = unlimited devices.
    pub max_devices: Option<u32>,
    pub issued_at: String,
    pub server_machine_id: String,
}

/// The answer from the licence service's `/v1/check` endpoint (docs §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckResult {
    Active,
    Revoked,
    Moved,
}

/// The most recent licence check. `result: None` means the service was
/// unreachable at the last attempt (perpetual model → licence stays active).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastCheck {
    pub result: Option<CheckResult>,
}

/// The effective licence status a device should act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenceStatus {
    Active,
    Revoked,
    Moved,
}

/// Verify a base64-encoded licence JSON against its base64 ed25519 signature.
///
/// Both `licence_b64_json` and `signature_b64` are STANDARD base64. The
/// signature is verified (with `verify_strict`) over the RAW decoded licence
/// bytes — the exact bytes the service signed, not a re-serialised form. Only on
/// a valid signature are those bytes parsed into a [`Licence`].
///
/// Any failure (bad base64, wrong signature length, wrong key, tampered bytes,
/// or malformed JSON) → [`CoreError::LicenceInvalid`].
pub fn verify(
    licence_b64_json: &str,
    signature_b64: &str,
    public_key: &[u8; 32],
) -> CoreResult<Licence> {
    // Decode the raw licence bytes and the signature.
    let licence_bytes = STANDARD
        .decode(licence_b64_json.as_bytes())
        .map_err(|_| CoreError::LicenceInvalid)?;
    let signature_bytes = STANDARD
        .decode(signature_b64.as_bytes())
        .map_err(|_| CoreError::LicenceInvalid)?;

    // Build the verifying key and signature (both length-checked here).
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| CoreError::LicenceInvalid)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| CoreError::LicenceInvalid)?;

    // Verify over the RAW decoded licence bytes.
    verifying_key
        .verify_strict(&licence_bytes, &signature)
        .map_err(|_| CoreError::LicenceInvalid)?;

    // Only now parse the JSON.
    serde_json::from_slice::<Licence>(&licence_bytes).map_err(|_| CoreError::LicenceInvalid)
}

/// Resolve the effective licence status from the last check (docs §10).
///
/// Perpetual model: an unreachable service (`result: None`) keeps the licence
/// **Active**; only an explicit `Revoked`/`Moved` answer changes the status.
pub fn status(check: LastCheck) -> LicenceStatus {
    match check.result {
        None | Some(CheckResult::Active) => LicenceStatus::Active,
        Some(CheckResult::Revoked) => LicenceStatus::Revoked,
        Some(CheckResult::Moved) => LicenceStatus::Moved,
    }
}

/// Enforce the licence's device/student caps (docs §10).
///
/// A `null` (`None`) cap means unlimited. If a non-null cap is exceeded the first
/// breach reported is students (then devices) → [`CoreError::LicenceLimit`].
pub fn check_limits(lic: &Licence, current_students: u32, current_devices: u32) -> CoreResult<()> {
    if let Some(max) = lic.max_students {
        if current_students > max {
            return Err(CoreError::LicenceLimit { what: "students".into() });
        }
    }
    if let Some(max) = lic.max_devices {
        if current_devices > max {
            return Err(CoreError::LicenceLimit { what: "devices".into() });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Licence v2 (offline files) — prompts/P12 Step 6, docs §10.
// ---------------------------------------------------------------------------

/// The signed payload of a v2 offline licence (docs §10). `tools/licence-maker`
/// produces the exact JSON bytes of this struct, signs them with ed25519, and
/// packs `payload ‖ signature` into the licence key / `.vlic` file.
///
/// The signature is always verified over the **raw** payload bytes (never a
/// re-serialisation), so the JSON field order the maker chose is irrelevant to
/// verification — [`verify_v2`] checks the signature first, then parses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceV2 {
    /// Format version — must be `2`.
    pub v: u8,
    pub licence_id: String,
    pub school_name: String,
    /// The buyer's machine code (`XXXX-XXXX-XXXX-C`), binding this licence to one PC.
    pub machine_code: String,
    pub issued_at: String,
    /// Always `"perpetual"`.
    pub plan: String,
    /// Enabled module keys, always beginning with `"core"`.
    pub modules: Vec<String>,
}

/// Derive the display machine code from the stable `machine_id` (docs §10):
/// `SHA-256(machine_id)` → first 60 bits → 12 Crockford base32 chars + a 1-char
/// checksum, shown as `XXXX-XXXX-XXXX-C` (e.g. `7KQ2-M9XD-4TRA-P`). Stable across
/// app updates because `machine_id` is stable in the OS keychain.
///
/// The checksum char is `CROCKFORD[(Σ data-char indices) mod 32]` — a simple,
/// deterministic guard against mistyping the code (recomputed identically by
/// `tools/licence-maker`, which cannot depend on this crate).
pub fn machine_code(machine_id: &str) -> String {
    let digest = Sha256::digest(machine_id.as_bytes());
    // Top 60 bits of the digest, big-endian.
    let mut first8 = [0u8; 8];
    first8.copy_from_slice(&digest[0..8]);
    let bits60 = u64::from_be_bytes(first8) >> 4;

    // 12 groups of 5 bits, most-significant first.
    let mut idx = [0usize; 12];
    let mut sum = 0usize;
    for (i, slot) in idx.iter_mut().enumerate() {
        let shift = 5 * (11 - i);
        let v = ((bits60 >> shift) & 0x1f) as usize;
        *slot = v;
        sum += v;
    }
    let check = CROCKFORD[sum % 32];

    let c = |g: usize, k: usize| CROCKFORD[idx[g * 4 + k]] as char;
    format!(
        "{}{}{}{}-{}{}{}{}-{}{}{}{}-{}",
        c(0, 0), c(0, 1), c(0, 2), c(0, 3),
        c(1, 0), c(1, 1), c(1, 2), c(1, 3),
        c(2, 0), c(2, 1), c(2, 2), c(2, 3),
        check as char,
    )
}

/// Crockford-normalise one character: uppercase, map the ambiguous `I/L → 1` and
/// `O → 0`; return its index in [`CROCKFORD`], or `None` if it is not a base32
/// digit (spaces and separators are handled by the caller).
fn crockford_value(ch: char) -> Option<usize> {
    let up = ch.to_ascii_uppercase();
    let mapped = match up {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    };
    CROCKFORD.iter().position(|&a| a as char == mapped)
}

/// Validate and canonicalise a typed machine code. Accepts any casing, optional
/// dashes/spaces and the Crockford look-alikes (`I/L→1`, `O→0`). Returns the
/// canonical **13-character** (12 data + checksum, no dashes) upper-case form on
/// success, or `None` if it is the wrong length, has a non-base32 character, or
/// the checksum does not match.
pub fn parse_machine_code(text: &str) -> Option<String> {
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
    Some(vals.iter().map(|&v| CROCKFORD[v] as char).collect())
}

/// Encode a v2 licence key: `base64url(payload ‖ signature)`, no padding. The UI
/// shows it in groups of 5 for pasting; [`parse_licence_key`] tolerates any
/// whitespace grouping. (Mirrored by `tools/licence-maker`.)
pub fn encode_licence_key(payload: &[u8], signature: &[u8]) -> String {
    let mut buf = Vec::with_capacity(payload.len() + signature.len());
    buf.extend_from_slice(payload);
    buf.extend_from_slice(signature);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Parse a pasted v2 licence key back into `(payload_bytes, signature_bytes)`.
///
/// Strips all whitespace (the display grouping), tolerates trailing `=` padding,
/// base64url-decodes, and splits off the trailing 64-byte ed25519 signature. Any
/// malformed input, or fewer than 64 bytes after the payload, →
/// [`CoreError::LicenceInvalid`].
pub fn parse_licence_key(text: &str) -> CoreResult<(Vec<u8>, Vec<u8>)> {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .collect();
    let bytes = URL_SAFE_NO_PAD
        .decode(compact.as_bytes())
        .map_err(|_| CoreError::LicenceInvalid)?;
    if bytes.len() <= SIG_LEN {
        return Err(CoreError::LicenceInvalid);
    }
    let split = bytes.len() - SIG_LEN;
    let payload = bytes[..split].to_vec();
    let signature = bytes[split..].to_vec();
    Ok((payload, signature))
}

/// Verify a v2 licence payload against its signature and this computer.
///
/// 1. ed25519 `verify_strict` over the **raw** `payload` bytes with `public_key`.
/// 2. parse the payload as [`LicenceV2`] and require `v == 2`.
/// 3. require the licence's `machine_code` to canonicalise to the same value as
///    `own_machine_code`.
///
/// Signature/parse/version failures → [`CoreError::LicenceInvalid`]; a valid
/// licence for a different PC → [`CoreError::LicenceOtherMachine`].
pub fn verify_v2(
    payload: &[u8],
    signature: &[u8],
    public_key: &[u8; 32],
    own_machine_code: &str,
) -> CoreResult<LicenceV2> {
    let verifying_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| CoreError::LicenceInvalid)?;
    let sig = Signature::from_slice(signature).map_err(|_| CoreError::LicenceInvalid)?;
    verifying_key
        .verify_strict(payload, &sig)
        .map_err(|_| CoreError::LicenceInvalid)?;

    let lic: LicenceV2 = serde_json::from_slice(payload).map_err(|_| CoreError::LicenceInvalid)?;
    if lic.v != 2 {
        return Err(CoreError::LicenceInvalid);
    }

    let want = parse_machine_code(own_machine_code).ok_or(CoreError::LicenceInvalid)?;
    match parse_machine_code(&lic.machine_code) {
        Some(got) if got == want => Ok(lic),
        _ => Err(CoreError::LicenceOtherMachine),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// A fixed 32-byte seed → deterministic keypair (no RNG needed; the
    /// `generate` helper is feature-gated and we only depend on the crate, not
    /// the `rand_core` feature).
    fn test_keypair(seed: u8) -> (SigningKey, [u8; 32]) {
        let secret = [seed; 32];
        let signing = SigningKey::from_bytes(&secret);
        let public = signing.verifying_key().to_bytes();
        (signing, public)
    }

    fn sample_licence() -> Licence {
        Licence {
            licence_id: "lic_123".into(),
            school_id: "sch_abc".into(),
            plan: "perpetual".into(),
            max_students: Some(200),
            max_devices: Some(5),
            issued_at: "2026-09-23T00:00:00Z".into(),
            server_machine_id: "machine_xyz".into(),
        }
    }

    /// Serialise, sign the RAW bytes, and base64-encode both — the exact shape
    /// the licence service produces.
    fn make_fixture(lic: &Licence, signing: &SigningKey) -> (String, String) {
        let raw = serde_json::to_vec(lic).unwrap();
        let sig = signing.sign(&raw);
        let lic_b64 = STANDARD.encode(&raw);
        let sig_b64 = STANDARD.encode(sig.to_bytes());
        (lic_b64, sig_b64)
    }

    #[test]
    fn good_signature_verifies() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        let got = verify(&lic_b64, &sig_b64, &public).unwrap();
        assert_eq!(got, lic);
    }

    #[test]
    fn tampered_licence_is_invalid() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        // Tamper with the licence bytes after signing: flip the plan.
        let mut raw = STANDARD.decode(&lic_b64).unwrap();
        // Flip one byte inside the JSON payload.
        raw[10] ^= 0x01;
        let tampered_b64 = STANDARD.encode(&raw);

        assert_eq!(
            verify(&tampered_b64, &sig_b64, &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn wrong_key_is_invalid() {
        let (signing, _public) = test_keypair(7);
        let (_other_signing, wrong_public) = test_keypair(9);
        let lic = sample_licence();
        let (lic_b64, sig_b64) = make_fixture(&lic, &signing);

        assert_eq!(
            verify(&lic_b64, &sig_b64, &wrong_public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn bad_base64_is_invalid() {
        let (_signing, public) = test_keypair(7);
        assert_eq!(
            verify("!!!not base64!!!", "also***bad", &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn wrong_signature_length_is_invalid() {
        let (signing, public) = test_keypair(7);
        let lic = sample_licence();
        let (lic_b64, _sig_b64) = make_fixture(&lic, &signing);
        // A short "signature" that base64-decodes but is not 64 bytes.
        let short_sig = STANDARD.encode([0u8; 10]);
        assert_eq!(
            verify(&lic_b64, &short_sig, &public),
            Err(CoreError::LicenceInvalid)
        );
    }

    #[test]
    fn status_unreachable_stays_active() {
        // Perpetual: no answer keeps the licence Active.
        assert_eq!(status(LastCheck { result: None }), LicenceStatus::Active);
    }

    #[test]
    fn status_explicit_answers() {
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Active) }),
            LicenceStatus::Active
        );
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Revoked) }),
            LicenceStatus::Revoked
        );
        assert_eq!(
            status(LastCheck { result: Some(CheckResult::Moved) }),
            LicenceStatus::Moved
        );
    }

    #[test]
    fn limits_null_is_unlimited() {
        let lic = Licence {
            max_students: None,
            max_devices: None,
            ..sample_licence()
        };
        // Huge counts are fine when caps are null.
        assert!(check_limits(&lic, u32::MAX, u32::MAX).is_ok());
    }

    #[test]
    fn limits_within_caps_ok() {
        let lic = sample_licence(); // 200 students, 5 devices
        assert!(check_limits(&lic, 200, 5).is_ok()); // exactly at cap
        assert!(check_limits(&lic, 199, 4).is_ok());
    }

    #[test]
    fn limits_over_students_errors() {
        let lic = sample_licence();
        assert_eq!(
            check_limits(&lic, 201, 5),
            Err(CoreError::LicenceLimit { what: "students".into() })
        );
    }

    #[test]
    fn limits_over_devices_errors() {
        let lic = sample_licence();
        assert_eq!(
            check_limits(&lic, 200, 6),
            Err(CoreError::LicenceLimit { what: "devices".into() })
        );
    }

    // ----- Licence v2 (offline) -----

    fn sample_v2(machine_code: &str) -> LicenceV2 {
        LicenceV2 {
            v: 2,
            licence_id: "lic_v2_1".into(),
            school_name: "Saraswati Public School".into(),
            machine_code: machine_code.into(),
            issued_at: "2026-09-25T00:00:00Z".into(),
            plan: "perpetual".into(),
            modules: vec!["core".into()],
        }
    }

    /// Mint a licence key exactly the way `tools/licence-maker` does.
    fn make_key_v2(lic: &LicenceV2, signing: &SigningKey) -> String {
        let payload = serde_json::to_vec(lic).unwrap();
        let sig = signing.sign(&payload);
        encode_licence_key(&payload, &sig.to_bytes())
    }

    #[test]
    fn machine_code_shape_and_checksum() {
        let mc = machine_code("machine-uuid-abc");
        // XXXX-XXXX-XXXX-C
        assert_eq!(mc.len(), 16);
        let groups: Vec<&str> = mc.split('-').collect();
        assert_eq!(groups.iter().map(|g| g.len()).collect::<Vec<_>>(), vec![4, 4, 4, 1]);
        // Deterministic.
        assert_eq!(mc, machine_code("machine-uuid-abc"));
        assert_ne!(mc, machine_code("machine-uuid-xyz"));
        // Round-trips through the validator (checksum is correct).
        assert!(parse_machine_code(&mc).is_some());
    }

    #[test]
    fn parse_machine_code_is_lenient_but_checks() {
        let mc = machine_code("some-machine");
        let canon = parse_machine_code(&mc).unwrap();
        // Lower-case, spaced, and Crockford look-alikes all normalise the same.
        let messy = mc.to_lowercase().replace('-', " ");
        assert_eq!(parse_machine_code(&messy), Some(canon.clone()));
        // Wrong length / junk / bad checksum are rejected.
        assert_eq!(parse_machine_code("SHORT"), None);
        assert_eq!(parse_machine_code("7KQ2-M9XD-4TRA-!"), None);
        // Flip the checksum char → rejected.
        let mut bad: Vec<char> = canon.chars().collect();
        bad[12] = if bad[12] == '0' { '1' } else { '0' };
        assert_eq!(parse_machine_code(&bad.into_iter().collect::<String>()), None);
    }

    #[test]
    fn licence_key_round_trips() {
        let (signing, _public) = test_keypair(3);
        let mc = machine_code("pc-1");
        let lic = sample_v2(&mc);
        let key = make_key_v2(&lic, &signing);
        // Add display grouping (spaces every 5 chars) — parsing must tolerate it.
        let grouped: String = key
            .as_bytes()
            .chunks(5)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        let (payload, sig) = parse_licence_key(&grouped).unwrap();
        assert_eq!(sig.len(), 64);
        assert_eq!(serde_json::from_slice::<LicenceV2>(&payload).unwrap(), lic);
    }

    #[test]
    fn verify_v2_good() {
        let (signing, public) = test_keypair(3);
        let mc = machine_code("pc-1");
        let lic = sample_v2(&mc);
        let key = make_key_v2(&lic, &signing);
        let (payload, sig) = parse_licence_key(&key).unwrap();
        // Verifies for this machine even when the code is typed lower-case/spaced.
        let typed = mc.to_lowercase();
        assert_eq!(verify_v2(&payload, &sig, &public, &typed).unwrap(), lic);
    }

    #[test]
    fn verify_v2_wrong_machine() {
        let (signing, public) = test_keypair(3);
        let lic = sample_v2(&machine_code("pc-1"));
        let key = make_key_v2(&lic, &signing);
        let (payload, sig) = parse_licence_key(&key).unwrap();
        let other = machine_code("pc-2");
        assert_eq!(
            verify_v2(&payload, &sig, &public, &other),
            Err(CoreError::LicenceOtherMachine)
        );
    }

    #[test]
    fn verify_v2_wrong_key_and_tamper_and_version() {
        let (signing, public) = test_keypair(3);
        let (_s2, wrong) = test_keypair(9);
        let mc = machine_code("pc-1");
        let lic = sample_v2(&mc);
        let key = make_key_v2(&lic, &signing);
        let (payload, sig) = parse_licence_key(&key).unwrap();

        // Wrong public key.
        assert_eq!(verify_v2(&payload, &sig, &wrong, &mc), Err(CoreError::LicenceInvalid));

        // Tampered payload (flip a byte) → signature fails.
        let mut bad_payload = payload.clone();
        bad_payload[5] ^= 0x01;
        assert_eq!(verify_v2(&bad_payload, &sig, &public, &mc), Err(CoreError::LicenceInvalid));

        // Wrong version → invalid (still correctly signed).
        let mut v1ish = sample_v2(&mc);
        v1ish.v = 1;
        let key2 = make_key_v2(&v1ish, &signing);
        let (p2, s2) = parse_licence_key(&key2).unwrap();
        assert_eq!(verify_v2(&p2, &s2, &public, &mc), Err(CoreError::LicenceInvalid));
    }

    #[test]
    fn parse_licence_key_rejects_garbage() {
        assert_eq!(parse_licence_key("****"), Err(CoreError::LicenceInvalid));
        // Decodes but is too short to hold a 64-byte signature.
        let tiny = URL_SAFE_NO_PAD.encode([0u8; 10]);
        assert_eq!(parse_licence_key(&tiny), Err(CoreError::LicenceInvalid));
    }

    /// Cross-crate golden vector: this exact key is produced by
    /// `tools/licence-maker` (its `golden_vector_is_stable` test pins the same
    /// string). It proves the maker and this crate agree byte-for-byte on the
    /// machine-code checksum, JSON payload and base64url key encoding. If either
    /// side's format changes, this test and the maker's must be updated together.
    #[test]
    fn golden_licence_key_from_maker_verifies() {
        // machine_id "golden-machine" → this code (also checked in the maker).
        let mc = machine_code("golden-machine");
        assert_eq!(mc, "3N0S-10YX-JQCR-T");

        // The maker signs with seed [42; 32]; derive its public key here.
        let (_signing, public) = test_keypair(42);
        const GOLDEN_KEY: &str = "eyJ2IjoyLCJsaWNlbmNlX2lkIjoiMDE5M2I2YzAtMDAwMC03MDAwLTgwMDAtMDAwMDAwMDAwMDAxIiwic2Nob29sX25hbWUiOiJTYXJhc3dhdGkgUHVibGljIFNjaG9vbCIsIm1hY2hpbmVfY29kZSI6IjNOMFMtMTBZWC1KUUNSLVQiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTI1VDAwOjAwOjAwWiIsInBsYW4iOiJwZXJwZXR1YWwiLCJtb2R1bGVzIjpbImNvcmUiXX0sel8zbFpx8xCuDxYhHoM9kZ6raBqbSkf-O3JKmdQ5t2OK9Z0ZYQO3RhjJdKu2gdWndFijhL48Powb0xSDAIoD";

        let (payload, sig) = parse_licence_key(GOLDEN_KEY).unwrap();
        let lic = verify_v2(&payload, &sig, &public, &mc).unwrap();
        assert_eq!(lic.v, 2);
        assert_eq!(lic.school_name, "Saraswati Public School");
        assert_eq!(lic.machine_code, "3N0S-10YX-JQCR-T");
        assert_eq!(lic.plan, "perpetual");
        assert_eq!(lic.modules, vec!["core".to_string()]);
    }
}
