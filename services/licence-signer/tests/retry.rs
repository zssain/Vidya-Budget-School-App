//! Owner decisions 2026-11-05, end to end against real Postgres with the real
//! migrations (0001–0003) and the production roles. TEST KEYS ONLY.
//!
//! (a) crash after signing but before the job is marked complete → retry →
//!     one paid order, one issued licence, one downloadable key, same licence_id;
//! (b) signer failure → issuance `failed`, order still paid ("Paid · Licence
//!     pending") → owner Retry issuance → issued with the ORIGINAL licence_id and
//!     byte-identical payload; every failed attempt stays audited;
//! (c) email failure after issuance → Resend reads the identical stored artefact;
//!     still one licence; the artefact can never change.
//!
//! Every artefact is checked with the app's verifier `vidya_core::licence::verify_v2`
//! (via `parse_licence_key` on the stored `.vlic`), like tests/db.rs and golden.rs.

mod common;

use common::{
    audit_count, audit_count_any, expire_lease, issuance, job, order_summary, portal_retry_issuance, seed_issue,
    SeedSpec, TestDb,
};
use licence_signer::worker::{
    claim_job, commit, prepare, process_one, record_failure, recover_expired_leases, JobError, Outcome, Prepared,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use vidya_core::licence::{parse_licence_key, verify_v2, LicenceV2};

const OWNER: &str = "00000000-0000-4000-8000-0000000000aa";

/// Verify the stored `.vlic` + `licence_key` with the app's verifier; return the licence.
fn verify_stored(vlic: &str, licence_key: &str, pk: &[u8; 32], machine: &str) -> LicenceV2 {
    let (p, s) = parse_licence_key(vlic).expect(".vlic parses");
    let (p2, s2) = parse_licence_key(licence_key).expect("grouped key parses");
    assert_eq!((&p, &s), (&p2, &s2), "key and .vlic encode the same bytes");
    verify_v2(&p, &s, pk, machine).expect("verify_v2 accepts the stored artefact")
}

async fn issue_jobs(db: &TestDb, issuance_id: Uuid) -> Vec<(Uuid, String, String, i32, Option<String>)> {
    sqlx::query(
        "SELECT id, dedupe_key, state, attempts, last_error FROM billing.jobs \
         WHERE kind = 'issue_licence' AND payload->>'issuance_id' = $1 ORDER BY created_at",
    )
    .bind(issuance_id.to_string())
    .fetch_all(&db.seed)
    .await
    .unwrap()
    .into_iter()
    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4)))
    .collect()
}

async fn delivery_jobs(db: &TestDb, issuance_id: Uuid) -> Vec<String> {
    sqlx::query(
        "SELECT dedupe_key FROM billing.jobs WHERE kind = 'deliver_licence' AND payload->>'issuance_id' = $1 \
         ORDER BY created_at",
    )
    .bind(issuance_id.to_string())
    .fetch_all(&db.seed)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.get(0))
    .collect()
}

async fn make_ready_now(db: &TestDb, job_id: Uuid) {
    sqlx::query("UPDATE billing.jobs SET available_at = now() WHERE id = $1")
        .bind(job_id)
        .execute(&db.seed)
        .await
        .unwrap();
}

// ------------------------------------------------------------------------ (a) --

