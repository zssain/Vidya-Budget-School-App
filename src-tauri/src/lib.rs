//! Vidya Tauri application entry point.
//!
//! Phase 1 is UI-only: no database, sync or real commands yet. The only
//! registered command is [`spike::size_spike`], which is never called — it
//! exists so the linker keeps every native dependency for the size spike
//! (docs/phase-notes/phase-1.md). Phase 2 deletes `spike.rs`.

mod spike;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![spike::size_spike])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
