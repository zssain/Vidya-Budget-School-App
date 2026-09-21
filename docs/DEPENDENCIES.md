# DEPENDENCIES.md — the only libraries allowed

Rules:
- Use only what is listed here. To add anything, stop and ask with: purpose, alternatives considered, size impact, licence, last release date.
- Add with `cargo add` / `npm install`. Never type version numbers from memory.
- After adding, fill in the **Installed version** column from `Cargo.lock` / `package-lock.json`, and note the prompt that added it.
- Before first use of any API from these libraries, verify it in the installed version (AGENTS.md section 4).
- Licences must be MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 or Unicode. `cargo deny` checks this from P1.1.

## Rust — application and core
| Crate | Purpose | Used in | Notes to verify | Installed version | Added in | Size impact |
|---|---|---|---|---|---|---|
| tauri | App framework | src-tauri, provider | Enable `tray-icon` feature for desktop | 2.11.5 | P1.1 | |
| tauri-build | Build script | src-tauri, provider | | 2.6.3 | P1.1 | |
| tauri-plugin-dialog | Save and open file dialogs | src-tauri | Desktop | 2.7.3 | P1.3 | |
| tauri-plugin-opener | Open URLs and system settings pages | src-tauri | Desktop | 2.5.5 | P1.3 | |
| tauri-plugin-single-instance | Only one Vidya instance on the office computer | src-tauri | Desktop | 2.4.4 | P1.3 | |
| tauri-plugin-autostart | Start at login | src-tauri | Desktop | | | |
| tauri-plugin-notification | Approval requests, backup warnings | src-tauri | | | | |
| tauri-plugin-log | Logging to file, no personal data | src-tauri | | 2.9.1 | P1.3 | |
| serde, serde_json | Serialisation | all | `derive` feature | serde 1.0.229, serde_json 1.0.151 | P1.1 | |
| thiserror | Error types | all library crates | | 2.0.20 | P1.3 | |
| chrono | Dates and times | core, db, services | `serde` feature; no `now()` inside core (pass time in) | 0.4.45 | P2.1 | |
| uuid | UUID v7 IDs | services | `v7`, `serde` features; `Uuid::now_v7()` | 1.26.1 | P2.5 | in P2.5 Mac size row |
| rusqlite | SQLite access | vidya-db | Apple: `bundled-sqlcipher`/CommonCrypto; other targets: `bundled-sqlcipher-vendored-openssl`. Check Windows and Android builds in CI | 0.40.2 | P2.3 | Fully linked Mac ARM DMG +0.57 MB |
| r2d2, r2d2_sqlite | Connection pool | vidya-db | r2d2_sqlite 0.35.0 resolves rusqlite 0.40.2 | 0.8.10, 0.35.0 | P2.3 | Included in SQLCipher size measurement |
| argon2 | Password and backup key hashing | services | Argon2id m=19456,t=2,p=1; `hash_password_with_salt`; salt from our own `Random` (rand-version independent) | 0.6.0 | P2.5 | in P2.5 Mac size row |
| rand | Random bytes, tokens | services, testkit | 0.9 API: `OsRng.try_fill_bytes`, `StdRng::seed_from_u64`, `random_range`/`random_bool`. Pinned to 0.9 (not 0.10) for argon2 0.6 | 0.9.5 | P2.5 | in P2.5 Mac size row |
| zeroize | Wipe secrets from memory | services, license, backup | | 1.9.0 | P1.3 | |
| sha2 | SHA-256 | services, license, sync, server | Pinned 0.10 (matches transitive; avoids a duplicate 0.11). Session-token hashing | 0.10.9 | P2.6 | not yet in shipped path (auth wired in P2.7) |
| base32 | Crockford base32 for codes | vidya-license | Verify the Crockford alphabet option exists | | | |
| ed25519-dalek | Signatures (license, device keys) | license, server, client | `rand_core` feature where keys are generated | | | |
| chacha20poly1305 | XChaCha20-Poly1305 encryption | backup, provider | | | | |
| zstd | Backup compression | vidya-backup (desktop only) | | | | |
| tokio | Async runtime | server, client, backup | Tauri already uses tokio; match features | | | |
| axum | LAN HTTP server and WebSocket | vidya-server | `ws` feature | | | |
| axum-server | TLS serving for axum | vidya-server | `tls-rustls` feature | | | |
| rustls | TLS | server, client | Use the same major version as axum-server and reqwest | | | |
| rcgen | Self-signed certificate | vidya-server | | | | |
| mdns-sd | mDNS advertise and browse | server, client | On Android needs Wi-Fi multicast lock (Kotlin) | | | |
| reqwest | HTTPS client | vidya-client | `rustls-tls`, no default features; custom certificate verifier for pinning | | | |
| rust_xlsxwriter | Excel export | vidya-export | | | | |
| calamine | Excel import | vidya-export | | | | |
| tracing | Structured logs inside crates | services | `default-features = false`, `std`; used for internal-error refs | 0.1.44 | P2.5 | in P2.5 Mac size row |