#[tokio::test]
async fn a_crash_after_signing_before_completion_retries_to_one_licence() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "test-k");

    // Signer A claims, persists the payload, signs …
    let claim_a = claim_job(&db.signer, "signer-a").await.unwrap().unwrap();
    let Prepared::Sign(first) = prepare(&db.signer, &claim_a, &signer).await.unwrap() else {
        panic!("expected a fresh artefact")
    };
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "pending");
    assert_eq!(
        row.payload.as_deref(),
        Some(first.payload.as_slice()),
        "payload is persisted BEFORE signing"
    );
    assert!(row.signature.is_none() && row.vlic.is_none());

    // … and dies INSIDE the commit transaction: the artefact is written but the
    // job is never marked done and nothing commits (process killed → rollback).
    {
        let mut tx = db.signer.begin().await.unwrap();
        sqlx::query(
            "UPDATE billing.licence_issuances SET signature = $2, licence_key = $3, vlic = $4, status = 'issued', \
             issued_ts = now(), signer_key_id = 'test-k' WHERE id = $1 AND status = 'pending'",
        )
        .bind(s.issuance_id)
        .bind(&first.signature)
        .bind(&first.licence_key)
        .bind(&first.vlic)
        .execute(&mut *tx)
        .await
        .unwrap();
        drop(tx); // crash before COMMIT
    }
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "pending");
    assert!(row.signature.is_none());
    assert_eq!(job(&db, s.job_id).await.state, "running");

    // Lease expires → recovered (audited as a failed attempt) → signer B retries.
    expire_lease(&db, s.job_id).await;
    assert_eq!(recover_expired_leases(&db.signer).await.unwrap(), 1);
    assert_eq!(audit_count(&db, "licence_issue_attempt_failed", s.issuance_id).await, 1);
    let out = process_one(&db.signer, &signer, "signer-b").await.unwrap();
    assert_eq!(
        out,
        Outcome::Issued {
            issuance_id: s.issuance_id,
            licence_id: s.licence_id.clone()
        }
    );

    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "issued");
    assert_eq!(row.payload.as_deref(), Some(first.payload.as_slice()));
    assert_eq!(row.signature.as_deref(), Some(first.signature.as_slice()));
    assert_eq!(row.vlic.as_deref(), Some(first.vlic.as_str()));
    let lic = verify_stored(
        row.vlic.as_deref().unwrap(),
        row.licence_key.as_deref().unwrap(),
        signer.public_key(),
        &s.machine_code,
    );
    assert_eq!(lic.licence_id, s.licence_id);
    assert_eq!(lic.issued_at, s.issued_at);

    // Exactly one paid order, one issuance, one issued licence, one downloadable key.
    assert_eq!(order_summary(&db, s.entitlement_id).await, (1, 1, 1, 1));
    let jobs = issue_jobs(&db, s.issuance_id).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!((jobs[0].2.as_str(), jobs[0].3), ("done", 2));
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 1);
    // The email is enqueued immediately, once.
    assert_eq!(
        delivery_jobs(&db, s.issuance_id).await,
        vec![format!("deliver:{}", s.issuance_id)]
    );

    // The zombie signer A wakes up: nothing is written twice.
    let zombie = commit(&db.signer, &claim_a, &signer, &Prepared::Sign(first.clone()))
        .await
        .unwrap();
    assert_eq!(zombie, Outcome::LeaseLost);
    assert_eq!(issuance(&db, s.issuance_id).await, row);
    assert_eq!(order_summary(&db, s.entitlement_id).await, (1, 1, 1, 1));
    db.drop_db().await;
}

// ------------------------------------------------------------------------ (b) --

