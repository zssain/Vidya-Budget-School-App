//! Vidya Tauri application.
//!
//! Phase 2 adds the encrypted database, the hash-chained audit log, the
//! single-transaction write helper, and the key/PIN/recovery primitives. No
//! Tauri commands or UI wiring yet (Phase 3). The Phase-1 size spike is gone.

pub mod config;
pub mod db;
pub mod security;
pub mod write;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
