//! The `.vop` Drive bundle format (§11, prompts/P06 Step 4/5).
//!
//! A bundle carries a group of ops that all share one audience. Its body is
//! `nonce(12) ‖ ChaCha20-Poly1305(audience_key, JSON array of ops)` — the exact
//! envelope §11 specifies, reusing the vetted primitives in [`crate::sync::seal`].
//! The associated data binds the audience and key version, so a bundle cannot be
//! relabelled to a different audience/version folder without failing the open.
//!
//! Filename: `<hlc>-<audience>-v<keyver>.vop` (§11). Because an HLC and a class
//! audience both contain `-` and `:` (ids are UUIDs), the filename is **not**
//! parsed for routing; the audience + key version travel in the Drive file's
//! `properties` so a reader can pick the right key without downloading (and
//! whether other `drive.file` users can read those properties is a Spike-A item —
//! see the handoff). The name stays human-readable and chronologically sortable.

use crate::sync::protocol::Op;
use crate::sync::seal::{self, SealError};
use std::collections::BTreeMap;

/// Max ops in one bundle (§11).
pub const MAX_OPS_PER_BUNDLE: usize = 500;
/// Max bundle body size (§11: 1 MB, decimal).
pub const MAX_BUNDLE_BYTES: usize = 1_000_000;
/// Headroom left below [`MAX_BUNDLE_BYTES`] for the nonce + AEAD tag while packing.
const PACK_BUDGET_BYTES: usize = MAX_BUNDLE_BYTES - 1_024;

/// Property keys carried on the Drive file (see module note).
pub const PROP_AUDIENCE: &str = "vidya_audience";
pub const PROP_KEYVER: &str = "vidya_keyver";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BundleError {
    #[error("seal/open failed")]
    Seal(SealError),
    #[error("bundle json: {0}")]
    Json(String),
    #[error("bundle too large even as a single op")]
    OpTooLarge,
    #[error("mixed audiences in one bundle")]
    MixedAudience,
}

/// Associated data bound into the bundle AEAD. A change to the audience or key
/// version fails the open.
pub fn bundle_aad(audience: &str, key_version: i64) -> Vec<u8> {
    format!("vidya/vop/v1\n{audience}\n{key_version}").into_bytes()
}

/// The §11 filename for a bundle.
pub fn bundle_filename(hlc: &str, audience: &str, key_version: i64) -> String {
    format!("{hlc}-{audience}-v{key_version}.vop")
}

/// The Drive `properties` a bundle file carries (audience + key version).
pub fn bundle_properties(audience: &str, key_version: i64) -> BTreeMap<String, String> {
    let mut p = BTreeMap::new();
    p.insert(PROP_AUDIENCE.to_string(), audience.to_string());
    p.insert(PROP_KEYVER.to_string(), key_version.to_string());
    p
}

/// Seal one group of same-audience ops into a `.vop` body.
pub fn seal_bundle(
    audience_key: &[u8; 32],
    audience: &str,
    key_version: i64,
    ops: &[Op],
) -> Result<Vec<u8>, BundleError> {
    if ops.iter().any(|o| o.audience != audience) {
        return Err(BundleError::MixedAudience);
    }
    let json = serde_json::to_vec(ops).map_err(|e| BundleError::Json(e.to_string()))?;
    let aad = bundle_aad(audience, key_version);
    Ok(seal::seal(audience_key, &aad, &json))
}

/// Open a `.vop` body back into its ops. An AEAD failure means a tampered bundle,
/// the wrong key, or the wrong audience/version label (all indistinguishable).
pub fn open_bundle(
    audience_key: &[u8; 32],
    audience: &str,
    key_version: i64,
    body: &[u8],
) -> Result<Vec<Op>, BundleError> {
    let aad = bundle_aad(audience, key_version);
    let json = seal::open(audience_key, &aad, body).map_err(BundleError::Seal)?;
    serde_json::from_slice(&json).map_err(|e| BundleError::Json(e.to_string()))
}

