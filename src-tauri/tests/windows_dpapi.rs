//! Windows DPAPI integration test (P2.4). Compiled and run only on Windows (CI
//! and the Windows VM). Exercises `CryptProtectData`/`CryptUnprotectData`
//! without touching `ProgramData`, so it needs no special permissions.
#![cfg(target_os = "windows")]

use vidya_app_lib::platform::windows::{dpapi_protect, dpapi_unprotect};

#[test]
fn protect_then_unprotect_round_trip() {
    let entropy = b"vidya-test-entropy";
    let blob = dpapi_protect(b"secret bytes", entropy).expect("protect");
    let plain = dpapi_unprotect(&blob, entropy).expect("unprotect");
    assert_eq!(plain.as_slice(), b"secret bytes");
}

#[test]
fn tampered_blob_fails() {
    let entropy = b"vidya-test-entropy";
    let mut blob = dpapi_protect(b"secret bytes", entropy).expect("protect");
    let mid = blob.len() / 2;
    blob[mid] ^= 0xFF;
    assert!(dpapi_unprotect(&blob, entropy).is_err());
}

#[test]
fn wrong_entropy_fails() {
    let blob = dpapi_protect(b"secret bytes", b"entropy-a").expect("protect");
    assert!(dpapi_unprotect(&blob, b"entropy-b").is_err());
}
