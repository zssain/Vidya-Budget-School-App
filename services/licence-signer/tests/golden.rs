//! Byte-for-byte compatibility with tools/licence-maker, and acceptance by the
//! app's verifier (`vidya_core::licence::verify_v2`). Test keys only.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use licence_signer::artefact::{group5, sign_licence, LicenceInputs};
use licence_signer::key::{fingerprint, read_key_file, read_public_key_file, Signer};
use vidya_core::licence::{machine_code, parse_licence_key, verify_v2, LicenceV2};

/// The golden key pinned by BOTH tools/licence-maker (`golden_vector_is_stable`)
/// and crates/vidya-core (`golden_licence_key_from_maker_verifies`): seed [42;32],
/// machine_id "golden-machine", the fixture below.
const GOLDEN_KEY: &str = "eyJ2IjoyLCJsaWNlbmNlX2lkIjoiMDE5M2I2YzAtMDAwMC03MDAwLTgwMDAtMDAwMDAwMDAwMDAxIiwic2Nob29sX25hbWUiOiJTYXJhc3dhdGkgUHVibGljIFNjaG9vbCIsIm1hY2hpbmVfY29kZSI6IjNOMFMtMTBZWC1KUUNSLVQiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTI1VDAwOjAwOjAwWiIsInBsYW4iOiJwZXJwZXR1YWwiLCJtb2R1bGVzIjpbImNvcmUiXX0sel8zbFpx8xCuDxYhHoM9kZ6raBqbSkf-O3JKmdQ5t2OK9Z0ZYQO3RhjJdKu2gdWndFijhL48Powb0xSDAIoD";

fn golden_inputs(typed_machine: &str) -> LicenceInputs {
    LicenceInputs {
        licence_id: "0193b6c0-0000-7000-8000-000000000001".into(),
        school_name: "Saraswati Public School".into(),
        machine_code: typed_machine.into(),
        issued_at: "2026-09-25T00:00:00Z".into(),
        modules: vec!["core".into()],
    }
}

#[test]
fn golden_vector_matches_licence_maker_and_verifies() {
    let mc = machine_code("golden-machine");
    assert_eq!(mc, "3N0S-10YX-JQCR-T");
    let signer = common::test_signer(42, "test-k1");

    // Also typed messily (lower-case, spaces): canonicalised exactly as the maker does.
    for typed in [mc.clone(), mc.to_lowercase().replace('-', " ")] {
        let a = sign_licence(&golden_inputs(&typed), &signer).unwrap();
        assert_eq!(
            a.vlic, GOLDEN_KEY,
            ".vlic contents must equal licence-maker's compact key"
        );
        assert_eq!(a.licence_key, group5(GOLDEN_KEY));
        assert!(a.licence_key.split(' ').all(|g| g.len() <= 5));

        let (payload, sig) = parse_licence_key(&a.licence_key).unwrap();
        assert_eq!(payload, a.payload);
        assert_eq!(sig, a.signature);
        let lic = verify_v2(&payload, &sig, signer.public_key(), &mc).unwrap();
        assert_eq!(lic, a.licence);
        assert_eq!(lic.plan, "perpetual");
        assert_eq!(lic.modules, vec!["core".to_string()]);
    }
}

#[test]
fn other_public_key_does_not_verify() {
    let signer = common::test_signer(42, "test-k1");
    let other = common::test_signer(43, "test-k2");
    let a = sign_licence(&golden_inputs("3N0S-10YX-JQCR-T"), &signer).unwrap();
    assert!(verify_v2(&a.payload, &a.signature, other.public_key(), "3N0S-10YX-JQCR-T").is_err());
}

#[test]
fn invalid_inputs_are_not_signed() {
    let signer = common::test_signer(42, "test-k1");
    let ok = golden_inputs("3N0S-10YX-JQCR-T");
    let cases: Vec<LicenceInputs> = vec![
        LicenceInputs {
            machine_code: "3N0S-10YX-JQCR-X".into(),
            ..ok.clone()
        }, // bad checksum
        LicenceInputs {
            machine_code: "3N0S-10YX".into(),
            ..ok.clone()
        },
        LicenceInputs {
            modules: vec!["accounts".into()],
            ..ok.clone()
        }, // no core
        LicenceInputs {
            modules: vec!["core".into(), "chess".into()],
            ..ok.clone()
        },
        LicenceInputs {
            issued_at: "yesterday".into(),
            ..ok.clone()
        },
        LicenceInputs {
            licence_id: "".into(),
            ..ok.clone()
        },
        LicenceInputs {
            school_name: "  ".into(),
            ..ok.clone()
        },
        LicenceInputs {
            school_name: "A\nB".into(),
            ..ok.clone()
        },
    ];
    for c in cases {
        assert!(sign_licence(&c, &signer).is_err(), "should refuse {c:?}");
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// End-to-end against the REAL licence-maker binary: `init` a throw-away key
/// folder, `issue` a licence, then load that folder's `licence-signing.key` with
/// the signer's loader (proving the key-file format), sign the same fields, and
/// compare the .vlic bytes and the printed grouped key exactly.
#[test]
fn real_licence_maker_output_is_reproduced_byte_for_byte() {
    let root = repo_root();
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("licence-maker-target");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--manifest-path"])
        .arg(root.join("tools/licence-maker/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .status()
        .expect("run cargo build for licence-maker");
    assert!(status.success(), "building tools/licence-maker failed");
    let bin = target.join("debug").join(if cfg!(windows) {
        "licence-maker.exe"
    } else {
        "licence-maker"
    });

    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(&bin)
        .args(["init", "--dir"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mc = machine_code("cross-check-pc");
    let typed = mc.to_lowercase().replace('-', " ");
    let school = "Sri Vidyā \"Test\", School";
    let out = Command::new(&bin)
        .args([
            "issue",
            "--school",
            school,
            "--machine",
            &typed,
            "--utr",
            "UTR-TEST-1",
            "--email",
            "t@example.test",
            "--dir",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "issue failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    let idx = lines
        .iter()
        .position(|l| l.starts_with("Licence key"))
        .expect("key header");
    let maker_grouped = lines[idx + 1].trim().to_string();

    let vlic_path = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "vlic"))
        .expect(".vlic written");
    let maker_vlic = std::fs::read_to_string(&vlic_path).unwrap();
    let (payload, _sig) = parse_licence_key(&maker_vlic).unwrap();
    let maker_lic: LicenceV2 = serde_json::from_slice(&payload).unwrap();

    // The maker's key folder, loaded through the signer's own loaders.
    let key = read_key_file(&dir.path().join("licence-signing.key")).expect("licence-maker key file loads");
    let public = read_public_key_file(&dir.path().join("public.key")).unwrap();
    assert_eq!(key.verifying_key().to_bytes(), public);
    let signer = Signer::new(key, "cross-check", &[fingerprint(&public)]).unwrap();

    let a = sign_licence(
        &LicenceInputs {
            licence_id: maker_lic.licence_id.clone(),
            school_name: school.into(),
            machine_code: typed.clone(),
            issued_at: maker_lic.issued_at.clone(),
            modules: vec!["core".into()],
        },
        &signer,
    )
    .unwrap();
    assert_eq!(a.payload, payload, "payload JSON bytes differ from licence-maker");
    assert_eq!(a.vlic, maker_vlic, ".vlic differs from licence-maker");
    assert_eq!(a.licence_key, maker_grouped, "grouped key differs from licence-maker");
    verify_v2(&a.payload, &a.signature, &public, &mc).unwrap();
}
