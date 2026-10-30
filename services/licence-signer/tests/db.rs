//! Job-loop tests against a real Postgres (the real billing migration), with the
//! signer running as the least-privilege `vidya_signer` role.

mod common;

use std::sync::Arc;

use common::{audit_count, expire_lease, insert_job, issuance, job, seed_issue, SeedSpec, TestDb};
use licence_signer::worker::{
    claim_job, commit, prepare, process_one, record_failure, recover_expired_leases, JobError, Outcome, Prepared,
};
use vidya_core::licence::{parse_licence_key, verify_v2, LicenceV2};

#[tokio::test]
async fn claim_sign_complete() {
    let db = TestDb::new().await;
    let s = seed_issue(
        &db,
        &SeedSpec {
            modules: &["store", "core", "accounts"],
            ..Default::default()
        },
    )
    .await;
    let signer = common::test_signer(7, "test-2026-a");

    let out = process_one(&db.signer, &signer, "w1").await.unwrap();
    assert_eq!(
        out,
        Outcome::Issued {
            issuance_id: s.issuance_id,
            licence_id: s.licence_id.clone()
        }
    );

    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "issued");
    assert_eq!(row.signer_key_id.as_deref(), Some("test-2026-a"));
    assert!(row.issued_ts_set);
    assert_eq!(row.last_error, None);
    let payload = row.payload.clone().unwrap();
    let sig = row.signature.clone().unwrap();

    // Stored artefact verifies with the app's verifier; key & .vlic encode the same bytes.
    let lic = verify_v2(&payload, &sig, signer.public_key(), &s.machine_code).unwrap();
    assert_eq!(
        lic,
        LicenceV2 {
            v: 2,
            licence_id: s.licence_id.clone(),
            school_name: "Saraswati Public School".into(),
            machine_code: s.machine_code.clone(),
            issued_at: s.issued_at.clone(),
            plan: "perpetual".into(),
            modules: vec!["core".into(), "accounts".into(), "store".into()],
        }
    );
    for text in [row.licence_key.as_deref().unwrap(), row.vlic.as_deref().unwrap()] {
        let (p, g) = parse_licence_key(text).unwrap();
        assert_eq!((p, g), (payload.clone(), sig.clone()));
    }

    let j = job(&db, s.job_id).await;
    assert_eq!(j.state, "done");
    assert_eq!(j.attempts, 1);
    assert!(j.completed && !j.lease_set);
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 1);

    // Nothing left to do.
    assert_eq!(process_one(&db.signer, &signer, "w1").await.unwrap(), Outcome::Idle);
    db.drop_db().await;
}

#[tokio::test]
async fn signer_role_cannot_create_entitlements_or_payments() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let e = sqlx::query(
        "INSERT INTO billing.entitlements (order_id, modules, machine_code, school_name, source) \
         SELECT order_id, modules, machine_code, school_name, source FROM billing.entitlements LIMIT 1",
    )
    .execute(&db.signer)
    .await
    .unwrap_err();
    assert!(e.to_string().contains("permission denied"), "{e}");
    let e = sqlx::query("UPDATE billing.licence_issuances SET machine_code = 'X' WHERE id = $1")
        .bind(s.issuance_id)
        .execute(&db.signer)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("permission denied"), "{e}");
    let e = sqlx::query("UPDATE billing.entitlements SET commercial_status = 'active'")
        .execute(&db.signer)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("permission denied"), "{e}");
    db.drop_db().await;
}

