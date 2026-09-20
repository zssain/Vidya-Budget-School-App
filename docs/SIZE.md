# SIZE.md — shipped and installed size history

| Date | Prompt | Change | Windows installer | Windows installed | macOS arm64 dmg | macOS arm64 installed | macOS x64 dmg | macOS x64 installed | macOS universal dmg | macOS universal installed | Android APK |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 2026-09-17 | P1.1 | React foundation and size gate; kept `opt-level = "z"` (2,535,792-byte binary vs 3,403,024 with `"s"`) | pending current CI | pending VM | 1.42 MB | 2.57 MB | — | — | — | — | pending P8.1 |
| 2026-09-19 | P1.3 | Edition 2 React shell, strict release CSP, five 2 MB log files, and exact platform trait. Separate Mac builds remain the default per D28. | pending current CI; prior Edition 1 CI artifact 1,222,379 bytes | pending VM | 1.46 MB | 2.62 MB | 1.52 MB | 3.25 MB | 2.94 MB | 5.85 MB | pending P8.1 |
| 2026-09-19 | P2.3 | SQLCipher 4.14.0 with CommonCrypto on Apple, vendored OpenSSL selected for Windows/Android; release startup probe forces the backend to link. | pending new CI | pending VM | 2.03 MB (2,031,836 bytes) | 3.58 MB (3492 KiB on disk) | stale P1.3 | stale P1.3 | stale P1.3 | stale P1.3 | pending P8.1 |
| 2026-09-20 | P2.4 | macOS `security-framework` 3.7.0 (Keychain + SecRandom); Windows `windows` 0.61.3 features (DPAPI/known-folder/BCrypt). `security-framework` links the system Security.framework, so the Mac size delta is negligible. | pending new CI | pending VM | 2.07 MB (2,073,176 bytes; +41,340 vs P2.3) | 3.63 MB (3556 KiB on disk) | 1.52 MB | 3.25 MB | 2.94 MB | 5.85 MB | pending P8.1 |
| 2026-09-21 | P2.5 | `vidya-services` foundation: `uuid` 1.26.1, `rand` 0.9.5, `argon2` 0.6.0, `tracing` 0.1.44 (+dev `insta`). `src-tauri` links but does not yet **call** `vidya-services`, so LTO strips these; near-zero delta. Argon2/uuid/rand will add size when P2.6/P2.7 wire real sign-in. | pending new CI | pending VM | 2.07 MB (2,073,757 bytes; +581 vs P2.4) | 3.63 MB (3556 KiB on disk) | stale P1.3 | stale P1.3 | stale P1.3 | stale P1.3 | pending P8.1 |

P1.1/P1.3 Mac values above are the byte-sum results of `npm run size`, rounded to decimal MB; P2.3 ARM installed size is `du -sk` on the mounted DMG. All measured Mac downloads and installed apps are below 30 MB; the P1.3 universal comparison has over 24 MB headroom in both measures. The current Windows installed-folder size is not known until the VM check, so no Windows installed-size compliance claim is made.

P2.3 crypto backend verification: installed `libsqlite3-sys` 0.38.2 `build.rs` selects `SQLCIPHER_CRYPTO_CC` and Apple Security/CoreFoundation with `bundled-sqlcipher` when no `OPENSSL_DIR` is set; `bundled-sqlcipher-vendored-openssl` selects static vendored OpenSSL on non-Apple targets. Its source contains `cipher_memory_security`. `cargo test -p vidya-db -- --nocapture` reported `SQLCipher 4.14.0 community, provider commoncrypto` on Apple Silicon. `r2d2_sqlite` 0.35.0 declares `rusqlite` 0.40, and Cargo.lock resolves both to `rusqlite` 0.40.2. The fully linked ARM release DMG grew 571,214 bytes from its 1,460,622-byte pre-database build (under the P2.3 four-MB stop threshold). Its binary is 3,526,832 bytes. The mounted DMG's `Vidya.app` uses 3492 KiB on disk, below 30 MB. Windows provider and installer size still require a build from these changes in CI.

## Frontend bundles

| Date | Prompt | Bundle | Total `dist/` | JavaScript | JavaScript gzip |
|---|---|---|---:|---:|---:|
| 2026-09-17 | P1.2 | Complete desktop React production build | 400 KiB | 375,692 bytes | 106,570 bytes |
| 2026-09-19 | P1.2 UI repair | Styled desktop React production build; CSS 28,496 bytes | 404 KiB | 377,226 bytes | 106,898 bytes |
