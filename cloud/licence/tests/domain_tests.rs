//! Domain-level Step-8 tests (prompts/P10 Step 8): issue idempotency, activation
//! binding + 409, transfer proof + `moved`/epoch, revoke. Drive `domain` over an
//! in-memory DB — no HTTP.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use rusqlite::{params, Connection};
use vidya_licence::config::Config;
use vidya_licence::domain::{self, DomainError, LicenceJson};
use vidya_licence::{crypto, db};

fn fresh() -> (Connection, Config, SigningKey) {
    (db::open_memory().unwrap(), Config::test_default(), SigningKey::from_bytes(&[7u8; 32]))
}

fn make_order(conn: &Connection, order_ref: &str) {
    let now = db::now_iso();
    let cust = db::new_id();
    conn.execute(
        "INSERT INTO customer (id,email,name,phone,school_name,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![cust, "a@school.test", "A", "9800000000", "Test School", now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO purchase_order (id,customer_id,provider,provider_order_id,amount_paise,currency,plan,status,created_at,updated_at)
         VALUES (?1,?2,'upi_manual',?3,0,'INR','perpetual','awaiting_verification',?4,?4)",
        params![db::new_id(), cust, order_ref, now],
    )
    .unwrap();
}

fn issue(conn: &mut Connection, cfg: &Config, order_ref: &str) -> (String, String) {
    make_order(conn, order_ref);
    let r = domain::verify_payment_and_issue(conn, cfg, "admin@x", order_ref, 500000, Some("UTR1"), "verified")
        .unwrap()
        .unwrap();
    let licence_id: String =
        conn.query_row("SELECT licence_id FROM licence WHERE order_id=(SELECT id FROM purchase_order WHERE provider_order_id=?1)", [order_ref], |r| r.get(0)).unwrap();
    (r.code, licence_id)
}

#[test]
fn verify_is_idempotent_one_licence_even_if_verified_twice() {
    let (mut conn, cfg, _) = fresh();
    make_order(&conn, "ORD1");
    let r1 = domain::verify_payment_and_issue(&mut conn, &cfg, "admin@x", "ORD1", 500000, Some("UTR1"), "verified").unwrap().unwrap();
    assert!(!r1.already_issued);
    let r2 = domain::verify_payment_and_issue(&mut conn, &cfg, "admin@x", "ORD1", 500000, Some("UTR1"), "again").unwrap().unwrap();
    assert!(r2.already_issued, "second verify returns the existing code");
    assert_eq!(r1.code, r2.code);
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM licence", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1, "never a second licence");
}

#[test]
fn buyer_awaiting_verification_has_no_licence() {
    // A recorded buyer claim (awaiting_verification) NEVER issues on its own.
    let (conn, _cfg, _) = fresh();
    make_order(&conn, "ORD2");
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM licence", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn activate_binds_then_same_machine_idempotent_other_machine_409() {
    let (mut conn, cfg, sk) = fresh();
    let (code, _lid) = issue(&mut conn, &cfg, "ORD3");

    let a = domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap().unwrap();
    // Retry on the same machine → byte-identical licence (deterministic signing).
    let a2 = domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap().unwrap();
    assert_eq!(a.licence_b64, a2.licence_b64);
    assert_eq!(a.signature_b64, a2.signature_b64);
    // A different machine → 409 CODE_ALREADY_USED.
    let other = domain::activate(&mut conn, &cfg, &sk, &code, "machineB", "1.0.0", None).unwrap();
    assert_eq!(other.err(), Some(DomainError::AlreadyUsed));
}

#[test]
fn unknown_code_is_not_found() {
    let (mut conn, cfg, sk) = fresh();
    let r = domain::activate(&mut conn, &cfg, &sk, "VIDYA-AAAA-AAAA-AAAA", "m", "1.0.0", None).unwrap();
    assert_eq!(r.err(), Some(DomainError::NotFound));
}