#[tokio::test]
async fn b_signer_failure_then_owner_retry_issues_the_original_licence_id() {
    let db = TestDb::new().await;
    let s = seed_issue(
        &db,
        &SeedSpec {
            max_attempts: 2,
            ..Default::default()
        },
    )
    .await;
    let signer = common::test_signer(7, "test-k");
    let portal = db.portal_pool().await;

    // Attempt 1: payload persisted, then the signing key is unavailable.
    let c1 = claim_job(&db.signer, "w").await.unwrap().unwrap();
    let Prepared::Sign(a1) = prepare(&db.signer, &c1, &signer).await.unwrap() else {
        panic!()
    };
    let key_down = JobError::Transient("signing key unavailable (simulated)".into());
    assert_eq!(record_failure(&db.signer, &c1, &key_down).await.unwrap(), Some(false));
    // Attempt 2 (after backoff): same failure → job dead → issuance failed.
    make_ready_now(&db, s.job_id).await;
    let c2 = claim_job(&db.signer, "w").await.unwrap().unwrap();
    assert_eq!(record_failure(&db.signer, &c2, &key_down).await.unwrap(), Some(true));

    let failed = issuance(&db, s.issuance_id).await;
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.payload.as_deref(), Some(a1.payload.as_slice()));
    assert!(failed.signature.is_none() && failed.vlic.is_none() && failed.licence_key.is_none());
    assert!(failed
        .last_error
        .as_deref()
        .unwrap()
        .contains("signing key unavailable"));
    // Order stays paid ("Paid · Licence pending"): no licence, no key.
    assert_eq!(order_summary(&db, s.entitlement_id).await, (1, 1, 0, 0));
    assert_eq!(audit_count(&db, "licence_issue_attempt_failed", s.issuance_id).await, 2);
    assert_eq!(audit_count(&db, "licence_issue_failed", s.issuance_id).await, 1);
    assert_eq!(process_one(&db.signer, &signer, "w").await.unwrap(), Outcome::Idle);

    // The portal role can retry, but can never touch identity/artefact columns.
    let e = sqlx::query("UPDATE billing.licence_issuances SET payload = NULL WHERE id = $1")
        .bind(s.issuance_id)
        .execute(&portal)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("permission denied"), "{e}");
    let e = sqlx::query("UPDATE billing.licence_issuances SET status = 'issued' WHERE id = $1")
        .bind(s.issuance_id)
        .execute(&portal)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("not allowed"), "{e}");

    // Owner: Retry issuance (same row, same licence_id, fresh job).
    let retry_job = portal_retry_issuance(&portal, s.issuance_id, OWNER, "signing key restored on the signer host")
        .await
        .unwrap();
    let pending = issuance(&db, s.issuance_id).await;
    assert_eq!(pending.status, "pending");
    assert_eq!(pending.last_error, None);
    assert_eq!(pending.payload.as_deref(), Some(a1.payload.as_slice()));
    assert_eq!(
        portal_retry_issuance(&portal, s.issuance_id, OWNER, "double click").await,
        Err("not_failed".into()),
        "a second retry while pending is refused"
    );

    let out = process_one(&db.signer, &signer, "w").await.unwrap();
    assert_eq!(
        out,
        Outcome::Issued {
            issuance_id: s.issuance_id,
            licence_id: s.licence_id.clone()
        }
    );
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(
        row.payload.as_deref(),
        Some(a1.payload.as_slice()),
        "byte-identical payload to the failed attempt"
    );
    assert_eq!(row.signature.as_deref(), Some(a1.signature.as_slice()));
    let lic = verify_stored(
        row.vlic.as_deref().unwrap(),
        row.licence_key.as_deref().unwrap(),
        signer.public_key(),
        &s.machine_code,
    );
    assert_eq!(lic.licence_id, s.licence_id, "original licence_id");
    assert_eq!(lic.issued_at, s.issued_at);

    // One paid order, one issuance (same generation), one issued licence, one key.
    assert_eq!(order_summary(&db, s.entitlement_id).await, (1, 1, 1, 1));
    let gens: i64 =
        sqlx::query("SELECT max(generation)::bigint FROM billing.licence_issuances WHERE entitlement_id = $1")
            .bind(s.entitlement_id)
            .fetch_one(&db.seed)
            .await
            .unwrap()
            .get(0);
    assert_eq!(gens, 1);

    // History: the dead job is untouched, the retry job is done, every attempt audited.
    let jobs = issue_jobs(&db, s.issuance_id).await;
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].0, s.job_id);
    assert_eq!((jobs[0].2.as_str(), jobs[0].3), ("dead", 2));
    assert!(jobs[0].4.as_deref().unwrap().contains("signing key unavailable"));
    assert_eq!(jobs[1].0, retry_job);
    assert_eq!(jobs[1].1, format!("issue:{}:retry:1", s.issuance_id));
    assert_eq!(jobs[1].2, "done");
    assert_eq!(audit_count(&db, "licence_issue_attempt_failed", s.issuance_id).await, 2);
    assert_eq!(audit_count(&db, "licence_issue_failed", s.issuance_id).await, 1);
    assert_eq!(
        audit_count_any(&db, "licence_issue_retry_requested", s.issuance_id).await,
        1
    );
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 1);
    let retry_audit = sqlx::query(
        "SELECT actor, reason, before_json->>'last_error' AS err, after_json->>'licence_id' AS lid \
         FROM billing.owner_audit WHERE action = 'licence_issue_retry_requested' AND target = $1",
    )
    .bind(s.issuance_id.to_string())
    .fetch_one(&db.seed)
    .await
    .unwrap();
    assert_eq!(retry_audit.get::<String, _>("actor"), OWNER);
    assert!(retry_audit.get::<String, _>("err").contains("signing key unavailable"));
    assert_eq!(retry_audit.get::<String, _>("lid"), s.licence_id);

    // Once issued, nothing can reset it — not the portal, not even a superuser.
    assert_eq!(
        portal_retry_issuance(&portal, s.issuance_id, OWNER, "again please").await,
        Err("not_failed".into())
    );
    for sql in [
        "UPDATE billing.licence_issuances SET status = 'pending' WHERE id = $1",
        "UPDATE billing.licence_issuances SET status = 'failed' WHERE id = $1",
    ] {
        let e = sqlx::query(sql)
            .bind(s.issuance_id)
            .execute(&db.seed)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("not allowed"), "{e}");
    }
    portal.close().await;
    db.drop_db().await;
}

