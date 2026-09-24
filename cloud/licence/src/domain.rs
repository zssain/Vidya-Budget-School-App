//! The business core: issue a licence for a paid order, activate a code, check
//! status, and transfer to a new machine. Pure over a `&Connection` + config +
//! signing key (no HTTP, no clock beyond `now_iso`), so it is unit-testable and
//! re-checked identically wherever it is called (rule 9 spirit).

use ed25519_dalek::{Signer, SigningKey};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::audit;
use crate::config::Config;
use crate::crypto;
use crate::db::{new_id, now_iso};

/// The exact §10 licence JSON that gets signed. Field set + order are fixed by the
/// contract; the app verifies the signature over these bytes, then parses them.
#[derive(Debug, Clone, Serialize)]
pub struct LicenceJson {
    pub licence_id: String,
    pub school_id: String,
    pub plan: String,
    pub max_students: Option<u32>,
    pub max_devices: Option<u32>,
    pub issued_at: String,
    pub server_machine_id: String,
}

/// A successful activate/transfer response payload.
pub struct Issued {
    pub licence_b64: String,
    pub signature_b64: String,
    pub relay_secret: String,
}

/// Business errors, each mapped to a generic HTTP answer by the API layer.
#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    NotFound,          // 404 CODE_NOT_FOUND
    AlreadyUsed,       // 409 CODE_ALREADY_USED
    Revoked,           // 403 REVOKED
    ProofInvalid,      // 403 PROOF_INVALID
    TransferUnavailable, // 403 TRANSFER_UNAVAILABLE
}

/// What issuing produced (shown once to the admin; re-viewable via the code store).
#[derive(Debug)]
pub struct IssueResult {
    pub licence_id: String,
    pub school_id: String,
    pub code: String,
    pub already_issued: bool,
}

// --- helpers ---------------------------------------------------------------

pub fn sign_licence(signing: &SigningKey, lic: &LicenceJson) -> (String, String) {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let raw = serde_json::to_vec(lic).expect("serialize licence");
    let sig = signing.sign(&raw);
    (STANDARD.encode(&raw), STANDARD.encode(sig.to_bytes()))
}

struct LicenceRow {
    licence_id: String,
    school_id: String,
    plan: String,
    max_students: Option<u32>,
    max_devices: Option<u32>,
    status: String,
    issued_at: String,
    server_machine_id: Option<String>,
    recovery_verifier_enc: Option<String>,
}

fn load_licence(conn: &Connection, licence_id: &str) -> rusqlite::Result<Option<LicenceRow>> {
    conn.query_row(
        "SELECT licence_id, school_id, plan, max_students, max_devices, status, issued_at,
                server_machine_id, recovery_verifier_enc
           FROM licence WHERE licence_id = ?1",
        [licence_id],
        |r| {
            Ok(LicenceRow {
                licence_id: r.get(0)?,
                school_id: r.get(1)?,
                plan: r.get(2)?,
                max_students: r.get(3)?,
                max_devices: r.get(4)?,
                status: r.get(5)?,
                issued_at: r.get(6)?,
                server_machine_id: r.get(7)?,
                recovery_verifier_enc: r.get(8)?,
            })
        },
    )
    .optional()
}

fn issued_for(row: &LicenceRow, cfg: &Config, signing: &SigningKey, machine_id: &str) -> Issued {
    let lic = LicenceJson {
        licence_id: row.licence_id.clone(),
        school_id: row.school_id.clone(),
        plan: row.plan.clone(),
        max_students: row.max_students,
        max_devices: row.max_devices,
        issued_at: row.issued_at.clone(),
        server_machine_id: machine_id.to_string(),
    };
    let (licence_b64, signature_b64) = sign_licence(signing, &lic);
    Issued {
        licence_b64,
        signature_b64,
        relay_secret: crypto::relay_secret(&cfg.relay_shared_key, &row.school_id),
    }
}

// --- issuing (called from the admin "verify payment" action) ---------------

