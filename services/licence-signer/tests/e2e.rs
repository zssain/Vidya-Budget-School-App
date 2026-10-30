//! The real binary, end to end: loads a key FILE, connects as `vidya_signer`,
//! signs a pending job, and exits cleanly on SIGTERM.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::time::Duration;

use common::{issuance, seed_issue, SeedSpec, TestDb};
use vidya_core::licence::verify_v2;

#[tokio::test]
async fn binary_signs_a_job_and_stops_on_sigterm() {
    let db = TestDb::new().await;
    let s = seed_issue(&db, &SeedSpec::default()).await;

    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("licence-signing.key");
    std::fs::write(&key_path, common::key_file_text(21)).unwrap();
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let fp = licence_signer::key::fingerprint(&common::test_key(21).verifying_key().to_bytes());

    let admin = common::admin_url();
    let base = admin.rsplit_once('/').unwrap().0;
    // Startup option `role=vidya_signer`: the binary runs with the production grants.
    let url = format!("{base}/{}?options=-c%20role%3Dvidya_signer", db.name);
    let mut child = Command::new(env!("CARGO_BIN_EXE_licence-signer"))
        .env_clear()
        .env("LICENCE_SIGNING_KEY_FILE", &key_path)
        .env("LICENCE_SIGNING_KEY_ID", "e2e-key")
        .env("EXPECTED_PUBLIC_KEY_FINGERPRINT", &fp)
        .env("SIGNER_DATABASE_URL", &url)
        .env("SIGNER_POLL_INTERVAL_SECS", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut issued = false;
    for _ in 0..150 {
        if issuance(&db, s.issuance_id).await.status == "issued" {
            issued = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let st = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(st.success());
    let mut exited = None;
    for _ in 0..100 {
        if let Some(code) = child.try_wait().unwrap() {
            exited = Some(code);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if exited.is_none() {
        child.kill().ok();
    }
    let out = child.wait_with_output().unwrap();
    let logs = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(issued, "binary did not sign the job: {logs}");
    assert!(exited.expect("did not exit after SIGTERM").success(), "{logs}");
    assert!(logs.contains("shutdown signal received"), "{logs}");
    assert!(logs.contains("licence issued"), "{logs}");
    assert!(!logs.contains(&common::key_file_text(21)), "key material in logs");

    let row = issuance(&db, s.issuance_id).await;
    assert_eq!(row.signer_key_id.as_deref(), Some("e2e-key"));
    let pk = common::test_key(21).verifying_key().to_bytes();
    verify_v2(
        row.payload.as_deref().unwrap(),
        row.signature.as_deref().unwrap(),
        &pk,
        &s.machine_code,
    )
    .unwrap();
    db.drop_db().await;
}