#[tokio::test]
async fn b_retry_is_refused_for_a_refunded_purchase() {
    let db = TestDb::new().await;
    let s = seed_issue(
        &db,
        &SeedSpec {
            commercial_status: "refunded",
            ..Default::default()
        },
    )
    .await;
    let signer = common::test_signer(7, "test-k");
    assert!(matches!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::Failed { dead: true, .. }
    ));
    assert_eq!(issuance(&db, s.issuance_id).await.status, "failed");
    let portal = db.portal_pool().await;
    assert_eq!(
        portal_retry_issuance(&portal, s.issuance_id, OWNER, "please retry").await,
        Err("blocked".into())
    );
    assert_eq!(issuance(&db, s.issuance_id).await.status, "failed");
    portal.close().await;
    db.drop_db().await;
}

#[tokio::test]
async fn b_stored_payload_that_no_longer_matches_is_never_signed() {
    // If the entitlement data drifted after the payload was persisted (e.g. a
    // school-name edit), the signer refuses rather than signing different bytes.
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "test-k");
    let c = claim_job(&db.signer, "w").await.unwrap().unwrap();
    prepare(&db.signer, &c, &signer).await.unwrap();
    record_failure(&db.signer, &c, &JobError::Transient("simulated crash".into()))
        .await
        .unwrap();
    sqlx::query("UPDATE billing.entitlements SET school_name = 'Renamed School' WHERE id = $1")
        .bind(s.entitlement_id)
        .execute(&db.seed)
        .await
        .unwrap();
    make_ready_now(&db, s.job_id).await;
    match process_one(&db.signer, &signer, "w").await.unwrap() {
        Outcome::Failed { dead: true, error } => assert!(error.contains("stored payload differs"), "{error}"),
        other => panic!("expected a permanent failure, got {other:?}"),
    }
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "failed");
    assert!(row.signature.is_none());
    db.drop_db().await;
}

// ------------------------------------------------------------------------ (c) --

/// What the billing worker's `deliverLicence` reads (as `vidya_worker`) to build
/// the email: the attachment is `vlic`, the pasted key is `licence_key`.
async fn worker_reads_artefact(worker: &PgPool, issuance_id: Uuid) -> (String, String, String) {
    let r = sqlx::query(
        "SELECT licence_id, licence_key, vlic FROM billing.licence_issuances WHERE id = $1 AND status = 'issued'",
    )
    .bind(issuance_id)
    .fetch_one(worker)
    .await
    .unwrap();
    (r.get(0), r.get(1), r.get(2))
}

