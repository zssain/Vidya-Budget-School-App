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

## What is NOT verified here

The Kotlin code and the JNI round trip were **not run on a device or emulator** (no system
image / device available when this was built). See the device plan below. Known residual
points:

- `run_mobile_plugin` blocks the calling thread until the Android UI thread answers. It is
  called from `setup` (Rust main thread — the same pattern `register_android_plugin` already
  uses, which blocks identically) and from the sync `restore_install` command. A hang there
  would show as a frozen splash; check it on device.
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
4. **Keystore invalidated / missing.** On a migrated install, delete the keystore key (debug
   only: a temporary debug button or `adb shell run-as … ` cannot do this — easiest is to clear
   the keystore by changing device lock type isn't sufficient for a non-auth key; instead copy
   the app's files to a second device/emulator with the same debug build). Expect the **Recover**
   screen, `vidya.db` and the wrapped file unchanged (compare `sha256sum` before/after), no new
   key created. Restore from a `.vbak` → app restarts into the restored school; wrapped file
   replaced, no `db-key`.
5. **Restore on a fresh phone.** Welcome → Recover → restore a `.vbak`. Expect wrapped file only.
6. **Release build (R8).** Repeat 1 with `npm run tauri android build -- --target aarch64` (release,
   minified) to confirm the plugin class and `@Command` methods survive shrinking.
7. **Webview cannot call it.** In the debug webview console:
   `window.__TAURI_INTERNALS__.invoke('plugin:vidya-android|unwrap', {v:1,iv:'',ct:''})` must
   reject (ACL / "not callable from the webview").
