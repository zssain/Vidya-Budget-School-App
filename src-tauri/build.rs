use std::path::Path;

fn main() {
    // A release build must never ship blank build config (prompts/P03 Step 1).
    // dev builds may leave values empty (e.g. licence_public_key before the dev
    // licence service has generated a keypair).
    let profile = std::env::var("PROFILE").unwrap_or_default();
    println!("cargo:rerun-if-changed=build-config/dev.json");
    println!("cargo:rerun-if-changed=build-config/release.json");
    if profile == "release" {
        check_release_config();
    }

    tauri_build::build()
}

fn check_release_config() {
    let path = Path::new("build-config/release.json");
    if !path.exists() {
        panic!(
            "release build config missing: {} — copy build-config/release.json.example \
             to build-config/release.json and fill every value (docs §10, prompts/P03 Step 1)",
            path.display()
        );
    }
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let json: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("build-config/release.json is not valid JSON: {e}"));
    for key in [
        "licence_api",
        "licence_public_key",
        "relay_url",
        "google_client_id_desktop",
        "google_client_id_android",
    ] {
        let val = json.get(key).and_then(|v| v.as_str()).unwrap_or("");
        if val.trim().is_empty() {
            panic!(
                "release build config value '{key}' is empty in build-config/release.json — \
                 fill it before a release build (prompts/P03 Step 1)"
            );
        }
    }
}