#[tokio::test]
async fn c_email_failure_then_resend_sends_the_identical_artefact() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "test-k");
    let worker = db.worker_pool().await;
    let portal = db.portal_pool().await;

    assert!(matches!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::Issued { .. }
    ));
    assert_eq!(
        delivery_jobs(&db, s.issuance_id).await,
        vec![format!("deliver:{}", s.issuance_id)]
    );

    // First email attempt: the worker reads the artefact, the provider fails.
    let first = worker_reads_artefact(&worker, s.issuance_id).await;
    sqlx::query(
        "INSERT INTO billing.deliveries (issuance_id, channel, idempotency_key, state, attempts, last_error) \
         VALUES ($1, 'email', $2, 'failed', 1, 'simulated email provider outage (503)')",
    )
    .bind(s.issuance_id)
    .bind(format!("deliver:{}", s.issuance_id))
    .execute(&worker)
    .await
    .unwrap();

    // A stray/duplicate issue job (e.g. an accidental requeue) cannot re-sign.
    sqlx::query(
        "INSERT INTO billing.jobs (kind, dedupe_key, payload) \
         VALUES ('issue_licence', $1, jsonb_build_object('issuance_id', $2::text))",
    )
    .bind(format!("issue:{}:requeue", s.issuance_id))
    .bind(s.issuance_id.to_string())
    .execute(&db.seed)
    .await
    .unwrap();
    assert_eq!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::AlreadyIssued {
            issuance_id: s.issuance_id
        }
    );

    // Owner: Resend licence (portal enqueues a resend job; same as requestResend).
    let key = format!("resend:{}:{}", s.issuance_id, Uuid::new_v4());
    sqlx::query(
        "INSERT INTO billing.jobs (kind, dedupe_key, payload) \
         VALUES ('deliver_licence', $1, jsonb_build_object('issuance_id', $2::text, 'idempotency_key', $1::text))",
    )
    .bind(&key)
    .bind(s.issuance_id.to_string())
    .execute(&portal)
    .await
    .unwrap();
    let resent = worker_reads_artefact(&worker, s.issuance_id).await;
    assert_eq!(resent, first, "resend reads the exact same licence id, key and .vlic");
    let lic = verify_stored(&resent.2, &resent.1, signer.public_key(), &s.machine_code);
    assert_eq!(lic.licence_id, s.licence_id);

    // Still one licence; the artefact cannot be altered by any role.
    assert_eq!(order_summary(&db, s.entitlement_id).await, (1, 1, 1, 1));
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 1);
    let e = sqlx::query("UPDATE billing.licence_issuances SET vlic = 'x' WHERE id = $1")
        .bind(s.issuance_id)
        .execute(&worker)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("permission denied"), "{e}");
    for sql in [
        "UPDATE billing.licence_issuances SET vlic = 'x' WHERE id = $1",
        "UPDATE billing.licence_issuances SET signature = '\\x00'::bytea WHERE id = $1",
        "UPDATE billing.licence_issuances SET payload = '\\x00'::bytea WHERE id = $1",
        "UPDATE billing.licence_issuances SET licence_id = 'VID-other' WHERE id = $1",
    ] {
        let e = sqlx::query(sql)
            .bind(s.issuance_id)
            .execute(&db.seed)
            .await
            .unwrap_err();
        assert!(e.to_string().contains("immutable"), "{sql}: {e}");
    }
    let e = sqlx::query("UPDATE billing.licence_issuances SET signature = '\\x00'::bytea WHERE id = $1")
        .bind(s.issuance_id)
        .execute(&db.signer)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("immutable"), "{e}");
    assert_eq!(worker_reads_artefact(&worker, s.issuance_id).await, first);
    worker.close().await;
    portal.close().await;
    db.drop_db().await;
}
