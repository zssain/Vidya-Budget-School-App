fn main() {
    // Keep this list identical to the commands in generate_handler! in lib.rs.
    // P2.7 adds the first application commands and their capability permissions.
    const APP_COMMANDS: &[&str] = &[];
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to build Tauri application metadata");
}
