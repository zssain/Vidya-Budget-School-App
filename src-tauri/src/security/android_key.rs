//! AndroidKeyStore-wrapped database key (docs/00-SYSTEM-CONTEXT.md §9, repair K1).
//!
//! On Android the 32-byte DB key is wrapped (AES-256-GCM) by a non-exportable key held
//! in AndroidKeyStore (`plugins/vidya-android`, Kotlin `KeystorePlugin`). Only the
//! wrapped blob is stored, in `data_dir/db-key.wrapped.json`.
//!
//! Older builds kept the key as plain hex in `data_dir/db-key`. [`boot`] migrates it:
//! wrap → write temp + fsync + rename → unwrap again and compare → open the existing
//! DB with the unwrapped key → only then delete the plaintext file. Any failure keeps
//! the plaintext key in use exactly as before and the migration is retried on the
//! next launch. Nothing here ever creates a key over an existing DB, and nothing here
//! deletes the plaintext key unless the wrapped copy has been proven to open the DB.
//!
//! All logic is platform-neutral and runs against the [`Wrapper`] trait, so it is
//! tested on desktop with a fake wrapper (tests below). The real wrapper
//! ([`PluginWrapper`]) exists only on Android. Design + state machine:
//! docs/implementation/android-keystore.md.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::keys::{ensure_key, from_hex, to_hex, DbKey, KeyError, KeyStore};

/// Legacy plaintext key file (hex). Read for migration; deleted once migrated.
pub const LEGACY_FILE: &str = "db-key";
/// The wrapped key (JSON [`WrappedBlob`]). Holds no plaintext.
pub const WRAPPED_FILE: &str = "db-key.wrapped.json";
/// Temp name used while writing [`WRAPPED_FILE`] (renamed over it when complete).
pub const WRAPPED_TMP: &str = "db-key.wrapped.json.tmp";
/// The only blob version this build understands.
pub const BLOB_VERSION: u32 = 1;

/// `{v, iv, ct}` as produced by the keystore `wrap` (base64 iv / ciphertext+tag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedBlob {
    pub v: u32,
    pub iv: String,
    pub ct: String,
}

/// Why a wrap/unwrap failed. Messages never contain key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrapError {
    /// Permanent: the keystore key is invalidated / missing, or the blob can never be
    /// opened. Without a plaintext copy this means the key is gone → Recover.
    Invalidated(String),
    /// Transient: keystore unavailable right now. Never destroys anything; retried
    /// on the next launch.
    Unavailable(String),
}

/// The keystore operations the store needs (real: [`PluginWrapper`]; tests: a fake).
pub trait Wrapper {
    fn wrap(&self, key: &DbKey) -> Result<WrappedBlob, WrapError>;
    fn unwrap(&self, blob: &WrappedBlob) -> Result<DbKey, WrapError>;
}

/// What the wrapped file currently yields. (No `Debug`: it may hold the key.)
enum Wrapped {
    Absent,
    Key(DbKey),
    /// Permanently unusable (invalidated keystore key, bad JSON, GCM failure).
    Unusable(String),
    /// Could not be read/unwrapped right now (I/O, keystore busy, newer blob version).
    Transient(String),
}

/// Result of one migration attempt (logged; asserted by tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Migration {
    /// No plaintext key file: nothing to migrate.
    NotNeeded,
    /// The wrapped key was verified against the DB and the plaintext file is gone.
    Migrated,
    /// The plaintext key stays in use; retried next launch. Reason has no key material.
    Deferred(String),
    /// Test-only: a simulated process death at this step (no cleanup ran).
    #[cfg(test)]
    Crashed(Step),
}

/// Points in the migration where a test can simulate the process dying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// After the keystore returned the blob, before anything was written.
    Wrapped,
    /// After the temp file was written + fsynced, before the rename.
    TempWritten,
    /// After the rename, before the re-unwrap / DB verification.
    Renamed,
    /// After the DB verification, before the plaintext file is deleted.
    Verified,
}

#[cfg(test)]
#[derive(Default, Clone, Copy)]
struct Faults {
    crash_at: Option<Step>,
    fail_delete: bool,
}

/// [`KeyStore`] whose only durable state is the wrapped blob (plus the legacy
/// plaintext file while a migration is pending).
pub struct KeystoreWrappedStore<W: Wrapper> {
    dir: PathBuf,
    wrapper: W,
    #[cfg(test)]
    faults: Faults,
}

