# AGENTS.md — Tauri app crate (src-tauri/)

Read the root `AGENTS.md` first. These rules add to it.

This crate is **thin glue**. It must not contain business rules or SQL.

Server code runs only with a ServerPermit (D29). Never add a way to start the server that bypasses LicenseService.

## What belongs here
- `src/lib.rs` — builds the Tauri app, registers plugins, state and every command in one `generate_handler!` list
- `src/state.rs` — `AppState` holding the service container, session store and platform
- `src/commands/<area>.rs` — one file per area (auth, students, fees, attendance, marks, reports, users, settings, setup, license, backup, devices, sync_status). Each command: read session token, call one service method, map errors to `AppError`, return a DTO.
- `src/platform/mod.rs` — the `Platform` trait; `windows.rs`, `macos.rs`, `android.rs` implement it. See `docs/PLATFORMS.md`.
- `src/background/` — desktop only: LAN server start and stop, backup scheduler, keep-awake
- `capabilities/` — Tauri 2 capability files. Allow only our own commands and the plugins listed in `docs/DEPENDENCIES.md`.

## Rules
- Use `#[cfg(desktop)]` and `#[cfg(mobile)]` for code that exists on only one kind of platform, and the Cargo features in `docs/ARCHITECTURE.md` for optional parts.
- Every new command must also be added to `src/api/commands.js` and `docs/API.md`. `npm run verify` checks this.
- Commands are `async` when they touch the database, and run database work on a blocking thread (`tauri::async_runtime::spawn_blocking`) so the window never freezes.
- Never log DTOs that contain personal data.
