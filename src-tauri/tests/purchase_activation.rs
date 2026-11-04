//! App side of the purchase → activation rehearsal (scripts/rehearse-purchase.sh,
//! docs/implementation/runbooks/purchase-rehearsal.md). Both tests are `#[ignore]`:
//! they need the rehearsal directory `VIDYA_REHEARSAL_DIR` and are driven by the
//! script in two phases:
//!
//! 1. `rehearsal_1_fake_machines` — before checkout: creates two FAKE machines with
//!    the app's real machine-id generator (`licence::machine::get_or_create` over a
//!    file store) and writes their machine codes (the app's real
//!    `vidya_core::licence::machine_code`) for the checkout order.
//! 2. `rehearsal_2_activate` — after the real billing worker + Rust signer issued
//!    the licences: activates the stored `.vlic` / licence key with the app's real
//!    activation logic (`activate_licence_with_keys`, what `activate_licence` runs)
//!    against the rehearsal's TEST public key, on a fresh encrypted DB.
//!
//! TEST KEYS AND FAKE SCHOOLS ONLY.

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::Connection;

use vidya_lib::commands::logic::activate_licence_with_keys;
use vidya_lib::licence::machine::{get_or_create, FileMachineStore};
use vidya_lib::state::{self, AppState, KV_PENDING_LICENCE};
use vidya_lib::{db, kv};

const DB_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn dir() -> PathBuf {
    PathBuf::from(std::env::var("VIDYA_REHEARSAL_DIR").expect("VIDYA_REHEARSAL_DIR (run scripts/rehearse-purchase.sh)"))
}

fn read(d: &Path, name: &str) -> String {
    std::fs::read_to_string(d.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

fn machine_id(d: &Path, which: &str) -> String {
    get_or_create(&FileMachineStore::new(d.join(format!("{which}.machine-id")))).unwrap()
}

/// A brand-new install: fresh encrypted DB, migrated, no school, no licence.
fn fresh_install() -> Connection {
    let mut c = db::open_in_memory(DB_KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    assert_eq!(state::compute(&c, None).unwrap().state, AppState::NoSchool);
    c
}

fn test_public_key(d: &Path) -> [u8; 32] {
    let raw = STANDARD.decode(read(d, "test-public.key").trim()).expect("public key base64");
    raw.try_into().expect("32-byte ed25519 public key")
}

fn pending_licence_id(c: &Connection) -> Option<String> {
    kv::get::<serde_json::Value>(c, KV_PENDING_LICENCE)
        .unwrap()
        .and_then(|v| v.get("licence_id").and_then(|x| x.as_str()).map(str::to_string))
}

#[test]
#[ignore = "purchase rehearsal phase 1 — run via scripts/rehearse-purchase.sh"]
fn rehearsal_1_fake_machines() {
    let d = dir();
    std::fs::create_dir_all(&d).unwrap();
    for which in ["buyer", "other"] {
        let id = machine_id(&d, which);
        let code = vidya_core::licence::machine_code(&id);
        std::fs::write(d.join(format!("{which}.machine-code")), &code).unwrap();
        println!("{which}: machine code {code}");
    }
    assert_ne!(read(&d, "buyer.machine-code"), read(&d, "other.machine-code"));
}

#[test]
#[ignore = "purchase rehearsal phase 2 — run via scripts/rehearse-purchase.sh"]
fn rehearsal_2_activate() {
    let d = dir();
    let pk = test_public_key(&d);
    let buyer = machine_id(&d, "buyer");
    let other = machine_id(&d, "other");
    let expected: serde_json::Value = serde_json::from_str(&read(&d, "issued.json")).unwrap();

    for case in ["purchase", "retry"] {
        let want_id = expected[case]["licence_id"].as_str().unwrap().to_string();
        let vlic = read(&d, &format!("{case}.vlic"));
        let key = read(&d, &format!("{case}.licence-key.txt"));

        // The .vlic file AND the pasted grouped key both activate on the buyer's PC.
        for (label, text) in [(".vlic", &vlic), ("licence key", &key)] {
            let mut c = fresh_install();
            let res = activate_licence_with_keys(&mut c, &buyer, text, &[pk], None)
                .unwrap_or_else(|e| panic!("{case} {label} must activate: {}", e.code));
            assert_eq!(res.state, AppState::Activated, "{case} {label}");
            assert_eq!(pending_licence_id(&c).as_deref(), Some(want_id.as_str()), "{case} {label}: pending licence stored");
            println!("{case} {label}: ACTIVATED on buyer machine ({want_id})");
        }

        // A different computer: correctly signed, but bound elsewhere.
        let mut c = fresh_install();
        let e = activate_licence_with_keys(&mut c, &other, &vlic, &[pk], None).unwrap_err();
        assert_eq!(e.code, "LICENCE_OTHER_MACHINE", "{case}: other machine");
        assert_eq!(pending_licence_id(&c), None, "nothing stored on rejection");
        assert_eq!(state::compute(&c, None).unwrap().state, AppState::NoSchool);
        println!("{case}: rejected on other machine (LICENCE_OTHER_MACHINE)");

        // Tampered: change the school name inside the signed payload, keep the signature.
        let (payload, sig) = vidya_core::licence::parse_licence_key(&vlic).unwrap();
        let text = String::from_utf8(payload).unwrap();
        assert!(text.contains("Rehearsal"), "fake school name in payload: {text}");
        let forged = text.replace("Rehearsal", "Rehearsa1");
        let tampered = vidya_core::licence::encode_licence_key(forged.as_bytes(), &sig);
        let mut c = fresh_install();
        let e = activate_licence_with_keys(&mut c, &buyer, &tampered, &[pk], None).unwrap_err();
        assert_eq!(e.code, "LICENCE_INVALID", "{case}: tampered payload");
        // Tampered signature byte.
        let mut bad_sig = sig.clone();
        bad_sig[10] ^= 0x01;
        let tampered = vidya_core::licence::encode_licence_key(text.as_bytes(), &bad_sig);
        let e = activate_licence_with_keys(&mut c, &buyer, &tampered, &[pk], None).unwrap_err();
        assert_eq!(e.code, "LICENCE_INVALID", "{case}: tampered signature");
        // A build carrying a different public key never accepts it.
        let e = activate_licence_with_keys(&mut c, &buyer, &vlic, &[[7u8; 32]], None).unwrap_err();
        assert_eq!(e.code, "LICENCE_INVALID", "{case}: other public key");
        assert_eq!(pending_licence_id(&c), None);
        println!("{case}: tampered / wrong-key rejected (LICENCE_INVALID)");
    }
}
