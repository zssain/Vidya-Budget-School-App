//! Step 8: "no private key in any app artifact." The licence signing PRIVATE key
//! must never appear in the app (the app ships only the PUBLIC key in build-config).
//!
//! We cannot build the app binaries in this environment, so we prove it at the
//! source/config level: the dev signing seed (`.dev-keys/ed25519.key`, base64) must
//! not appear anywhere under `src-tauri/` or `src/` (build-config, source, configs).
//! If the dev key file is absent (fresh checkout), the test is a no-op.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // cloud/licence → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").canonicalize().unwrap()
}

fn walk_contains(dir: &Path, needle: &str) -> Option<PathBuf> {
    if !dir.exists() {
        return None;
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(p) = stack.pop() {
        let entries = match std::fs::read_dir(&p) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip build output.
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(name, "target" | "node_modules" | "gen" | "dist") {
                    continue;
                }
                stack.push(path);
            } else {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "rs" | "json" | "ts" | "tsx" | "js" | "toml" | "html") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        if text.contains(needle) {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

#[test]
fn dev_signing_private_seed_is_absent_from_the_app() {
    let seed_file = Path::new(env!("CARGO_MANIFEST_DIR")).join(".dev-keys/ed25519.key");
    let seed_b64 = match std::fs::read_to_string(&seed_file) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            eprintln!("no dev key present ({}); skipping", seed_file.display());
            return;
        }
    };
    assert!(!seed_b64.is_empty());

    let root = repo_root();
    for sub in ["src-tauri", "src"] {
        if let Some(hit) = walk_contains(&root.join(sub), &seed_b64) {
            panic!("licence signing PRIVATE seed leaked into an app file: {}", hit.display());
        }
    }
}

#[test]
fn app_dev_build_config_carries_only_the_public_key() {
    // If the app's dev build-config exists, it must hold `licence_public_key` and
    // must NOT contain the private seed.
    let root = repo_root();
    let dev_json = root.join("src-tauri/build-config/dev.json");
    let text = match std::fs::read_to_string(&dev_json) {
        Ok(t) => t,
        Err(_) => return, // app not present in this checkout
    };
    assert!(text.contains("licence_public_key"), "dev.json should carry the public key");

    let seed_file = Path::new(env!("CARGO_MANIFEST_DIR")).join(".dev-keys/ed25519.key");
    if let Ok(seed) = std::fs::read_to_string(&seed_file) {
        assert!(!text.contains(seed.trim()), "dev.json must not contain the private seed");
    }
}