#[test]
fn self_service_transfer_moves_old_machine_and_bumps_epoch() {
    let (mut conn, cfg, sk) = fresh();
    let (code, lid) = issue(&mut conn, &cfg, "ORD4");

    // Activate on A, registering the recovery verifier.
    let tvk = [9u8; 32];
    let verifier_b64 = STANDARD.encode(tvk);
    domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", Some(&verifier_b64)).unwrap().unwrap();

    // Valid proof → transfer to B.
    let ts = db::now_iso();
    let now_unix = time::OffsetDateTime::now_utc().unix_timestamp();
    let proof = crypto::transfer_proof(&tvk, &lid, "machineB", &ts);
    domain::transfer(&mut conn, &cfg, &sk, &lid, "machineB", &proof, &ts, now_unix).unwrap().unwrap();

    assert_eq!(domain::check(&conn, &lid, "machineA", "").unwrap().as_deref(), Some("moved"));
    assert_eq!(domain::check(&conn, &lid, "machineB", "").unwrap().as_deref(), Some("active"));
    let epoch: i64 = conn.query_row("SELECT server_epoch FROM licence WHERE licence_id=?1", [&lid], |r| r.get(0)).unwrap();
    assert_eq!(epoch, 2);

    // A bad proof is rejected.
    let bad = domain::transfer(&mut conn, &cfg, &sk, &lid, "machineC", "not-a-proof", &ts, now_unix).unwrap();
    assert_eq!(bad.err(), Some(DomainError::ProofInvalid));
}

#[test]
fn transfer_without_registered_verifier_is_unavailable() {
    let (mut conn, cfg, sk) = fresh();
    let (code, lid) = issue(&mut conn, &cfg, "ORD5");
    domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap().unwrap();
    let ts = db::now_iso();
    let now_unix = time::OffsetDateTime::now_utc().unix_timestamp();
    let r = domain::transfer(&mut conn, &cfg, &sk, &lid, "machineB", "x", &ts, now_unix).unwrap();
    assert_eq!(r.err(), Some(DomainError::TransferUnavailable));
}

#[test]
fn admin_transfer_rebinds_and_old_machine_moves() {
    let (mut conn, cfg, sk) = fresh();
    let (code, lid) = issue(&mut conn, &cfg, "ORD6");
    domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap().unwrap();
    domain::transfer_admin(&mut conn, &lid, "machineB", "admin@x", "support case").unwrap().unwrap();
    assert_eq!(domain::check(&conn, &lid, "machineA", "").unwrap().as_deref(), Some("moved"));
    // The new PC fetches its signed licence via the idempotent transfer branch (no proof).
    let ts = db::now_iso();
    let now_unix = time::OffsetDateTime::now_utc().unix_timestamp();
    let issued = domain::transfer(&mut conn, &cfg, &sk, &lid, "machineB", "", &ts, now_unix).unwrap();
    assert!(issued.is_ok());
}

#[test]
fn revoked_licence_blocks_activation_and_reports_revoked() {
    let (mut conn, cfg, sk) = fresh();
    let (code, lid) = issue(&mut conn, &cfg, "ORD7");
    domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap().unwrap();
    conn.execute("UPDATE licence SET status='revoked' WHERE licence_id=?1", [&lid]).unwrap();
    let r = domain::activate(&mut conn, &cfg, &sk, &code, "machineA", "1.0.0", None).unwrap();
    assert_eq!(r.err(), Some(DomainError::Revoked));
    assert_eq!(domain::check(&conn, &lid, "machineA", "").unwrap().as_deref(), Some("revoked"));
}

#[test]
fn signed_licence_verifies_with_public_key() {
    let sk = SigningKey::from_bytes(&[5u8; 32]);
    let lic = LicenceJson {
        licence_id: "lic_1".into(),
        school_id: "sch_1".into(),
        plan: "perpetual".into(),
        max_students: None,
        max_devices: None,
        issued_at: "2026-09-24T00:00:00Z".into(),
        server_machine_id: "m1".into(),
    };
    let (lic_b64, sig_b64) = domain::sign_licence(&sk, &lic);
    let raw = STANDARD.decode(lic_b64).unwrap();
    let sig = Signature::from_slice(&STANDARD.decode(sig_b64).unwrap()).unwrap();
    let vk = VerifyingKey::from_bytes(&sk.verifying_key().to_bytes()).unwrap();
    assert!(vk.verify(&raw, &sig).is_ok());
    let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(v["plan"], "perpetual");
    assert_eq!(v["server_machine_id"], "m1");
    assert_eq!(v["max_students"], serde_json::Value::Null);
}
