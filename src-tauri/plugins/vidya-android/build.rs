// No webview-callable commands: the keystore is used ONLY from Rust
// (`PluginHandle::run_mobile_plugin`), which bypasses the IPC ACL. With an empty
// command list no `allow-*` permission even exists, so no capability can ever
// expose `wrap`/`unwrap` to page JavaScript.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
