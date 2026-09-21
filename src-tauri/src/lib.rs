//! Vidya desktop/mobile application entry (Tauri glue only).
//!
//! This crate is thin glue: it builds the Tauri app and registers plugins,
//! shared state and every command in one `generate_handler!` list. No business
//! rules or SQL live here (see `src-tauri/AGENTS.md`).

#[cfg(desktop)]
pub mod background;
pub mod commands;
pub mod platform;
pub mod state;

#[cfg(target_os = "macos")]
fn macos_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{AboutMetadataBuilder, MenuBuilder, SubmenuBuilder};

    let app_menu = SubmenuBuilder::new(app, "Vidya")
        .about_with_text(
            "About Vidya",
            Some(AboutMetadataBuilder::new().name(Some("Vidya")).build()),
        )
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    // Tauri 2.11.5 exposes the native macOS Zoom item as `maximize`.
    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize_with_text("Zoom")
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &edit_menu, &window_menu])
        .build()
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{log::LevelFilter, RotationStrategy, Target, TargetKind};

    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let mut targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
    }

    // Never log personal data, credentials, secrets, database keys or DTOs.
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets(targets)
        .level(level)
        .max_file_size(2_000_000)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Run the harmless in-memory probe in release too, so the size gate
    // measures the linked SQLCipher backend before the main DB is wired up.
    let cipher_status = vidya_db::sqlcipher_version();
    #[cfg(debug_assertions)]
    match cipher_status {
        Ok((version, provider)) => {
            tauri_plugin_log::log::debug!("SQLCipher {version}, provider {provider}")
        }
        Err(error) => tauri_plugin_log::log::error!("Could not query SQLCipher version: {error}"),
    }
    #[cfg(not(debug_assertions))]
    drop(cipher_status);

    let builder = tauri::Builder::default();

    // The single-instance plugin must be registered before every other plugin.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
            drop(window.show());
            drop(window.unminimize());
            drop(window.set_focus());
        }
    }));

    let builder = builder.plugin(log_plugin());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(target_os = "macos")]
    let builder = builder.menu(macos_menu);

    let builder = builder.manage(state::AppState::new()).setup(|app| {
        use tauri::Manager;
        // Resolve the app data directory, then run the start sequence (data
        // folder → key → open database → build services) on a blocking thread so
        // the window never freezes. The result is published to AppState.
        let state = app.state::<state::AppState>().inner().clone();
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string());
        tauri::async_runtime::spawn_blocking(move || {
            let result = match data_dir {
                Ok(dir) => state::start(platform::current(dir)),
                Err(detail) => state::StartResult::Failed {
                    kind: state::StartupFailure::DataFolder,
                    detail,
                },
            };
            state.publish(result);
        });
        #[cfg(desktop)]
        background::spawn_idle_watcher(app.handle().clone());
        Ok(())
    });

    // The sample-school command exists only in debug builds.
    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app::app_status,
        commands::app::load_sample_school,
        commands::auth::sign_in,
        commands::auth::set_first_password,
        commands::auth::sign_out,
        commands::auth::current_user,
        commands::auth::change_password,
        commands::auth::set_language,
        commands::auth::get_settings,
        commands::users::list_users,
        commands::users::create_user,
        commands::users::update_user,
        commands::users::reset_user_password,
        commands::users::unlock_user,
        commands::users::set_user_active,
        commands::students::list_students,
        commands::students::get_student,
        commands::students::add_student,
        commands::students::update_student,
        commands::students::mark_student_left,
        commands::fees::fee_register,
        commands::fees::get_fee_account,
        commands::fees::collect_fee,
        commands::fees::get_receipt,
        commands::fees::cancel_receipt,
        commands::fees::day_book,
        commands::fees::list_alerts,
        commands::fees::resolve_alert,
    ]);
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app::app_status,
        commands::auth::sign_in,
        commands::auth::set_first_password,
        commands::auth::sign_out,
        commands::auth::current_user,
        commands::auth::change_password,
        commands::auth::set_language,
        commands::auth::get_settings,
        commands::users::list_users,
        commands::users::create_user,
        commands::users::update_user,
        commands::users::reset_user_password,
        commands::users::unlock_user,
        commands::users::set_user_active,
        commands::students::list_students,
        commands::students::get_student,
        commands::students::add_student,
        commands::students::update_student,
        commands::students::mark_student_left,
        commands::fees::fee_register,
        commands::fees::get_fee_account,
        commands::fees::collect_fee,
        commands::fees::get_receipt,
        commands::fees::cancel_receipt,
        commands::fees::day_book,
        commands::fees::list_alerts,
        commands::fees::resolve_alert,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