/// Split ops (already all one audience) into bundle-sized chunks: at most
/// [`MAX_OPS_PER_BUNDLE`] ops and roughly [`MAX_BUNDLE_BYTES`] of serialized
/// payload each. Chunking is by serialized JSON length (a safe over-estimate of
/// the ciphertext, which adds only nonce + tag). One op that alone exceeds the
/// budget is an error rather than a silently-oversized bundle.
pub fn chunk_ops(ops: &[Op]) -> Result<Vec<Vec<Op>>, BundleError> {
    let mut chunks: Vec<Vec<Op>> = Vec::new();
    let mut cur: Vec<Op> = Vec::new();
    let mut cur_bytes = 2usize; // the enclosing "[]"

    for op in ops {
        let op_bytes = serde_json::to_vec(op)
            .map_err(|e| BundleError::Json(e.to_string()))?
            .len()
            + 1; // trailing comma / separator
        if op_bytes + 2 > PACK_BUDGET_BYTES {
            return Err(BundleError::OpTooLarge);
        }
        let would_overflow =
            cur.len() >= MAX_OPS_PER_BUNDLE || cur_bytes + op_bytes > PACK_BUDGET_BYTES;
        if would_overflow && !cur.is_empty() {
            chunks.push(std::mem::take(&mut cur));
            cur_bytes = 2;
        }
        cur.push(op.clone());
        cur_bytes += op_bytes;
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn key() -> [u8; 32] {
        [0x5au8; 32]
    }

    fn op(n: usize, audience: &str) -> Op {
        Op {
            op_id: format!("op-{n}"),
            hlc: format!("00000000000000000{n:03}dev"),
            device_id: "dev".into(),
            staff_id: "st1".into(),
            audience: audience.to_string(),
            table: "attendance_mark".into(),
            record_id: format!("m-{n}"),
            kind: "update".into(),
            payload: json!({ "mark": "P" }),
            base_version: Some(1),
            server_epoch: 1,
        }
    }

    #[test]
    fn seals_and_opens_round_trip() {
        let ops = vec![op(1, "class:c1"), op(2, "class:c1")];
        let body = seal_bundle(&key(), "class:c1", 3, &ops).unwrap();
        let back = open_bundle(&key(), "class:c1", 3, &body).unwrap();
        assert_eq!(back, ops);
    }

    #[test]
    fn tampering_one_byte_is_rejected() {
        // Step 9: a tampered bundle must fail to open (→ quarantine).
        let ops = vec![op(1, "class:c1")];
        let mut body = seal_bundle(&key(), "class:c1", 1, &ops).unwrap();
        let last = body.len() - 1;
        body[last] ^= 0x01;
        assert!(matches!(
            open_bundle(&key(), "class:c1", 1, &body),
            Err(BundleError::Seal(_))
        ));
    }

    #[test]
    fn wrong_audience_key_cannot_open() {
        // A teacher V-A key cannot open a class:VI-B bundle (§8.8 test).
        let ops = vec![op(1, "class:VI-B")];
        let body = seal_bundle(&[0x11u8; 32], "class:VI-B", 1, &ops).unwrap();
        assert!(open_bundle(&[0x22u8; 32], "class:VI-B", 1, &body).is_err());
    }

    #[test]
    fn relabelling_audience_or_version_fails_open() {
        let ops = vec![op(1, "class:c1")];
        let body = seal_bundle(&key(), "class:c1", 1, &ops).unwrap();
        // same key, but a reader that believes it is a different audience/version
        // (the AAD differs) cannot open it.
        assert!(open_bundle(&key(), "finance", 1, &body).is_err());
        assert!(open_bundle(&key(), "class:c1", 2, &body).is_err());
    }

    #[test]
    fn mixed_audiences_are_refused() {
        let ops = vec![op(1, "class:c1"), op(2, "finance")];
        assert_eq!(
            seal_bundle(&key(), "class:c1", 1, &ops),
            Err(BundleError::MixedAudience)
        );
    }

    #[test]
    fn chunks_respect_the_500_op_cap() {
        let ops: Vec<Op> = (0..1201).map(|n| op(n, "class:c1")).collect();
        let chunks = chunk_ops(&ops).unwrap();
        assert_eq!(chunks.len(), 3); // 500 + 500 + 201
        assert!(chunks.iter().all(|c| c.len() <= MAX_OPS_PER_BUNDLE));
        assert_eq!(chunks.iter().map(|c| c.len()).sum::<usize>(), 1201);
        // order preserved
        assert_eq!(chunks[0][0].op_id, "op-0");
        assert_eq!(chunks[2].last().unwrap().op_id, "op-1200");
    }

    #[test]
    fn chunks_respect_the_size_cap() {
        // Big payloads → each chunk's sealed body stays under the 1 MB cap.
        let big = "x".repeat(20_000);
        let ops: Vec<Op> = (0..120)
            .map(|n| {
                let mut o = op(n, "class:c1");
                o.payload = json!({ "blob": big });
                o
            })
            .collect();
        let chunks = chunk_ops(&ops).unwrap();
        assert!(chunks.len() > 1, "20k×120 ≈ 2.4 MB must split");
        for c in &chunks {
            let body = seal_bundle(&key(), "class:c1", 1, c).unwrap();
            assert!(body.len() <= MAX_BUNDLE_BYTES, "sealed {} bytes", body.len());
        }
    }

    #[test]
    fn filename_and_properties_are_consistent() {
        assert_eq!(
            bundle_filename("00000000000000000001dev", "class:c1", 4),
            "00000000000000000001dev-class:c1-v4.vop"
        );
        let p = bundle_properties("finance", 2);
        assert_eq!(p.get(PROP_AUDIENCE).unwrap(), "finance");
        assert_eq!(p.get(PROP_KEYVER).unwrap(), "2");
    }
}
