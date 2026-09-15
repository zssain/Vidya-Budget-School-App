//! Vidya desktop/mobile application entry (Tauri glue only).
//!
//! This crate is thin glue: it builds the Tauri app and, in later prompts,
//! registers plugins, shared state and every command in one `generate_handler!`
//! list. No business rules or SQL live here (see `src-tauri/AGENTS.md`).

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
