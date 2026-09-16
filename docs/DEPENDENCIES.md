# DEPENDENCIES.md — the only libraries allowed

Rules:
- Use only what is listed here. To add anything, stop and ask with: purpose, alternatives considered, size impact, licence, last release date.
- Add with `cargo add` / `npm install`. Never type version numbers from memory.
- After adding, fill in the **Installed version** column from `Cargo.lock` / `package-lock.json`, and note the prompt that added it.
- Before first use of any API from these libraries, verify it in the installed version (AGENTS.md section 4).
- Licences must be MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 or Unicode. `cargo deny` checks this from P1.1.

## Rust — application and core
| Crate | Purpose | Used in | Notes to verify | Installed version | Added in |
|---|---|---|---|---|---|
| tauri | App framework | src-tauri, provider | Enable `tray-icon` feature for desktop | 2.11.5 | P1.1 |
| tauri-build | Build script | src-tauri, provider | | 2.6.3 | P1.1 |
| tauri-plugin-dialog | Save and open file dialogs | src-tauri | Desktop | 2.7.3 | P1.3 |
| tauri-plugin-opener | Open URLs and system settings pages | src-tauri | Desktop | 2.5.5 | P1.3 |
| tauri-plugin-single-instance | Only one Vidya instance on the office computer | src-tauri | Desktop | 2.4.4 | P1.3 |
| tauri-plugin-autostart | Start at login | src-tauri | Desktop | | |
| tauri-plugin-notification | Approval requests, backup warnings | src-tauri | | | |
| tauri-plugin-log | Logging to file, no personal data | src-tauri | | 2.9.1 | P1.3 |
| serde, serde_json | Serialisation | all | `derive` feature | serde 1.0.229, serde_json 1.0.151 | P1.1 |
| thiserror | Error types | all library crates | | 2.0.20 | P1.3 |
| chrono | Dates and times | core, db, services | `serde` feature; no `now()` inside core (pass time in) | | |
| uuid | UUID v7 IDs | core, db | `v7`, `serde` features | | |
| rusqlite | SQLite access | vidya-db | Feature `bundled-sqlcipher-vendored-openssl`. Check Windows and Android builds in CI | | |
| r2d2, r2d2_sqlite | Connection pool | vidya-db | r2d2_sqlite version must match rusqlite | | |
| argon2 | Password and backup key hashing | services, backup, provider | | | |
| rand | Random bytes, tokens | services, license, backup | Use the OS-backed generator | | |
| zeroize | Wipe secrets from memory | services, license, backup | | 1.9.0 | P1.3 |
| sha2 | SHA-256 | license, sync, server | | | |
| base32 | Crockford base32 for codes | vidya-license | Verify the Crockford alphabet option exists | | |
| ed25519-dalek | Signatures (license, device keys) | license, server, client | `rand_core` feature where keys are generated | | |
| chacha20poly1305 | XChaCha20-Poly1305 encryption | backup, provider | | | |
| zstd | Backup compression | vidya-backup (desktop only) | | | |
| tokio | Async runtime | server, client, backup | Tauri already uses tokio; match features | | |
| axum | LAN HTTP server and WebSocket | vidya-server | `ws` feature | | |
| axum-server | TLS serving for axum | vidya-server | `tls-rustls` feature | | |
| rustls | TLS | server, client | Use the same major version as axum-server and reqwest | | |
| rcgen | Self-signed certificate | vidya-server | | | |
| mdns-sd | mDNS advertise and browse | server, client | On Android needs Wi-Fi multicast lock (Kotlin) | | |
| reqwest | HTTPS client | vidya-client, vidya-backup (Drive) | `rustls-tls`, no default features; custom certificate verifier for pinning | | |
| oauth2 | Google OAuth with PKCE | vidya-backup | | | |
| rust_xlsxwriter | Excel export | vidya-export | | | |
| calamine | Excel import | vidya-export | | | |
| tracing | Structured logs inside crates | all | Bridge to tauri-plugin-log | | |

## Rust — platform specific
| Crate | Purpose | Platform | Notes to verify | Installed version | Added in |
|---|---|---|---|---|---|
| windows | DPAPI (CryptProtectData), SetThreadExecutionState | Windows | Enable only the needed `Win32_*` features | | |
| winreg | Read MachineGuid | Windows | | | |
| wmi | Read system UUID | Windows | | | |
| security-framework | Keychain items | macOS | Check how to set "this device only" accessibility | | |
| core-foundation, io-kit-sys | IOPlatformUUID, power assertions | macOS | Verify function bindings exist; otherwise declare `extern "C"` with `#[link(name = "IOKit", kind = "framework")]` and say so | | |
| jni | Only if a Kotlin plugin cannot do the job | Android | Prefer a Tauri mobile plugin in Kotlin | | |

## Rust — development only
| Crate | Purpose |
|---|---|
| tempfile | Temporary databases in tests |
| proptest | Property tests for money, fees, HLC, merge rules |
| insta | Snapshot tests for DTOs and exports |
| tokio (test feature) | Async tests |

## Tools (installed on the MacBook or in CI, not linked into the app)
| Tool | Purpose |
|---|---|
| cargo-deny | Licences and security advisories |
| cargo-bloat | Size audit |
| cargo-nextest (optional) | Faster tests |

## JavaScript
| Package | Purpose | Installed version | Added in |
|---|---|---|---|
| @tauri-apps/cli | Tauri CLI | 2.11.4 | P1.1 |
| @tauri-apps/api | `invoke` and events | 2.11.1 | P1.1 |
| @tauri-apps/plugin-dialog | Dialog JS API | 2.7.3 | P1.3 |
| @tauri-apps/plugin-opener | Opener JS API | 2.5.5 | P1.3 |
| @tauri-apps/plugin-notification | Notification JS API | | |
| vite | Frontend build | 8.3.0 | P1.1 |
| vitest | Frontend tests | 4.1.11 | P1.1 |
| jsdom | DOM for tests | 29.1.1 | P1.1 |
| eslint, @eslint/js, globals | Linting | eslint 10.10.0, @eslint/js 10.0.1, globals 17.12.0 | P1.1 |
| prettier | Formatting | 3.9.6 | P1.1 |

No other runtime JavaScript libraries. No CSS frameworks, no icon fonts from the internet, no chart libraries (bars are plain CSS like the prototype).

## Kotlin (Android plugin)
Only the Android SDK. No extra Maven libraries unless approved.
