# Android DB key in AndroidKeyStore (repair K1, spec Phase 9 — Android part)

Owner decision: an in-repo Kotlin Tauri plugin (`src-tauri/plugins/vidya-android`), not the
`jni` crate. Desktop behaviour is unchanged (OS keychain via `keyring`).

## What is stored where

| File (app data dir) | Contents | Who writes it |
|---|---|---|
| `db-key.wrapped.json` | `{"v":1,"iv":"<b64>","ct":"<b64 ciphertext+GCM tag>"}` — the 32-byte DB key wrapped by AndroidKeyStore. No plaintext. | `KeystoreWrappedStore` (fresh install, migration, restore) |
| `db-key.wrapped.json.tmp` | Transient copy during an atomic write (wrapped blob only). Removed at every start. | same |
| `db-key` | **Legacy** plaintext hex key (builds before K1). Read only to migrate it; deleted once the wrapped copy is proven to open the DB. Also the fallback if the keystore cannot wrap (no regression). | older builds; fallback only |
| `vidya.db` | SQLCipher DB, keyed with the DB key. | app |

AndroidKeyStore holds one AES-256-GCM key, alias `vidya.dbkey.wrap.v1`, non-exportable, purpose
encrypt+decrypt, randomized encryption required (fresh IV per wrap), no user-auth binding (the
DB must open at start; the app has its own PIN lock). AAD = `vidya.dbkey|v=1|alias=vidya.dbkey.wrap.v1`
binds blob version and alias to the ciphertext. `android:allowBackup="false"` already keeps the
files out of cloud/adb backups.

## Components

- **Kotlin** `app.vidya.plugin.KeystorePlugin` (`plugins/vidya-android/android/…/KeystorePlugin.kt`):
  - `wrap({plaintextB64})` → `{v, iv, ct}`. Creates the keystore key on first use. If Android
    reports the key permanently invalidated, it deletes and regenerates it (any blob under it is
    unrecoverable anyway). Plaintext bytes are zeroed after use.
  - `unwrap({v, iv, ct})` → `{plaintextB64}`. Never deletes or creates anything. Rejects with
    `KEY_INVALIDATED` (KeyPermanentlyInvalidatedException), `KEY_MISSING` (alias gone),
    `BAD_BLOB` (GCM tag failure / unknown version / bad base64) — all **permanent** — or
    `KEYSTORE_UNAVAILABLE` (anything else) — **transient**.
  - No plaintext in logs, SharedPreferences, files or error messages (messages carry exception
    class names only).
- **Rust plugin crate** `tauri-plugin-vidya-android` (`plugins/vidya-android/src/lib.rs`):
  registers the Kotlin class, exposes `Keystore::{wrap, unwrap}` (blocking
  `run_mobile_plugin`) with errors mapped to `Permanent{code}` / `Transient`. It has **no
  webview-callable commands**: `build.rs` declares an empty command list (so no `allow-*`
  permission exists to grant) and the plugin's invoke handler rejects every webview call. Rust
  reaches it via `PluginHandle::run_mobile_plugin`, which does not go through the IPC ACL.
  Hence no capability change.