#[tokio::test]
async fn concurrent_signers_produce_exactly_one_artefact() {
    let db = TestDb::new().await;
    // One issuance with TWO jobs pointing at it (e.g. an accidental re-enqueue),
    // plus 12 ordinary issuances.
    let dup = seed_issue(&db, &SeedSpec::default()).await;
    let dup_job2 = insert_job(&db, dup.issuance_id, &format!("issue:{}:requeue", dup.issuance_id), 8).await;
    let mut others = Vec::new();
    for i in 0..12 {
        let school = format!("School {i}");
        let school: &'static str = Box::leak(school.into_boxed_str());
        others.push(
            seed_issue(
                &db,
                &SeedSpec {
                    school,
                    ..Default::default()
                },
            )
            .await,
        );
    }

    let signer = Arc::new(common::test_signer(7, "k"));
    let pool_b = db.another_signer_pool().await;
    let pools = [db.signer.clone(), pool_b.clone()];
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let mut tasks = Vec::new();
    for (n, pool) in pools.into_iter().enumerate() {
        let signer = signer.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut issued = 0;
            loop {
                match process_one(&pool, &signer, &format!("signer-{n}")).await.unwrap() {
                    Outcome::Idle => break,
                    Outcome::Issued { .. } => issued += 1,
                    Outcome::AlreadyIssued { .. } => {}
                    other => panic!("unexpected {other:?}"),
                }
            }
            issued
        }));
    }
    let mut total = 0;
    for t in tasks {
        total += t.await.unwrap();
    }
    assert_eq!(total, 13, "13 issuances → exactly 13 signed artefacts");

    assert_eq!(audit_count(&db, "licence_issued", dup.issuance_id).await, 1);
    assert_eq!(issuance(&db, dup.issuance_id).await.status, "issued");
    assert_eq!(job(&db, dup.job_id).await.state, "done");
    assert_eq!(job(&db, dup_job2).await.state, "done");
    for o in &others {
        assert_eq!(issuance(&db, o.issuance_id).await.status, "issued");
        assert_eq!(audit_count(&db, "licence_issued", o.issuance_id).await, 1);
        assert_eq!(job(&db, o.job_id).await.attempts, 1, "no job was claimed twice");
    }
    pool_b.close().await;
    db.drop_db().await;
}

#[tokio::test]
async fn crash_between_signing_and_commit_retries_to_identical_artefact() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "k");

    // Signer A claims and signs… then "crashes" before commit.
    let claim_a = claim_job(&db.signer, "signer-a").await.unwrap().unwrap();
    let Prepared::Sign(first) = prepare(&db.signer, &claim_a, &signer).await.unwrap() else {
        panic!()
    };
    assert_eq!(issuance(&db, s.issuance_id).await.status, "pending");

    // Lease not expired yet → nobody else can take it.
    assert_eq!(recover_expired_leases(&db.signer).await.unwrap(), 0);
    assert_eq!(
        process_one(&db.signer, &signer, "signer-b").await.unwrap(),
        Outcome::Idle
    );

    // Lease runs out → recovered → signer B completes it.
    expire_lease(&db, s.job_id).await;
    assert_eq!(recover_expired_leases(&db.signer).await.unwrap(), 1);
    let out = process_one(&db.signer, &signer, "signer-b").await.unwrap();
    assert!(matches!(out, Outcome::Issued { .. }), "{out:?}");

    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.payload.as_deref(), Some(first.payload.as_slice()));
    assert_eq!(row.signature.as_deref(), Some(first.signature.as_slice()));
    assert_eq!(row.licence_key.as_deref(), Some(first.licence_key.as_str()));
    assert_eq!(row.vlic.as_deref(), Some(first.vlic.as_str()));
    assert_eq!(job(&db, s.job_id).await.attempts, 2);

    // The zombie signer A wakes up and tries to commit: it no longer owns the job.
    let zombie = commit(&db.signer, &claim_a, &signer, &Prepared::Sign(first.clone()))
        .await
        .unwrap();
    assert_eq!(zombie, Outcome::LeaseLost);
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 1);
    assert_eq!(issuance(&db, s.issuance_id).await, row);
    db.drop_db().await;
}