/// A "does this key open the DB?" check, or `None` when no DB file exists yet.
pub type DbCheck<'a> = Option<&'a dyn Fn(&DbKey) -> bool>;

impl<W: Wrapper> KeystoreWrappedStore<W> {
    pub fn new(dir: impl Into<PathBuf>, wrapper: W) -> Self {
        Self {
            dir: dir.into(),
            wrapper,
            #[cfg(test)]
            faults: Faults::default(),
        }
    }

    fn legacy_path(&self) -> PathBuf {
        self.dir.join(LEGACY_FILE)
    }

    fn wrapped_path(&self) -> PathBuf {
        self.dir.join(WRAPPED_FILE)
    }

    fn read_plain(&self) -> Result<Option<DbKey>, String> {
        match fs::read_to_string(self.legacy_path()) {
            // A malformed file reads as "no key" (as FileKeyStore always did) and is
            // never deleted by us.
            Ok(hex) => Ok(from_hex(hex.trim())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("read {LEGACY_FILE}: {e}")),
        }
    }

    fn read_wrapped(&self) -> Wrapped {
        let raw = match fs::read(self.wrapped_path()) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Wrapped::Absent,
            Err(e) => return Wrapped::Transient(format!("read {WRAPPED_FILE}: {e}")),
        };
        let blob: WrappedBlob = match serde_json::from_slice(&raw) {
            Ok(b) => b,
            Err(_) => return Wrapped::Unusable(format!("{WRAPPED_FILE} is not a wrapped-key blob")),
        };
        if blob.v != BLOB_VERSION {
            // Probably written by a newer build: do not call it lost (→ Recover).
            return Wrapped::Transient(format!("{WRAPPED_FILE} has unsupported version {}", blob.v));
        }
        match self.wrapper.unwrap(&blob) {
            Ok(k) => Wrapped::Key(k),
            Err(WrapError::Invalidated(m)) => Wrapped::Unusable(m),
            Err(WrapError::Unavailable(m)) => Wrapped::Transient(m),
        }
    }

    /// Distinct keys currently recoverable from disk, preferred (wrapped) first.
    pub fn candidates(&self) -> Vec<DbKey> {
        let mut out = Vec::with_capacity(2);
        if let Wrapped::Key(k) = self.read_wrapped() {
            out.push(k);
        }
        if let Ok(Some(p)) = self.read_plain() {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// Wrap `key`, write it atomically, read it back and check it round-trips.
    /// On a read-back failure the just-written wrapped file is removed again.
    fn store_wrapped(&self, key: &DbKey) -> Result<(), String> {
        let blob = self.wrapper.wrap(key).map_err(|e| format!("wrap: {e:?}"))?;
        let json = serde_json::to_vec(&blob).map_err(|e| format!("encode blob: {e}"))?;
        write_atomic(&self.dir, WRAPPED_FILE, WRAPPED_TMP, &json).map_err(|e| format!("write {WRAPPED_FILE}: {e}"))?;
        match self.read_wrapped() {
            Wrapped::Key(k) if &k == key => Ok(()),
            _ => {
                self.discard_wrapped();
                Err("wrapped key did not round-trip".into())
            }
        }
    }

    fn discard_wrapped(&self) {
        if let Err(e) = remove_if_exists(&self.dir, WRAPPED_FILE) {
            tracing::warn!("could not remove unverified {WRAPPED_FILE}: {e}");
        }
    }

    /// Delete the plaintext key file (the last migration step).
    fn finish(&self) -> Migration {
        #[cfg(test)]
        if self.faults.fail_delete {
            return Migration::Deferred("delete legacy key file: injected failure".into());
        }
        match remove_if_exists(&self.dir, LEGACY_FILE) {
            Ok(()) => Migration::Migrated,
            Err(e) => Migration::Deferred(format!("delete {LEGACY_FILE}: {e}")),
        }
    }

    /// One migration attempt (see module docs). `db` checks a key against the
    /// existing DB; `None` = no DB file yet (nothing to protect, round-trip only).
    pub fn migrate(&self, db: DbCheck<'_>) -> Migration {
        let opens = |k: &DbKey| db.map_or(true, |f| f(k));

        // A temp file left by a crash before the rename holds only a wrapped blob.
        let _ = fs::remove_file(self.dir.join(WRAPPED_TMP));

        let plain = match self.read_plain() {
            Ok(Some(p)) => p,
            Ok(None) => return Migration::NotNeeded,
            Err(e) => return Migration::Deferred(e),
        };

        match self.read_wrapped() {
            Wrapped::Transient(e) => return Migration::Deferred(format!("keystore unavailable: {e}")),
            Wrapped::Key(k) if k == plain => {
                // An earlier run got past the rename but not the delete: verify, finish.
                if !opens(&k) {
                    return Migration::Deferred("wrapped key does not open the database".into());
                }
                return self.finish();
            }
            Wrapped::Key(k) => {
                // Disagreement (an interrupted key replacement). Never guess.
                if db.is_some() && opens(&k) && !opens(&plain) {
                    // The wrapped key is the one this DB uses; the plaintext is stale.
                    return self.finish();
                }
                if !opens(&plain) {
                    return Migration::Deferred("neither stored key opens the database".into());
                }
                // The plaintext key is the live one: re-wrap it below (overwrites k).
            }
            Wrapped::Absent | Wrapped::Unusable(_) => {}
        }

        // 1. Wrap.
        let blob = match self.wrapper.wrap(&plain) {
            Ok(b) => b,
            Err(e) => return Migration::Deferred(format!("wrap: {e:?}")),
        };
        #[cfg(test)]
        if self.faults.crash_at == Some(Step::Wrapped) {
            return Migration::Crashed(Step::Wrapped);
        }

        // 2. Temp file + fsync, then rename over the wrapped file + fsync the dir.
        let json = match serde_json::to_vec(&blob) {
            Ok(j) => j,
            Err(e) => return Migration::Deferred(format!("encode blob: {e}")),
        };
        let tmp = self.dir.join(WRAPPED_TMP);
        if let Err(e) = write_synced(&tmp, &json) {
            let _ = fs::remove_file(&tmp);
            return Migration::Deferred(format!("write {WRAPPED_TMP}: {e}"));
        }
        #[cfg(test)]
        if self.faults.crash_at == Some(Step::TempWritten) {
            return Migration::Crashed(Step::TempWritten);
        }
        if let Err(e) = fs::rename(&tmp, self.wrapped_path()) {
            let _ = fs::remove_file(&tmp);
            return Migration::Deferred(format!("rename {WRAPPED_TMP}: {e}"));
        }
        sync_dir(&self.dir);
        #[cfg(test)]
        if self.faults.crash_at == Some(Step::Renamed) {
            return Migration::Crashed(Step::Renamed);
        }

        // 3. Unwrap the file just written and compare with the plaintext key.
        let unwrapped = match self.read_wrapped() {
            Wrapped::Key(k) => k,
            _ => {
                self.discard_wrapped();
                return Migration::Deferred("re-reading the wrapped key failed".into());
            }
        };
        if unwrapped != plain {
            self.discard_wrapped();
            return Migration::Deferred("wrapped key did not round-trip".into());
        }

        // 4. The unwrapped key must actually open the existing DB.
        if !opens(&unwrapped) {
            self.discard_wrapped();
            return Migration::Deferred("the key does not open the database".into());
        }
        #[cfg(test)]
        if self.faults.crash_at == Some(Step::Verified) {
            return Migration::Crashed(Step::Verified);
        }

        // 5. Only now delete the plaintext copy.
        self.finish()
    }
}

impl<W: Wrapper> KeyStore for KeystoreWrappedStore<W> {
    /// Wrapped key first; the plaintext file is the fallback while a migration is
    /// pending. A permanently unusable wrapped key with no plaintext copy → `None`
    /// (with a DB present, `ensure_key` turns that into `Missing` → Recover).
    fn get(&self) -> Result<Option<DbKey>, KeyError> {
        let wrapped = self.read_wrapped();
        if let Wrapped::Key(k) = wrapped {
            return Ok(Some(k));
        }
        match self.read_plain() {
            Ok(Some(p)) => return Ok(Some(p)),
            Ok(None) => {}
            Err(e) => return Err(KeyError::Backend(e)),
        }
        match wrapped {
            Wrapped::Absent => Ok(None),
            Wrapped::Unusable(why) => {
                tracing::warn!("wrapped DB key is permanently unusable and no plaintext copy exists: {why}");
                Ok(None)
            }
            Wrapped::Transient(e) => Err(KeyError::Backend(e)),
            Wrapped::Key(_) => unreachable!("handled above"),
        }
    }

    /// Store a NEW key (fresh install, restore). Wrapped when the keystore works —
    /// then any stale plaintext file is removed; otherwise the legacy plaintext file
    /// (today's behaviour, no regression) and the next launch migrates it.
    fn set(&self, key: &DbKey) -> Result<(), KeyError> {
        match self.store_wrapped(key) {
            Ok(()) => {
                if let Err(e) = remove_if_exists(&self.dir, LEGACY_FILE) {
                    tracing::warn!("could not remove stale {LEGACY_FILE}: {e}");
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("keystore wrapping unavailable ({e}); storing the DB key in {LEGACY_FILE} for now");
                write_atomic(&self.dir, LEGACY_FILE, &format!("{LEGACY_FILE}.tmp"), to_hex(key).as_bytes())
                    .map_err(|e| KeyError::Backend(format!("write {LEGACY_FILE}: {e}")))?;
                // A wrapped file left from an older key would be preferred by `get`.
                remove_if_exists(&self.dir, WRAPPED_FILE)
                    .map_err(|e| KeyError::Backend(format!("remove stale {WRAPPED_FILE}: {e}")))?;
                Ok(())
            }
        }
    }
}

/// True if `key` opens the encrypted DB at `db_path` and can read it.
pub fn db_opens(db_path: &Path, key: &DbKey) -> bool {
    match crate::db::open_encrypted(db_path, &to_hex(key)) {
        Ok(conn) => crate::db::is_readable(&conn),
        Err(_) => false,
    }
}

/// Android startup: migrate a legacy plaintext key (best effort), then return the
/// key that opens the DB — or a fresh key when there is no DB yet. A DB with no
/// recoverable key → `KeyError::Missing` (Recover); a transient keystore failure with
/// no plaintext copy → `KeyError::Backend` (error screen, retried next launch).
pub fn boot<W: Wrapper>(store: &KeystoreWrappedStore<W>, db_path: &Path) -> Result<DbKey, KeyError> {
    let db_exists = db_path.exists();
    let opens = |k: &DbKey| db_opens(db_path, k);
    let check: DbCheck<'_> = if db_exists { Some(&opens) } else { None };
    match store.migrate(check) {
        Migration::NotNeeded => {}
        Migration::Migrated => tracing::info!("DB key moved into AndroidKeyStore; plaintext key file removed"),
        Migration::Deferred(why) => tracing::warn!("DB key migration deferred (plaintext key kept): {why}"),
        #[cfg(test)]
        Migration::Crashed(_) => {}
    }
    if db_exists {
        // Prefer whichever stored key actually opens this DB (resolves any leftover
        // wrapped/plaintext disagreement without guessing).
        if let Some(k) = store.candidates().into_iter().find(|k| opens(k)) {
            return Ok(k);
        }
    }
    ensure_key(store, db_exists)
}

// ---------------------------------------------------------------- files ------

fn open_private(path: &Path) -> io::Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = open_private(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Write `name` atomically via `tmp_name` (+ fsync file, rename, fsync dir).
fn write_atomic(dir: &Path, name: &str, tmp_name: &str, bytes: &[u8]) -> io::Result<()> {
    let tmp = dir.join(tmp_name);
    let res = write_synced(&tmp, bytes).and_then(|()| fs::rename(&tmp, dir.join(name)));
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    } else {
        sync_dir(dir);
    }
    res
}

fn remove_if_exists(dir: &Path, name: &str) -> io::Result<()> {
    match fs::remove_file(dir.join(name)) {
        Ok(()) => {
            sync_dir(dir);
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Make a rename/delete durable (POSIX). Best effort: not supported on Windows.
fn sync_dir(dir: &Path) {
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
}

// -------------------------------------------------------- real wrapper -------

/// The real wrapper: the Kotlin `KeystorePlugin` via `tauri-plugin-vidya-android`.
/// Blocks on the plugin call — use from the Rust main thread / a command, never
/// from the Android UI thread.
#[cfg(target_os = "android")]
pub struct PluginWrapper<R: tauri::Runtime>(pub tauri::AppHandle<R>);

#[cfg(target_os = "android")]
impl<R: tauri::Runtime> PluginWrapper<R> {
    fn keystore(&self) -> Result<tauri::State<'_, tauri_plugin_vidya_android::Keystore<R>>, WrapError> {
        use tauri::Manager;
        self.0
            .try_state::<tauri_plugin_vidya_android::Keystore<R>>()
            .ok_or_else(|| WrapError::Unavailable("vidya-android plugin not registered".into()))
    }
}

#[cfg(target_os = "android")]
fn plugin_err(e: tauri_plugin_vidya_android::KeystoreError) -> WrapError {
    match e {
        tauri_plugin_vidya_android::KeystoreError::Permanent { code, message } => {
            WrapError::Invalidated(format!("{code}: {message}"))
        }
        tauri_plugin_vidya_android::KeystoreError::Transient(m) => WrapError::Unavailable(m),
    }
}

#[cfg(target_os = "android")]
impl<R: tauri::Runtime> Wrapper for PluginWrapper<R> {
    fn wrap(&self, key: &DbKey) -> Result<WrappedBlob, WrapError> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(key);
        let b = self.keystore()?.wrap(&b64).map_err(plugin_err)?;
        Ok(WrappedBlob { v: b.v, iv: b.iv, ct: b.ct })
    }

    fn unwrap(&self, blob: &WrappedBlob) -> Result<DbKey, WrapError> {
        use base64::Engine;
        let arg = tauri_plugin_vidya_android::WrappedBlob { v: blob.v, iv: blob.iv.clone(), ct: blob.ct.clone() };
        let b64 = self.keystore()?.unwrap(&arg).map_err(plugin_err)?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(b64.as_bytes())
            .map_err(|_| WrapError::Unavailable("keystore returned malformed key encoding".into()))?;
        DbKey::try_from(raw.as_slice())
            .map_err(|_| WrapError::Unavailable("keystore returned a key of the wrong length".into()))
    }
}

/// The Android DB-key store for this app.
#[cfg(target_os = "android")]
pub fn plugin_store<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    data_dir: &Path,
) -> KeystoreWrappedStore<PluginWrapper<R>> {
    KeystoreWrappedStore::new(data_dir, PluginWrapper(app.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::keys::random_key;
    use std::cell::{Cell, RefCell};

    // ---- fake keystore -------------------------------------------------------

    /// In-memory stand-in for AndroidKeyStore: XORs with a secret pad and tags with
    /// a generation counter; `invalidate()` models KeyPermanentlyInvalidatedException.
    #[derive(Default)]
    struct FakeKs {
        pad: RefCell<Option<[u8; 32]>>,
        generation: Cell<u32>,
        invalidated: Cell<bool>,
        wrap_down: Cell<bool>,
        unwrap_down: Cell<bool>,
        /// Corrupt the round-trip (unwrap returns a different key).
        lie: Cell<bool>,
        wrap_calls: Cell<u32>,
    }

    impl FakeKs {
        fn invalidate(&self) {
            self.invalidated.set(true);
        }
    }

    impl Wrapper for &FakeKs {
        fn wrap(&self, key: &DbKey) -> Result<WrappedBlob, WrapError> {
            self.wrap_calls.set(self.wrap_calls.get() + 1);
            if self.wrap_down.get() {
                return Err(WrapError::Unavailable("keystore busy".into()));
            }
            if self.pad.borrow().is_none() || self.invalidated.get() {
                // Like the Kotlin side: (re)generate the keystore key.
                *self.pad.borrow_mut() = Some(random_key());
                self.generation.set(self.generation.get() + 1);
                self.invalidated.set(false);
            }
            let pad = self.pad.borrow().unwrap();
            let mut ct = [0u8; 32];
            for i in 0..32 {
                ct[i] = key[i] ^ pad[i];
            }
            Ok(WrappedBlob { v: BLOB_VERSION, iv: format!("gen{}", self.generation.get()), ct: to_hex(&ct) })
        }

        fn unwrap(&self, blob: &WrappedBlob) -> Result<DbKey, WrapError> {
            if self.unwrap_down.get() {
                return Err(WrapError::Unavailable("keystore busy".into()));
            }
            let pad = match *self.pad.borrow() {
                Some(p) if !self.invalidated.get() => p,
                _ => return Err(WrapError::Invalidated("KEY_INVALIDATED".into())),
            };
            if blob.iv != format!("gen{}", self.generation.get()) {
                return Err(WrapError::Invalidated("BAD_BLOB".into()));
            }
            let ct = from_hex(&blob.ct).ok_or_else(|| WrapError::Invalidated("BAD_BLOB".into()))?;
            let mut k = [0u8; 32];
            for i in 0..32 {
                k[i] = ct[i] ^ pad[i];
            }
            if self.lie.get() {
                k[0] ^= 0xff;
            }
            Ok(k)
        }
    }

    // ---- fixtures ------------------------------------------------------------

    struct Env {
        dir: PathBuf,
    }

    impl Env {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("vidya-akey-{tag}-{}-{nanos}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }
        fn db(&self) -> PathBuf {
            self.dir.join("vidya.db")
        }
        fn legacy(&self) -> PathBuf {
            self.dir.join(LEGACY_FILE)
        }
        fn wrapped(&self) -> PathBuf {
            self.dir.join(WRAPPED_FILE)
        }
        fn tmp(&self) -> PathBuf {
            self.dir.join(WRAPPED_TMP)
        }
        /// A pre-K1 install: a real SQLCipher DB with a row, key in the plaintext file.
        fn legacy_install(&self) -> DbKey {
            let key = random_key();
            let conn = crate::db::open_encrypted(&self.db(), &to_hex(&key)).unwrap();
            conn.execute_batch("CREATE TABLE school(name TEXT); INSERT INTO school VALUES ('Vidya High');")
                .unwrap();
            drop(conn);
            fs::write(self.legacy(), to_hex(&key)).unwrap();
            key
        }
        fn store<'a>(&self, ks: &'a FakeKs) -> KeystoreWrappedStore<&'a FakeKs> {
            KeystoreWrappedStore::new(&self.dir, ks)
        }
        /// The data is still there and readable with `key`.
        fn assert_data(&self, key: &DbKey) {
            let conn = crate::db::open_encrypted(&self.db(), &to_hex(key)).unwrap();
            let name: String = conn.query_row("SELECT name FROM school", [], |r| r.get(0)).unwrap();
            assert_eq!(name, "Vidya High");
        }
        fn no_plaintext_anywhere(&self, key: &DbKey) {
            let hex = to_hex(key);
            for entry in fs::read_dir(&self.dir).unwrap() {
                let p = entry.unwrap().path();
                if p.extension().is_some_and(|e| e == "db" || e == "db-wal" || e == "db-shm") {
                    continue;
                }
                let s = fs::read(&p).unwrap_or_default();
                assert!(!String::from_utf8_lossy(&s).contains(&hex), "plaintext key found in {}", p.display());
            }
        }
    }

    impl Drop for Env {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    // ---- fresh install -------------------------------------------------------

    #[test]
    fn fresh_install_stores_wrapped_only() {
        let env = Env::new("fresh");
        let ks = FakeKs::default();
        let k = boot(&env.store(&ks), &env.db()).unwrap();
        assert!(env.wrapped().exists());
        assert!(!env.legacy().exists());
        env.no_plaintext_anywhere(&k);
        // Next launch (DB now created by the app) → same key, no new wrap.
        crate::db::open_encrypted(&env.db(), &to_hex(&k)).unwrap().execute_batch("CREATE TABLE t(x);").unwrap();
        let calls = ks.wrap_calls.get();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), k);
        assert_eq!(ks.wrap_calls.get(), calls);
    }

    #[test]
    fn fresh_install_with_keystore_down_falls_back_to_plaintext_then_migrates() {
        let env = Env::new("fresh-down");
        let ks = FakeKs::default();
        ks.wrap_down.set(true);
        let k = boot(&env.store(&ks), &env.db()).unwrap();
        assert!(env.legacy().exists() && !env.wrapped().exists());
        crate::db::open_encrypted(&env.db(), &to_hex(&k)).unwrap().execute_batch("CREATE TABLE t(x);").unwrap();
        ks.wrap_down.set(false);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), k);
        assert!(env.wrapped().exists() && !env.legacy().exists());
    }

    // ---- migration -----------------------------------------------------------

    #[test]
    fn migration_success_removes_plaintext_only_after_verified_reopen() {
        let env = Env::new("migrate");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        let store = env.store(&ks);
        let opened = Cell::new(0);
        let check = |k: &DbKey| {
            // The plaintext file must still exist while the DB is being verified.
            assert!(env.legacy().exists(), "plaintext deleted before verification");
            assert!(env.wrapped().exists(), "verification ran before the wrapped file was in place");
            opened.set(opened.get() + 1);
            db_opens(&env.db(), k)
        };
        assert_eq!(store.migrate(Some(&check)), Migration::Migrated);
        assert_eq!(opened.get(), 1);
        assert!(!env.legacy().exists());
        assert!(!env.tmp().exists());
        env.no_plaintext_anywhere(&key);
        assert_eq!(boot(&store, &env.db()).unwrap(), key);
        env.assert_data(&key);
    }

    #[test]
    fn boot_migrates_a_legacy_install() {
        let env = Env::new("boot-migrate");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert!(!env.legacy().exists() && env.wrapped().exists());
        env.assert_data(&key);
    }

    /// Simulated process death at every step, then a normal next launch: the key is
    /// always recoverable, the data always opens, and the migration completes.
    #[test]
    fn crash_at_every_step_is_recoverable() {
        for step in [Step::Wrapped, Step::TempWritten, Step::Renamed, Step::Verified] {
            let env = Env::new("crash");
            let key = env.legacy_install();
            let ks = FakeKs::default();
            let mut store = env.store(&ks);
            store.faults.crash_at = Some(step);
            let check = |k: &DbKey| db_opens(&env.db(), k);
            assert_eq!(store.migrate(Some(&check)), Migration::Crashed(step));
            // Whatever is on disk now, the plaintext key is still there.
            assert!(env.legacy().exists(), "{step:?}: plaintext gone after crash");
            assert_eq!(step == Step::TempWritten, env.tmp().exists(), "{step:?}");
            // Next launch.
            let next = env.store(&ks);
            assert_eq!(boot(&next, &env.db()).unwrap(), key, "{step:?}");
            env.assert_data(&key);
            assert!(!env.legacy().exists(), "{step:?}: migration did not complete on relaunch");
            assert!(!env.tmp().exists(), "{step:?}");
            env.no_plaintext_anywhere(&key);
        }
    }

    #[test]
    fn verify_failure_keeps_plaintext_and_removes_wrapped() {
        let env = Env::new("verify-fail");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        let store = env.store(&ks);
        // The DB refuses the key (e.g. it cannot be opened right now).
        let refuse = |_: &DbKey| false;
        assert!(matches!(store.migrate(Some(&refuse)), Migration::Deferred(_)));
        assert!(env.legacy().exists());
        assert!(!env.wrapped().exists());
        assert_eq!(fs::read_to_string(env.legacy()).unwrap(), to_hex(&key));
        // Next launch with a working DB → migrates.
        assert_eq!(boot(&store, &env.db()).unwrap(), key);
        assert!(!env.legacy().exists());
    }

    #[test]
    fn plaintext_that_does_not_open_the_db_is_never_deleted() {
        let env = Env::new("wrong-plain");
        env.legacy_install();
        let wrong = random_key();
        fs::write(env.legacy(), to_hex(&wrong)).unwrap();
        let ks = FakeKs::default();
        // Real DB check: the plaintext key cannot open it.
        let got = boot(&env.store(&ks), &env.db()).unwrap();
        assert_eq!(got, wrong, "falls back to exactly what is stored, as before K1");
        assert!(env.legacy().exists(), "a key that failed verification must never be deleted");
        assert!(!env.wrapped().exists());
    }

    #[test]
    fn round_trip_mismatch_keeps_plaintext() {
        let env = Env::new("lie");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        ks.lie.set(true);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert!(env.legacy().exists());
        assert!(!env.wrapped().exists());
        env.assert_data(&key);
    }

    #[test]
    fn delete_failure_is_finished_next_launch() {
        let env = Env::new("delete-fail");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        let mut store = env.store(&ks);
        store.faults.fail_delete = true;
        let check = |k: &DbKey| db_opens(&env.db(), k);
        assert!(matches!(store.migrate(Some(&check)), Migration::Deferred(_)));
        assert!(env.legacy().exists() && env.wrapped().exists());
        // Both copies present: the app still gets the right key.
        assert_eq!(store.get().unwrap(), Some(key));
        // Next launch finishes the delete without wrapping again.
        let calls = ks.wrap_calls.get();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert_eq!(ks.wrap_calls.get(), calls);
        assert!(!env.legacy().exists());
        env.assert_data(&key);
    }

    #[test]
    fn wrap_unavailable_during_migration_keeps_using_plaintext() {
        let env = Env::new("wrap-down");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        ks.wrap_down.set(true);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert!(env.legacy().exists() && !env.wrapped().exists() && !env.tmp().exists());
    }

    // ---- keystore failures after migration -----------------------------------

    #[test]
    fn invalidated_keystore_with_db_is_missing_and_destroys_nothing() {
        let env = Env::new("invalidated");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        let blob_before = fs::read(env.wrapped()).unwrap();
        let db_before = fs::read(env.db()).unwrap();
        ks.invalidate();
        for _ in 0..2 {
            assert!(matches!(boot(&env.store(&ks), &env.db()), Err(KeyError::Missing)));
        }
        assert_eq!(fs::read(env.wrapped()).unwrap(), blob_before, "wrapped blob must be left alone");
        assert_eq!(fs::read(env.db()).unwrap(), db_before, "DB must be left alone");
        assert!(!env.legacy().exists(), "no new key may be written over an existing DB");
    }

    #[test]
    fn invalidated_keystore_but_plaintext_still_present_uses_plaintext_and_rewraps() {
        let env = Env::new("invalidated-plain");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        let mut store = env.store(&ks);
        store.faults.fail_delete = true;
        let check = |k: &DbKey| db_opens(&env.db(), k);
        let _ = store.migrate(Some(&check)); // wrapped + plaintext both on disk
        ks.invalidate();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert!(!env.legacy().exists(), "re-wrapped under the new keystore key and finished");
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
    }

    #[test]
    fn transient_unwrap_error_is_an_error_not_missing_and_destroys_nothing() {
        let env = Env::new("transient");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        let blob_before = fs::read(env.wrapped()).unwrap();
        ks.unwrap_down.set(true);
        assert!(matches!(boot(&env.store(&ks), &env.db()), Err(KeyError::Backend(_))));
        assert_eq!(fs::read(env.wrapped()).unwrap(), blob_before);
        assert!(!env.legacy().exists());
        // Keystore back → same key, data intact.
        ks.unwrap_down.set(false);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        env.assert_data(&key);
    }

    #[test]
    fn transient_unwrap_during_migration_defers() {
        let env = Env::new("transient-mig");
        let key = env.legacy_install();
        let ks = FakeKs::default();
        let mut store = env.store(&ks);
        store.faults.crash_at = Some(Step::Renamed);
        let check = |k: &DbKey| db_opens(&env.db(), k);
        let _ = store.migrate(Some(&check));
        ks.unwrap_down.set(true);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key, "plaintext still serves the key");
        assert!(env.legacy().exists() && env.wrapped().exists());
        ks.unwrap_down.set(false);
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), key);
        assert!(!env.legacy().exists());
    }

    #[test]
    fn corrupt_or_newer_wrapped_file_is_never_overwritten_without_a_plaintext_copy() {
        for (content, expect_missing) in [(&b"{not json"[..], true), (&br#"{"v":2,"iv":"a","ct":"b"}"#[..], false)] {
            let env = Env::new("corrupt");
            env.legacy_install();
            fs::remove_file(env.legacy()).unwrap();
            fs::write(env.wrapped(), content).unwrap();
            let ks = FakeKs::default();
            let r = boot(&env.store(&ks), &env.db());
            if expect_missing {
                assert!(matches!(r, Err(KeyError::Missing)));
            } else {
                assert!(matches!(r, Err(KeyError::Backend(_))));
            }
            assert_eq!(fs::read(env.wrapped()).unwrap(), content);
            assert_eq!(ks.wrap_calls.get(), 0);
        }
    }

    // ---- set() (restore) -----------------------------------------------------

    #[test]
    fn set_replaces_key_and_removes_stale_plaintext() {
        let env = Env::new("set");
        env.legacy_install();
        let ks = FakeKs::default();
        let store = env.store(&ks);
        let new_key = random_key();
        store.set(&new_key).unwrap();
        assert!(!env.legacy().exists());
        assert_eq!(store.get().unwrap(), Some(new_key));
        env.no_plaintext_anywhere(&new_key);
    }

    #[test]
    fn set_with_keystore_down_writes_plaintext_and_drops_stale_wrapped() {
        let env = Env::new("set-down");
        let ks = FakeKs::default();
        let store = env.store(&ks);
        let old = random_key();
        store.set(&old).unwrap();
        assert!(env.wrapped().exists());
        ks.wrap_down.set(true);
        let new_key = random_key();
        store.set(&new_key).unwrap();
        assert!(!env.wrapped().exists());
        assert_eq!(store.get().unwrap(), Some(new_key));
    }

    #[test]
    fn disagreeing_copies_resolve_to_the_key_that_opens_the_db() {
        // A restore swapped in a DB under `new_key`, stored it in plaintext (keystore
        // down) but could not remove the old wrapped key.
        let env = Env::new("disagree");
        let ks = FakeKs::default();
        let store = env.store(&ks);
        let old = random_key();
        store.set(&old).unwrap();
        let new_key = env.legacy_install(); // DB + plaintext under the new key
        assert!(env.wrapped().exists());
        assert_eq!(boot(&env.store(&ks), &env.db()).unwrap(), new_key);
        assert!(!env.legacy().exists(), "plaintext re-wrapped, stale wrapped overwritten");
        assert_eq!(env.store(&ks).get().unwrap(), Some(new_key));
        env.assert_data(&new_key);
    }
}
