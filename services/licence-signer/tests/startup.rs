//! The real binary refuses to start with a key that app releases would not trust,
//! and never echoes key material. No database is needed: the key is checked first.
#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use licence_signer::key::{fingerprint, Signer};

fn write_key(dir: &std::path::Path, seed: u8, mode: u32) -> std::path::PathBuf {
    let p = dir.join("licence-signing.key");
    std::fs::write(&p, common::key_file_text(seed)).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
    p
}

fn run(key_file: &std::path::Path, fp: &str, extra: &[(&str, &str)]) -> std::process::Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_licence-signer"));
    c.env_clear()
        .env("LICENCE_SIGNING_KEY_FILE", key_file)
        .env("LICENCE_SIGNING_KEY_ID", "test-k1")
        .env("EXPECTED_PUBLIC_KEY_FINGERPRINT", fp)
        // Unroutable on purpose: the process must exit before ever connecting.
        .env("SIGNER_DATABASE_URL", "postgres://nobody@127.0.0.1:1/none");
    for (k, v) in extra {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn fp_of(seed: u8) -> String {
    fingerprint(&common::test_key(seed).verifying_key().to_bytes())
}

#[test]
fn wrong_public_key_fingerprint_refuses_to_start() {
    let dir = tempfile::tempdir().unwrap();
    let key = write_key(dir.path(), 11, 0o600);
    let out = run(&key, &fp_of(12), &[]);
    assert!(!out.status.success());
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains("refusing to start"), "{all}");
    assert!(
        !all.contains(&common::key_file_text(11)),
        "key material leaked to output"
    );
    assert!(!all.contains("connected"));

    // Library path agrees.
    assert!(Signer::load(&key, "test-k1", &fp_of(12)).is_err());
    assert!(
        Signer::load(&key, "test-k1", &format!("{},{}", fp_of(12), fp_of(11))).is_ok(),
        "allow-list"
    );
}

#[test]
fn right_fingerprint_gets_past_the_key_check() {
    let dir = tempfile::tempdir().unwrap();
    let key = write_key(dir.path(), 11, 0o600);
    let out = run(&key, &fp_of(11), &[]);
    // It then fails only because the DB is unreachable — after the key check.
    assert!(!out.status.success());
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains("matches the expected fingerprint"), "{all}");
    assert!(all.contains("cannot connect"), "{all}");
    assert!(!all.contains(&common::key_file_text(11)));
}

#[test]
fn group_readable_key_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let key = write_key(dir.path(), 11, 0o644);
    let out = run(&key, &fp_of(11), &[]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("permissions")
            || String::from_utf8_lossy(&out.stderr).contains("permissions")
    );
}

#[test]
fn inline_key_in_environment_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let key = write_key(dir.path(), 11, 0o600);
    let inline = common::key_file_text(11);
    let out = run(&key, &fp_of(11), &[("LICENCE_SIGNING_KEY", &inline)]);
    assert!(!out.status.success());
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(all.contains("LICENCE_SIGNING_KEY_FILE"), "{all}");
    assert!(!all.contains(&inline));
}

#[test]
fn fingerprint_subcommand_prints_public_info_only() {
    let dir = tempfile::tempdir().unwrap();
    let key = write_key(dir.path(), 11, 0o600);
    let out = Command::new(env!("CARGO_BIN_EXE_licence-signer"))
        .arg("fingerprint")
        .env_clear()
        .env("LICENCE_SIGNING_KEY_FILE", &key)
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains(&fp_of(11)));
    assert!(!s.contains(&common::key_file_text(11)));
}