async fn assert_not_signed(spec: SeedSpec<'_>, expect_in_error: &str) {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &spec).await;
    let signer = common::test_signer(7, "k");
    let out = process_one(&db.signer, &signer, "w").await.unwrap();
    match out {
        Outcome::Failed { dead: true, error } => assert!(error.contains(expect_in_error), "{error}"),
        other => panic!("expected dead failure, got {other:?}"),
    }
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "failed");
    assert!(row.payload.is_none() && row.signature.is_none() && row.licence_key.is_none() && row.vlic.is_none());
    assert!(row.last_error.unwrap().contains(expect_in_error));
    let j = job(&db, s.job_id).await;
    assert_eq!(j.state, "dead");
    assert!(j.locked_by.is_none() && !j.lease_set);
    assert_eq!(audit_count(&db, "licence_issued", s.issuance_id).await, 0);
    assert_eq!(audit_count(&db, "licence_issue_failed", s.issuance_id).await, 1);
    db.drop_db().await;
}

#[tokio::test]
async fn refunded_entitlement_is_not_signed() {
    assert_not_signed(
        SeedSpec {
            commercial_status: "refunded",
            ..Default::default()
        },
        "refunded",
    )
    .await;
}

#[tokio::test]
async fn disputed_entitlement_is_not_signed() {
    assert_not_signed(
        SeedSpec {
            commercial_status: "disputed",
            ..Default::default()
        },
        "disputed",
    )
    .await;
}

#[tokio::test]
async fn bad_machine_code_is_not_signed() {
    assert_not_signed(
        SeedSpec {
            machine_code: "3N0S-10YX-JQCR-X",
            ..Default::default()
        },
        "machine code",
    )
    .await;
}

#[tokio::test]
async fn unknown_module_or_missing_core_is_not_signed() {
    assert_not_signed(
        SeedSpec {
            modules: &["core", "chess"],
            ..Default::default()
        },
        "chess",
    )
    .await;
    assert_not_signed(
        SeedSpec {
            modules: &["accounts"],
            ..Default::default()
        },
        "core",
    )
    .await;
}

#[tokio::test]
async fn refund_between_sign_and_commit_is_not_written() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "k");
    let claim = claim_job(&db.signer, "w").await.unwrap().unwrap();
    let prepared = prepare(&db.signer, &claim, &signer).await.unwrap();
    sqlx::query("UPDATE billing.entitlements SET commercial_status = 'refunded' WHERE id = $1")
        .bind(s.entitlement_id)
        .execute(&db.seed)
        .await
        .unwrap();
    let err = commit(&db.signer, &claim, &signer, &prepared).await.unwrap_err();
    assert!(matches!(&err, JobError::Permanent(m) if m.contains("refunded")));
    assert_eq!(record_failure(&db.signer, &claim, &err).await.unwrap(), Some(true));
    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.status, "failed");
    assert!(row.payload.is_none());
    db.drop_db().await;
}

#[tokio::test]
async fn transient_failure_backs_off_then_dies_after_max_attempts() {
    let db = TestDb::new().await;
    let s = seed_issue(
        &db,
        &SeedSpec {
            max_attempts: 2,
            ..Default::default()
        },
    )
    .await;
    let signer = common::test_signer(7, "k");

    let c1 = claim_job(&db.signer, "w").await.unwrap().unwrap();
    assert_eq!(c1.attempts, 1);
    let dead = record_failure(
        &db.signer,
        &c1,
        &JobError::Transient("database error: simulated".into()),
    )
    .await
    .unwrap();
    assert_eq!(dead, Some(false));
    let j = job(&db, s.job_id).await;
    assert_eq!((j.state.as_str(), j.attempts), ("ready", 1));
    assert!(j.available_in_future, "backoff pushes available_at into the future");
    assert_eq!(j.last_error.as_deref(), Some("database error: simulated"));
    assert!(
        claim_job(&db.signer, "w").await.unwrap().is_none(),
        "not claimable during backoff"
    );
    assert_eq!(issuance(&db, s.issuance_id).await.status, "pending");

    sqlx::query("UPDATE billing.jobs SET available_at = now() WHERE id = $1")
        .bind(s.job_id)
        .execute(&db.seed)
        .await
        .unwrap();
    let c2 = claim_job(&db.signer, "w").await.unwrap().unwrap();
    assert_eq!(c2.attempts, 2);
    let dead = record_failure(
        &db.signer,
        &c2,
        &JobError::Transient("database error: simulated".into()),
    )
    .await
    .unwrap();
    assert_eq!(dead, Some(true));
    assert_eq!(job(&db, s.job_id).await.state, "dead");
    assert_eq!(issuance(&db, s.issuance_id).await.status, "failed");
    let _ = signer;
    db.drop_db().await;
}

