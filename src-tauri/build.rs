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
    // `relay_url` is OPTIONAL (the off-by-default "Instant sync" module, §14) and
    // there is no `licence_api` in v2 (licences verify offline — prompts/P12 Step 6).
    // `google_client_id_desktop` / `google_client_id_android` are also OPTIONAL: an
    // empty value means Google Drive is simply unavailable in that build (the app shows
    // a clean "not set up in this build" state — never a fake success), so a school can
    // ship the full local + LAN product before it has an OAuth client. Only the licence
    // public key below is strictly required.

    // At least one licence public key is required, and NONE may be the dev key
    // (prompts/P09 §5, P12 Step 6.4). Mint a production keypair with
    // `tools/licence-maker init` and put its public.key here.
    const DEV_LICENCE_PUBLIC_KEY: &str = "yLQ8lt26cM/ZdKnfaYGS/VgV6DT6CrAyLHS1br28XJs=";
    let keys = json
        .get("licence_public_keys")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let key_strs: Vec<&str> = keys.iter().filter_map(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).collect();
    if key_strs.is_empty() {
        panic!(
            "release build config 'licence_public_keys' is empty in build-config/release.json — \
             add the public key from `tools/licence-maker init` (prompts/P12 Step 6/7)"
        );
    }
    if key_strs.contains(&DEV_LICENCE_PUBLIC_KEY) {
        panic!(
            "release build config uses the DEV licence public key — mint a production keypair \
             (`tools/licence-maker init`) and use its public.key in build-config/release.json \
             (prompts/P09 §5, P12 Step 6.4)"
        );
    }

    // `pwa_origin` is OPTIONAL (empty → no iPhone invite link), but when set it must be
    // a usable public origin (mirrors config::validate_pwa_origin): https, bare origin,
    // not a per-deployment Vercel preview URL (those change each deploy and sit behind
    // Vercel login, so every iPhone invite would break).
    let pwa = json.get("pwa_origin").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !pwa.is_empty() {
        let host = pwa.strip_prefix("https://").unwrap_or_else(|| {
            panic!("release build config 'pwa_origin' must start with https:// (got {pwa})")
        });
        if host.is_empty() || host.contains('/') || pwa.contains("REPLACE") {
            panic!("release build config 'pwa_origin' must be a bare https origin with no path (got {pwa})");
        }
        if let Some(sub) = host.strip_suffix(".vercel.app") {
            if sub.split('-').count() >= 3 {
                panic!("release build config 'pwa_origin' is a Vercel preview URL ({pwa}); use the production domain");
            }
        }
    }
}