- **App** `src-tauri/src/security/android_key.rs`:
  - `trait Wrapper { wrap, unwrap }` — the only thing that touches the keystore; real impl
    `PluginWrapper` (Android only), fake in tests.
  - `KeystoreWrappedStore<W>: KeyStore` — `get`: wrapped key, else legacy plaintext, else
    `None` (permanently unusable) / `Backend` error (transient). `set` (fresh install, restore):
    wrap → atomic write → read back + unwrap + compare → remove stale plaintext; if the keystore
    cannot wrap, write the legacy file instead (today's behaviour) and drop any stale wrapped file.
  - `migrate(db_check)` — the migration state machine below.
  - `boot(store, db_path)` — called by `lib.rs build_ctx` on Android: `migrate`, then return
    whichever stored key actually opens the DB, else `ensure_key` (generates a key **only when
    no DB file exists**; with a DB and no key → `KeyError::Missing` → existing Recover screen).
- **Wiring**: `lib.rs` registers `tauri_plugin_vidya_android::init()` in `setup` (Android only)
  before `build_ctx`; `restore_install` uses the same store on Android. Desktop paths untouched.

## Migration state machine (each launch)

Inputs: `P` = legacy plaintext key (if `db-key` exists and parses), `W` = what
`db-key.wrapped.json` yields (Absent / Key(k) / Unusable / Transient), `opens(k)` = the existing DB
opens and reads with `k` (`db::open_encrypted` + `SELECT count(*) FROM sqlite_master`; always true
when no DB file exists).

```
start: delete stale db-key.wrapped.json.tmp
no P ............................................ NotNeeded
W = Transient ................................... Deferred (keep P; retry next launch)
W = Key(k), k == P:  opens(k) ? delete P → Migrated : Deferred
W = Key(k), k != P:  DB exists ∧ opens(k) ∧ ¬opens(P) ? delete stale P → Migrated
                     ¬opens(P) ? Deferred (never guess)
                     else fall through (P is live; re-wrap it)
W = Absent | Unusable | (fall-through):
  1 wrap(P)                       fail → Deferred
  2 write .tmp + fsync            fail → rm .tmp, Deferred
    rename .tmp → wrapped + fsync dir   fail → rm .tmp, Deferred
  3 read wrapped back, unwrap → k2; k2 == P ?      no → rm wrapped, Deferred
  4 opens(k2) ?                                     no → rm wrapped, Deferred
  5 delete P (+ fsync dir)        fail → Deferred (both copies stay; next launch finishes at "k == P")
                                  ok → Migrated
```

After `migrate`, `boot` picks the first of [wrapped key, plaintext key] that opens the DB, so the
app always uses the right key even when both files exist. If neither opens it, it falls back to
`get()` exactly as before K1 (the open then fails visibly; nothing is deleted).

### Crash / failure guarantees (all covered by tests)

| Interruption | On disk afterwards | Next launch |
|---|---|---|
| After wrap, before any write | `db-key` only | migrates normally |
| After temp write, before rename | `db-key` + `.tmp` (wrapped blob only) | `.tmp` removed, migrates |
| After rename, before verify | `db-key` + wrapped | `k == P` → verify → delete → Migrated |
| Verify fails | `db-key` only (wrapped removed) | retries; plaintext keeps working |
| Round-trip mismatch | `db-key` only | retries |
| Delete of `db-key` fails | both | finishes the delete, no re-wrap |
| Keystore busy (wrap or unwrap) | unchanged | plaintext keeps working / retried |
| Keystore invalidated, `db-key` still present | both | re-wrapped under a new keystore key |
| Keystore invalidated, no `db-key`, DB exists | unchanged (blob and DB never touched) | `Missing` → **Recover** (restore from backup) |
| Transient unwrap error, no `db-key` | unchanged | `Backend` error → DB unavailable screen, no Recover, retried next launch |
| Corrupt wrapped JSON, no `db-key` | unchanged | `Missing` → Recover; file left in place |
| Wrapped blob from a newer version (`v != 1`) | unchanged | treated as transient (not "lost") |

Invariant: the plaintext key file is deleted only after a wrapped copy, read back from disk and
unwrapped by the keystore, has opened the existing DB. No code path creates a key while
`vidya.db` exists.

## Tests (desktop, `cargo test -p vidya --lib security::android_key`)

18 tests with a fake keystore (`FakeKs`: models generation, invalidation, busy, lying
round-trip) and **real SQLCipher DB files** in a temp dir: fresh install (wrapped only; keystore
down → plaintext fallback, then migrated), migration success (plaintext still present while the
DB is verified; removed after), simulated crash at each of the 4 steps then relaunch, verify
failure, wrong plaintext key never deleted, round-trip mismatch, delete failure, wrap
unavailable, invalidated keystore → Missing with blob and DB byte-identical, invalidated with
plaintext present → re-wrap, transient unwrap → Backend (not Missing) with nothing touched,
transient during migration, corrupt / newer blob untouched, `set` replacing a key (restore),
`set` with keystore down, disagreeing copies resolved by the DB.

## Build verification (31 Oct 2026, macOS, NDK 26.3.11579264, JDK 21, Gradle 8.14.3)

| Check | Result |
|---|---|
| `cargo test -p vidya --lib --locked` | 344 passed, 0 failed (incl. the 18 above) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean (the plugin crate is a workspace member and is linted on desktop) |
| `cargo check -p vidya --lib --target aarch64-linux-android --locked` | OK, with `ANDROID_NDK_HOME`, `CC_/CXX_/AR_/RANLIB_aarch64_linux_android` (NDK `aarch64-linux-android24-clang`, `llvm-ar`, `llvm-ranlib`) and `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` set |
| `cargo clippy -p vidya -p tauri-plugin-vidya-android --lib --target aarch64-linux-android -- -D warnings` | clean (covers the `#[cfg(target_os = "android")]` code) |
| `tauri android build --debug --target aarch64 --apk --ci` (stub frontend via `--config`, because the worktree had no `node_modules`) | **APK built.** The first attempt failed because vendored OpenSSL could not find `aarch64-linux-android-ranlib`; it succeeded once `AR_aarch64_linux_android` / `RANLIB_aarch64_linux_android` pointed at NDK `llvm-ar` / `llvm-ranlib`. Generated `tauri.settings.gradle` includes `:tauri-plugin-vidya-android`. The Kotlin plugin compiled, and `Lapp/vidya/plugin/KeystorePlugin;` plus the alias string are in `classes4.dex`. |
| Release (R8-minified) build | not run |
| Emulator / device run | Emulator: **run 5 Nov 2026**, see next section. Physical device: **not run**. |

## Emulator verification (5 Nov 2026) — emulator, NOT a physical device

**Environment.** Apple M5, macOS 26.2, Hypervisor.Framework acceleration. Installed with Google's
`sdkmanager` (Homebrew `android-commandlinetools`, `--sdk_root=~/Library/Android/sdk`):
`cmdline-tools;latest` and `system-images;android-34;google_apis;arm64-v8a` (rev 14). AVD
`vidya_api34` (`avdmanager create avd -k "system-images;android-34;google_apis;arm64-v8a" -d pixel_6`),
booted headless: `emulator -avd vidya_api34 -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect`
(Android Emulator 37.1.11). Device: Android 14, API 34, `google/sdk_gphone64_arm64/emu64a:14/UE1A.230829.050/12077443:userdebug`.

**APK.** Same env as above (NDK 26.3, JDK 21, `CC_/CXX_/AR_/RANLIB_aarch64_linux_android`,
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`), real frontend (`npm ci`), then
`npm run tauri android build -- --debug --apk --target aarch64 --ci` →
`app-universal-debug.apk`, package `in.vidyabudget.app.debug`, data dir
`/data/data/in.vidyabudget.app.debug/` (key files and `vidya.db` live directly in it).

**"Old" (pre-K1) build.** Same tree with only the Android branch of `build_ctx` switched back to
the exact pre-K1 code (`ensure_key(&FileKeyStore::new(data_dir.join("db-key")), …)`, as in
`08e95e6^`), same `versionCode`, so `adb install -r` is a real in-place upgrade with the old
on-disk layout (64-hex `db-key`, no wrapped file). Not a build of `391e8c4` itself.

**How it was driven.** `adb` (`~/Library/Android/sdk/platform-tools/adb`); files via
`adb shell run-as in.vidyabudget.app.debug ls/cat/sha256sum/cp`; the app via the debug
WebView's DevTools socket (`adb forward tcp:9333 localabstract:webview_devtools_remote_<pid>`,
CDP `Runtime.evaluate` of `window.__TAURI_INTERNALS__.invoke(...)`): `seed_demo_school` (debug-only),
`app_state`, the setup commands, `unlock`, `list_students`, `backup_now`, `restore_summary`,
`restore_install`. Keystore calls were confirmed in logcat (`Tauri/Plugin: … pluginId: vidya-android, command: wrap|unwrap`);
the migration outcome in the app's own log `logs/vidya-<date>.log`.

| # | Test | Result |
|---|---|---|
| 1 | Fresh install, first launch | **PASS.** One `wrap` + one `unwrap`; only `db-key.wrapped.json` (103 B, mode 600, content `{"v":1,"iv":"…","ct":"…"}` only); **no `db-key`**. Seeded a school; force-stop + relaunch → only `unwrap`, `app_state` = `setup_in_progress`, licence `active` (data present). `adb reboot` → still opens, same state. |
| 2 | Upgrade from the plaintext build (`adb install -r`) | **PASS.** Old build: `db-key` (64 hex) + DB, no wrapped file. New build first launch: `wrap`, `unwrap`, `unwrap`; log `DB key moved into AndroidKeyStore; plaintext key file removed`; `db-key` gone, wrapped file present, same school state. Relaunch → `unwrap` only, data present. |
| 3 | Interrupted upgrade (time-based) | **PASS.** Pre-K1 state restored before each run, `am start` then `am force-stop` after 0.1/0.3/0.5/0.7/0.9/1.0/1.1/1.2/1.4/1.7/2.0 s. Kills ≤1.7 s left `db-key` only (migration not reached yet); 2.0 s left the migrated state. Every relaunch: wrapped only, school opens with its data. |
| 3' | Interrupted upgrade (exact points) | **PASS.** A same-uid on-device loop (`run-as … kill -9 $(pidof …)`) killed the app the instant `db-key.wrapped.json.tmp` / `db-key.wrapped.json` appeared (+0/0.02/0.05/0.1 s). Seen on disk after the kill: `db-key` **and** `db-key.wrapped.json` (after rename, before delete) in 5 of 5 runs at ≤0.05 s; at 0.1 s already migrated. Each relaunch finished the migration (plaintext deleted) and opened the DB. |
| 3(a–c) | Forced on-disk states | **PASS.** (a) `db-key` only → migrates. (b) `db-key` + a valid wrapped file → relaunch made **no `wrap` call** (two `unwrap`s), deleted `db-key`. (c) garbage `db-key.wrapped.json.tmp` next to `db-key` → tmp removed, migrated normally. |
| 4 | Keystore key gone (uninstall → reinstall drops the alias; app files pushed back without `db-key`) | **PASS.** Log `wrapped DB key is permanently unusable and no plaintext copy exists: KEY_MISSING: keystore key vidya.dbkey.wrap.v1 is missing`; `app_state` = `db_key_missing`; `sha256` of `vidya.db` and `db-key.wrapped.json` identical before and after 3 relaunches; no key file created. Done twice (seeded school; completed school with 670 students). |
| 4' | Recover from that state | **PASS after the fix below.** "Start recovery" → Recover screen; `restore_summary` of a `.vbak` made on the device by `backup_now` (670 students, chain ok); `restore_install` → new `wrap` + verifying `unwrap`, new `db-key.wrapped.json` (different hash), restored `vidya.db`, old DB kept as `vidya.db.pre-restore-*`, no `db-key`. Relaunch → DB opens with the new wrapped key, `list_students` = 670. See the restart crash below. |
| 7 | Webview cannot call the plugin | **PASS.** `invoke('plugin:vidya-android|unwrap', …)` and `…|wrap` both reject: `vidya-android.unwrap not allowed. Command not found`. |
| 5 | Restore on a fresh phone via the file picker | **not run.** The test above passed an app-private path to `restore_install` directly. The Android picker returns a `content://` URI, and restore also reads a `.meta` sidecar next to the file, so the picker path on Android is unverified. |
| 6 | Release (R8) build | **not run** (needs `build-config/release.json` and a signing key). |

**Bug found and fixed (this run).** The `db_key_missing` screen was a dead end: just the text
"This device's key is missing. Recover to continue." with no control, and `/recover` was only
routed under the Welcome state, so on Android (no address bar) Recover could not be reached.
The screen now has a **Start recovery** button, and `/recover` renders `RecoverScreen` in that state
(`src/App.tsx`, `keyMissingView` in `src/lib/store.ts`, test
`store.test.ts › lets the key-missing state reach the restore flow`). Checked on the emulator
(button shown, tap opens "Recover a school").

**Found, NOT fixed (outside K1).** After a successful `restore_install` the app calls
`app.restart()`. On Android that aborts the process (logcat: `Abort message: 'Error changing dalvik-cache ownership : Permission denied'`
in the spawned child, then `FORTIFY: pthread_mutex_lock called on a destroyed mutex` in EGL teardown under
`tauri::process::restart` → `std::process::exit`). The user sees a crash and has to reopen the app.
All restore data is durable before the restart, and the next launch is correct (4'). Android needs
its own post-restore path, such as reopening the DB in-process or a clean exit with a "reopen Vidya"
message. After restore the state is `needs_rejoin` (restore semantics, not K1).

**Fixed after this run (not re-run on the emulator):** `restore_install` no longer calls `app.restart()` on Android; it returns success and the Recover screen asks the user to close and reopen Vidya (same pattern as `join_school`).

**Emulator ≠ physical device.** The emulator's AndroidKeyStore is the emulated KeyMint. It is
not a TEE/StrongBox on real silicon, and OEM keystore quirks (Samsung/Xiaomi key invalidation
after OS updates, slow `keystore2`) cannot show up here. Run the plan below on at least one real
low-end phone before shipping.

## What is NOT verified here

The Kotlin code and the JNI round trip have now run on an **emulator** (API 34, arm64; see above).
Still not run: a **physical device**, the R8 release build, Android API levels other than 34, and
the restore picker (`content://`) path. Known residual
points:

- `run_mobile_plugin` blocks the calling thread until the Android UI thread answers. It is
  called from `setup` (Rust main thread — the same pattern `register_android_plugin` already
  uses, which blocks identically) and from the sync `restore_install` command. A hang there
  would show as a frozen splash. On the emulator it did not hang: cold start was 0.7–1.8 s
  (`am start -W` TotalTime) with migration included, about 14 s right after a reboot. Check it on a device.
- Plaintext key bytes in Rust are not zeroized (`String` hex is passed to SQLCipher, as before).
- StrongBox is not requested (TEE-backed keystore is used); deleting a file on flash does not
  guarantee physical erasure of the old plaintext.
- The machine id (`licence::machine::FileMachineStore`) is still a plain file (lower sensitivity;
  out of scope for K1).

## Manual device test plan (must pass before shipping an Android build)

Use `adb` from `~/Library/Android/sdk/platform-tools`. Debug package: `in.vidyabudget.app.debug`.
Inspect files with `adb shell run-as in.vidyabudget.app.debug ls -la files/ ../` — check the actual
`app_data_dir` (Tauri: `/data/data/<pkg>/` + identifier dir; locate `vidya.db` with
`run-as <pkg> find . -name vidya.db`). Watch logs with `adb logcat | grep -iE "vidya|Tauri|keystore|RustStdout"`.

1. **Fresh install.** Uninstall, install the new build, open, set up a school (or join).
   Expect: `db-key.wrapped.json` present, **no** `db-key`; `cat` the wrapped file shows only
   `v/iv/ct`. Force-stop, reopen → unlock works, data present. Reboot the device → still opens.
2. **Upgrade from a plaintext build.** Install a build from before K1 (e.g. `v2/p21-repairs`
   at `391e8c4`), set up a school, add a student, note the hex in `db-key`. Install the new
   build **over it** (`adb install -r`), open. Expect log "DB key moved into AndroidKeyStore",
   `db-key` gone, wrapped file present, all data present after force-stop + reopen.
3. **Interrupted upgrade.** Repeat 2 but kill during the first launch:
   `adb shell am start -n in.vidyabudget.app.debug/in.vidyabudget.app.MainActivity; sleep 0.3; adb shell am force-stop in.vidyabudget.app.debug`
   with sleeps 0.1 … 2.0 s. After each kill relaunch normally; every time the school must open
   with its data, and eventually `db-key` must be gone. To force each intermediate on-disk state
   deterministically: (a) copy back only `db-key` (wrapped removed) → migrates; (b) restore
   `db-key` next to a valid wrapped file → plaintext deleted on next launch; (c) put garbage in
   `db-key.wrapped.json.tmp` → removed, no effect.
4. **Keystore key missing (simulates invalidation).** On an emulator with a rootable image
   (`google_apis`, not `google_apis_playstore`): migrate as in 2, then `adb root`, copy the
   app's data dir out (`adb pull /data/data/in.vidyabudget.app.debug /tmp/vidya-a`), uninstall
   (this deletes the keystore key), reinstall the same build, push the files back
   (`adb push … ; adb shell chown -R <app uid> …; restorecon -R …`) **without** `db-key`, open.
   Expect log `KEY_MISSING`, the **Recover** screen, and `sha256sum` of `vidya.db` and
   `db-key.wrapped.json` unchanged before/after several relaunches; no new key file created.
   Then restore a `.vbak` → app restarts into the restored school; wrapped file replaced, no
   `db-key`.
5. **Restore on a fresh phone.** Welcome → Recover → restore a `.vbak`. Expect wrapped file only.
6. **Release build (R8).** Repeat 1 with `npm run tauri android build -- --target aarch64` (release,
   minified) to confirm the plugin class and `@Command` methods survive shrinking.
7. **Webview cannot call it.** In the debug webview console:
   `window.__TAURI_INTERNALS__.invoke('plugin:vidya-android|unwrap', {v:1,iv:'',ct:''})` must
   reject (ACL / "not callable from the webview").