#[tokio::test]
async fn lease_recovery() {
    let db = TestDb::new().await;
    let a = seed_issue(&db, &SeedSpec::default()).await;
    let b = seed_issue(
        &db,
        &SeedSpec {
            school: "Other School",
            max_attempts: 1,
            ..Default::default()
        },
    )
    .await;
    let signer = common::test_signer(7, "k");

    // Two signers claim one job each, then both die.
    let c1 = claim_job(&db.signer, "dead-1").await.unwrap().unwrap();
    let c2 = claim_job(&db.signer, "dead-2").await.unwrap().unwrap();
    let mut claimed = [c1.job_id, c2.job_id];
    claimed.sort();
    let mut want = [a.job_id, b.job_id];
    want.sort();
    assert_eq!(claimed, want);
    assert_eq!(job(&db, a.job_id).await.state, "running");
    assert!(job(&db, a.job_id).await.lease_set);

    expire_lease(&db, a.job_id).await;
    expire_lease(&db, b.job_id).await;
    assert_eq!(recover_expired_leases(&db.signer).await.unwrap(), 2);

    // a: attempts 1 < 8 → ready again; b: max_attempts 1 used → dead + issuance failed.
    let ja = job(&db, a.job_id).await;
    assert_eq!(ja.state, "ready");
    assert!(ja.locked_by.is_none() && !ja.lease_set);
    assert!(ja.last_error.unwrap().contains("lease expired"));
    assert_eq!(job(&db, b.job_id).await.state, "dead");
    assert_eq!(issuance(&db, b.issuance_id).await.status, "failed");

    // a is then signed normally; recovery is idempotent.
    assert!(matches!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::Issued { .. }
    ));
    assert_eq!(issuance(&db, a.issuance_id).await.status, "issued");
    assert_eq!(recover_expired_leases(&db.signer).await.unwrap(), 0);
    assert_eq!(process_one(&db.signer, &signer, "w").await.unwrap(), Outcome::Idle);
    db.drop_db().await;
}

#[tokio::test]
async fn transfer_generation_is_signed_and_old_licence_untouched() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;
    let signer = common::test_signer(7, "k");
    assert!(matches!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::Issued { .. }
    ));
    let gen1 = issuance(&db, s.issuance_id).await;

    // The worker records a transfer: generation 2, new licence_id + machine code.
    let new_mc = vidya_core::licence::machine_code("new-pc");
    let row = sqlx::query(
        "INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at) \
         VALUES ($1, 2, $2, $3, '2026-11-01T10:00:00Z') RETURNING id",
    )
    .bind(s.entitlement_id)
    .bind(format!("VID-{}", uuid::Uuid::new_v4()))
    .bind(&new_mc)
    .fetch_one(&db.seed)
    .await
    .unwrap();
    let gen2_id: uuid::Uuid = sqlx::Row::get(&row, 0);
    insert_job(&db, gen2_id, &format!("issue:{gen2_id}"), 8).await;
    assert!(matches!(
        process_one(&db.signer, &signer, "w").await.unwrap(),
        Outcome::Issued { .. }
    ));

    let gen2 = issuance(&db, gen2_id).await;
    verify_v2(
        gen2.payload.as_deref().unwrap(),
        gen2.signature.as_deref().unwrap(),
        signer.public_key(),
        &new_mc,
    )
    .unwrap();
    // The old PC's licence is untouched and still verifies for the old machine.
    assert_eq!(issuance(&db, s.issuance_id).await, gen1);
    verify_v2(
        gen1.payload.as_deref().unwrap(),
        gen1.signature.as_deref().unwrap(),
        signer.public_key(),
        &s.machine_code,
    )
    .unwrap();
    db.drop_db().await;
}
