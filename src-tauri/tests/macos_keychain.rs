//! macOS Keychain integration test (P2.4).
//!
//! Uses the real login Keychain, so it is opt-in: run it with
//! `VIDYA_KEYCHAIN_TEST=1 cargo test -p vidya-app --test macos_keychain`.
//! macOS may prompt to allow Keychain access the first time.
#![cfg(target_os = "macos")]

use vidya_app_lib::platform;

#[test]
fn keychain_secret_round_trip() {
    if std::env::var("VIDYA_KEYCHAIN_TEST").as_deref() != Ok("1") {
        eprintln!("skipped: set VIDYA_KEYCHAIN_TEST=1 to run (Keychain may prompt)");
        return;
    }

    let platform = platform::current(std::env::temp_dir().join("vidya-keychain-test"));
    let name = "test-macos-keychain-p24";

    platform
        .store_secret(name, b"hello-secret")
        .expect("store secret");
    let loaded = platform
        .load_secret(name)
        .expect("load secret")
        .map(|secret| secret.to_vec());
    assert_eq!(loaded, Some(b"hello-secret".to_vec()));

    platform.delete_secret(name).expect("delete secret");
    assert!(platform.load_secret(name).expect("load after delete").is_none());
}