/// Verify a payment and issue exactly one licence + activation code for an order,
/// in one transaction. Idempotent: a second verify of the same order returns the
/// SAME code (never a second licence).
pub fn verify_payment_and_issue(
    conn: &mut Connection,
    cfg: &Config,
    admin_email: &str,
    order_id: &str,
    amount_paise: i64,
    upi_ref: Option<&str>,
    reason: &str,
) -> rusqlite::Result<Result<IssueResult, DomainError>> {
    let tx = conn.transaction()?;

    // Order must exist.
    let order: Option<(String, String)> = tx
        .query_row(
            "SELECT id, status FROM purchase_order WHERE id = ?1 OR provider_order_id = ?1",
            [order_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    let (oid, _status) = match order {
        Some(o) => o,
        None => return Ok(Err(DomainError::NotFound)),
    };

    // Idempotency: if a licence already exists for this order, return its code.
    let existing: Option<(String, String)> = tx
        .query_row(
            "SELECT l.licence_id, l.school_id FROM licence l WHERE l.order_id = ?1",
            [&oid],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    if let Some((licence_id, school_id)) = existing {
        let code_enc: String = tx.query_row(
            "SELECT code_enc FROM activation_code WHERE order_id = ?1 AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [&oid],
            |r| r.get(0),
        )?;
        let code = crypto::decrypt(&cfg.code_enc_key, &code_enc)
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default();
        tx.commit()?;
        return Ok(Ok(IssueResult { licence_id, school_id, code, already_issued: true }));
    }

    // Record the admin-verified payment (idempotent by provider_event_id).
    let now = now_iso();
    tx.execute(
        "INSERT OR IGNORE INTO payment_event
           (id, provider_event_id, order_id, kind, amount_paise, upi_ref, actor, verified, raw_json, received_at)
         VALUES (?1, ?2, ?3, 'admin_verify', ?4, ?5, ?6, 1, ?7, ?8)",
        params![
            new_id(),
            format!("verify:{oid}"),
            oid,
            amount_paise,
            upi_ref,
            admin_email,
            serde_json::json!({ "amount_paise": amount_paise, "upi_ref": upi_ref }).to_string(),
            now,
        ],
    )?;

    // Create licence + code.
    let licence_id = format!("lic_{}", new_id());
    let school_id = format!("sch_{}", new_id());
    tx.execute(
        "INSERT INTO licence
           (licence_id, school_id, order_id, plan, max_students, max_devices, status, issued_at,
            server_machine_id, server_epoch, created_at)
         VALUES (?1, ?2, ?3, 'perpetual', NULL, NULL, 'active', ?4, NULL, 1, ?4)",
        params![licence_id, school_id, oid, now],
    )?;

    let code = crypto::generate_code();
    tx.execute(
        "INSERT INTO activation_code (id, code_hash, code_enc, licence_id, order_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new_id(),
            crypto::hash_code(&code),
            crypto::encrypt(&cfg.code_enc_key, code.as_bytes()),
            licence_id,
            oid,
            now,
        ],
    )?;

    // Record the relay-secret issuance (see the table note re: rotation).
    tx.execute(
        "INSERT INTO relay_secret (id, licence_id, secret_hash, generation, active, created_at)
         VALUES (?1, ?2, ?3, 1, 1, ?4)",
        params![
            new_id(),
            licence_id,
            crypto::sha256_hex(crypto::relay_secret(&cfg.relay_shared_key, &school_id).as_bytes()),
            now,
        ],
    )?;

    tx.execute("UPDATE purchase_order SET status = 'paid', updated_at = ?2 WHERE id = ?1", params![oid, now])?;

    audit::append(
        &tx,
        admin_email,
        "verify_payment_issue_licence",
        Some("order"),
        Some(&oid),
        Some(reason),
        serde_json::json!({ "licence_id": licence_id, "amount_paise": amount_paise, "upi_ref": upi_ref }),
    )?;

    tx.commit()?;
    Ok(Ok(IssueResult { licence_id, school_id, code, already_issued: false }))
}

// --- activation ------------------------------------------------------------

/// `/v1/activate`: bind a valid, unredeemed code to a machine and return the
/// signed licence. Idempotent for the machine currently bound; `409` for another.
pub fn activate(
    conn: &mut Connection,
    cfg: &Config,
    signing: &SigningKey,
    code: &str,
    machine_id: &str,
    app_version: &str,
    recovery_verifier_b64: Option<&str>,
) -> rusqlite::Result<Result<Issued, DomainError>> {
    let tx = conn.transaction()?;
    let code_hash = crypto::hash_code(code);

    let found: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT licence_id, revoked_at FROM activation_code WHERE code_hash = ?1",
            [&code_hash],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let (licence_id, revoked_at) = match found {
        Some(f) => f,
        None => return Ok(Err(DomainError::NotFound)),
    };
    if revoked_at.is_some() {
        // A reissued (revoked) code is treated as not-found — the buyer must use
        // the new code (generic, does not leak that it once existed).
        return Ok(Err(DomainError::NotFound));
    }

    let row = match load_licence(&tx, &licence_id)? {
        Some(r) => r,
        None => return Ok(Err(DomainError::NotFound)),
    };
    if row.status == "revoked" {
        return Ok(Err(DomainError::Revoked));
    }

    let now = now_iso();
    match &row.server_machine_id {
        // Already bound: same machine → idempotent; another → 409.
        Some(bound) if bound == machine_id => {
            tx.execute(
                "UPDATE licence SET last_check_at = ?2, app_version_last_seen = ?3 WHERE licence_id = ?1",
                params![licence_id, now, app_version],
            )?;
            let out = issued_for(&row, cfg, signing, machine_id);
            tx.commit()?;
            Ok(Ok(out))
        }
        Some(_) => Ok(Err(DomainError::AlreadyUsed)),
        // First activation: bind now.
        None => {
            let verifier_enc = recovery_verifier_b64
                .and_then(crypto::decode_key32)
                .map(|k| crypto::encrypt(&cfg.code_enc_key, &k));
            tx.execute(
                "UPDATE licence
                    SET server_machine_id = ?2, last_check_at = ?3, app_version_last_seen = ?4,
                        recovery_verifier_enc = COALESCE(?5, recovery_verifier_enc)
                  WHERE licence_id = ?1",
                params![licence_id, machine_id, now, app_version, verifier_enc],
            )?;
            tx.execute(
                "UPDATE activation_code SET redeemed_at = ?2, redeemed_machine_id = ?3 WHERE code_hash = ?1",
                params![code_hash, now, machine_id],
            )?;
            let out = issued_for(&row, cfg, signing, machine_id);
            tx.commit()?;
            Ok(Ok(out))
        }
    }
}

// --- status check ----------------------------------------------------------

/// `/v1/check`: report `active | revoked | moved` for a licence + machine.
pub fn check(conn: &Connection, licence_id: &str, machine_id: &str, app_version: &str) -> rusqlite::Result<Option<String>> {
    let row = match load_licence(conn, licence_id)? {
        Some(r) => r,
        None => return Ok(None),
    };
    let status = if row.status == "revoked" {
        "revoked"
    } else if let Some(bound) = &row.server_machine_id {
        if !machine_id.is_empty() && bound != machine_id {
            "moved"
        } else {
            "active"
        }
    } else {
        "active"
    };
    let now = now_iso();
    conn.execute(
        "UPDATE licence SET last_check_at = ?2, app_version_last_seen = COALESCE(NULLIF(?3,''), app_version_last_seen) WHERE licence_id = ?1",
        params![licence_id, now, app_version],
    )?;
    Ok(Some(status.to_string()))
}

// --- transfer --------------------------------------------------------------

/// `/v1/transfer`: rebind a licence to a new machine and return the signed
/// licence. If `new_machine_id` is already the bound machine, this is idempotent
/// and needs no proof (also the delivery path after an admin-approved rebind).
/// Otherwise a valid `recovery_proof` (HMAC over the transfer message with the
/// app-registered verifier key) is required. `server_epoch + 1`; the old machine
/// then reports `moved` on its next `/v1/check`.
#[allow(clippy::too_many_arguments)]
pub fn transfer(
    conn: &mut Connection,
    cfg: &Config,
    signing: &SigningKey,
    licence_id: &str,
    new_machine_id: &str,
    recovery_proof: &str,
    timestamp: &str,
    now_unix: i64,
) -> rusqlite::Result<Result<Issued, DomainError>> {
    let tx = conn.transaction()?;
    let row = match load_licence(&tx, licence_id)? {
        Some(r) => r,
        None => return Ok(Err(DomainError::NotFound)),
    };
    if row.status == "revoked" {
        return Ok(Err(DomainError::Revoked));
    }

    // Idempotent / post-admin-approval delivery: already the bound machine.
    if row.server_machine_id.as_deref() == Some(new_machine_id) {
        let out = issued_for(&row, cfg, signing, new_machine_id);
        tx.commit()?;
        return Ok(Ok(out));
    }

    // Self-service: require a fresh, valid recovery proof.
    let verifier_enc = match &row.recovery_verifier_enc {
        Some(v) => v,
        None => return Ok(Err(DomainError::TransferUnavailable)),
    };
    let tvk = match crypto::decrypt(&cfg.code_enc_key, verifier_enc) {
        Some(k) if k.len() == 32 => k,
        _ => return Ok(Err(DomainError::TransferUnavailable)),
    };
    // Timestamp must be recent (±10 min) to bound replay.
    match parse_unix(timestamp) {
        Some(ts) if (now_unix - ts).abs() <= 600 => {}
        _ => return Ok(Err(DomainError::ProofInvalid)),
    }
    let expected = crypto::transfer_proof(&tvk, licence_id, new_machine_id, timestamp);
    if !crypto::ct_eq(expected.as_bytes(), recovery_proof.as_bytes()) {
        return Ok(Err(DomainError::ProofInvalid));
    }

    rebind(&tx, &row, new_machine_id, "self_service", None, None)?;
    let out = issued_for(&row, cfg, signing, new_machine_id);
    tx.commit()?;
    Ok(Ok(out))
}

/// Admin-approved rebind (support case): no recovery proof; audited with a reason.
/// The new PC then fetches its signed licence via the idempotent `/v1/transfer`
/// branch above.
pub fn transfer_admin(
    conn: &mut Connection,
    licence_id: &str,
    new_machine_id: &str,
    admin_email: &str,
    reason: &str,
) -> rusqlite::Result<Result<(), DomainError>> {
    let tx = conn.transaction()?;
    let row = match load_licence(&tx, licence_id)? {
        Some(r) => r,
        None => return Ok(Err(DomainError::NotFound)),
    };
    if row.status == "revoked" {
        return Ok(Err(DomainError::Revoked));
    }
    rebind(&tx, &row, new_machine_id, "admin", Some(admin_email), Some(reason))?;
    audit::append(
        &tx,
        admin_email,
        "approve_transfer",
        Some("licence"),
        Some(licence_id),
        Some(reason),
        serde_json::json!({ "new_machine_id": new_machine_id, "old_machine_id": row.server_machine_id }),
    )?;
    tx.commit()?;
    Ok(Ok(()))
}

fn rebind(
    tx: &Connection,
    row: &LicenceRow,
    new_machine_id: &str,
    method: &str,
    admin_email: Option<&str>,
    reason: Option<&str>,
) -> rusqlite::Result<()> {
    let now = now_iso();
    tx.execute(
        "UPDATE licence SET server_machine_id = ?2, server_epoch = server_epoch + 1 WHERE licence_id = ?1",
        params![row.licence_id, new_machine_id],
    )?;
    tx.execute(
        "INSERT INTO transfer (id, licence_id, old_machine_id, new_machine_id, at, method, admin_email, reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![new_id(), row.licence_id, row.server_machine_id, new_machine_id, now, method, admin_email, reason],
    )?;
    Ok(())
}

fn parse_unix(ts: &str) -> Option<i64> {
    time::OffsetDateTime::parse(ts, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp())
}