## Rust — platform specific
| Crate | Purpose | Platform | Notes to verify | Installed version | Added in | Size impact |
|---|---|---|---|---|---|---|
| windows | DPAPI (CryptProtectData/CryptUnprotectData), known folder, BCryptGenRandom | Windows | Features enabled: `Win32_Security_Cryptography`, `Win32_UI_Shell`, `Win32_System_Com`, `Win32_System_Memory`, `Win32_Foundation`. Verified symbols against source | 0.61.3 | P2.4 | Windows-only; measured in CI |
| winreg | Read MachineGuid | Windows | | | | |
| wmi | Read system UUID | Windows | | | | |
| security-framework | Keychain generic passwords, SecRandom | macOS | "This device only" accessibility not exposed by `passwords::set_generic_password`; using default (see KNOWN_ISSUES) | 3.7.0 | P2.4 | macOS-only; links system Security.framework (negligible) |
| core-foundation, io-kit-sys | IOPlatformUUID, power assertions | macOS | Verify function bindings exist; otherwise declare `extern "C"` with `#[link(name = "IOKit", kind = "framework")]` and say so | | | |
| jni | Only if a Kotlin plugin cannot do the job | Android | Prefer a Tauri mobile plugin in Kotlin | | | |

## Rust — development only
| Crate | Purpose | Installed version | Added in | Size impact |
|---|---|---|---|---|
| tempfile | Temporary databases in tests | 3.27.0 | P2.3 | Dev only |
| proptest | Property tests for money, fees, HLC, merge rules | 1.11.0 | P2.1 | |
| insta | Snapshot tests for DTOs and exports | 1.48.0 | P2.5 | Dev only (`json` feature) |
| tokio (test feature) | Async tests | | | |

## Tools (installed on the MacBook or in CI, not linked into the app)
| Tool | Purpose |
|---|---|
| cargo-deny | Licences and security advisories |
| cargo-bloat | Size audit |
| cargo-nextest (optional) | Faster tests |

## JavaScript
| Package | Purpose | Installed version | Added in | Size impact |
|---|---|---|---|---|
| react | UI components and rendering | 19.3.0 | P1.1 | Included in P1.1 artifact measurement |
| react-dom | Render React into the WebView DOM | 19.3.0 | P1.1 | Included in P1.1 artifact measurement |
| @tauri-apps/cli | Tauri CLI | 2.11.4 | P1.1 | |
| @tauri-apps/api | `invoke` and events | 2.11.1 | P1.1 | |
| @tauri-apps/plugin-dialog | Dialog JS API | 2.7.3 | P1.3 | |
| @tauri-apps/plugin-opener | Opener JS API | 2.5.5 | P1.3 | |
| @tauri-apps/plugin-notification | Notification JS API | | | |
| vite | Frontend build | 8.3.0 | P1.1 | |
| vitest | Frontend tests | 4.1.11 | P1.1 | |
| jsdom | DOM for tests | 29.1.1 | P1.1 | |
| eslint, @eslint/js, globals | Linting | eslint 9.39.5, @eslint/js 9.39.5, globals 17.12.0 | P1.1 | Development only |
| prettier | Formatting | 3.9.6 | P1.1 | |
| @vitejs/plugin-react | Vite JSX transform and React development support | 6.1.1 | P1.1 | Development only |
| eslint-plugin-react | React lint rules | 7.37.5 | P1.1 | Development only |
| eslint-plugin-react-hooks | React Hooks lint rules | 7.1.1 | P1.1 | Development only |
| @testing-library/react | React component tests | 16.3.3 | P1.1 | Development only |
| @testing-library/user-event | User interaction tests | 14.6.7 | P1.1 | Development only |
| @testing-library/jest-dom | DOM matchers for component tests | 7.0.1 | P1.1 | Development only |

React and ReactDOM are the only frontend runtime libraries. No router, state, form, CSS-in-JS or other runtime JavaScript libraries. No CSS frameworks, internet icon fonts or chart libraries (bars are plain CSS like the prototype).

## Kotlin (Android plugin)
Only the Android SDK. No extra Maven libraries unless approved.
