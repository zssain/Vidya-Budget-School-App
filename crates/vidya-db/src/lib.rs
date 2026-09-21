//! SQLCipher connection, migrations, repositories (one module per table group)
//! and the change-log writer.
//!
//! Contains no business rules and no permission checks (see `crates/AGENTS.md`).

use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Transaction, TransactionBehavior};
use thiserror::Error;
use zeroize::Zeroizing;

pub mod repo;

const MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("../migrations/0001_init.sql"))];

/// Errors returned by database opening, migration, reads, and writes.
#[derive(Debug, Error)]
pub enum DbError {
    /// The supplied key cannot read the encrypted database.
    #[error("incorrect database key or unreadable encrypted database")]
    WrongKey,
    /// The selected file is a plaintext SQLite database.
    #[error("database is not encrypted")]
    NotEncrypted,
    /// The schema cannot be migrated safely.
    #[error("database migration {version} failed: {message}")]
    Migration { version: u32, message: String },
    /// A SQLite operation failed.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// A connection could not be obtained from the pool.
    #[error("database pool: {0}")]
    Pool(String),
    /// File inspection failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// A four-connection SQLCipher pool with serialized immediate writes.
pub struct Db {
    pool: Pool<SqliteConnectionManager>,
    write_lock: Mutex<()>,
}

impl Db {
    /// Opens or creates an encrypted database and applies known migrations.
    ///
    /// Call this only after the platform layer has retrieved the 32-byte database key.
    /// Returns `NotEncrypted` for a plaintext SQLite file, `WrongKey` if the
    /// supplied key cannot read it, or a migration, pool, SQLite, or I/O error.
    pub fn open(path: &Path, key: &Zeroizing<[u8; 32]>) -> Result<Self, DbError> {
        reject_plaintext_file(path)?;
        let connection = Connection::open(path)?;
        init_connection(&connection, key).map_err(|_| DbError::WrongKey)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        drop(connection);

        let key = Arc::new(Zeroizing::new(**key));
        let manager = SqliteConnectionManager::file(path)
            .with_init(move |connection| init_connection(connection, &key));
        let db = Self::from_manager(manager)?;
        db.apply_migrations()?;
        Ok(db)
    }

    /// Opens an isolated encrypted, shared in-memory database for tests.
    ///
    /// This never touches the device key or filesystem. Errors are propagated.
    pub fn open_in_memory_for_tests() -> Result<Self, DbError> {
        let key = Arc::new(Zeroizing::new([0u8; 32]));
        let manager =
            SqliteConnectionManager::memory().with_init(move |connection| init_connection(connection, &key));
        let db = Self::from_manager(manager)?;
        db.apply_migrations()?;
        Ok(db)
    }

    fn from_manager(manager: SqliteConnectionManager) -> Result<Self, DbError> {
        let pool = Pool::builder()
            .max_size(4)
            .build(manager)
            .map_err(|error| DbError::Pool(error.to_string()))?;
        Ok(Self {
            pool,
            write_lock: Mutex::new(()),
        })
    }

    /// Borrows a configured connection for a read-only operation.
    ///
    /// Callers must not issue writes here; application writes go through
    /// `vidya-services` and `Db::write`. Pool and SQLite errors are propagated.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, DbError> {
        let connection = self
            .pool
            .get()
            .map_err(|error| DbError::Pool(error.to_string()))?;
        f(&connection)
    }

    /// Runs one serialized `BEGIN IMMEDIATE` write transaction.
    ///
    /// Service callers authorize and validate *before* the transaction, then do
    /// repository writes and the change-log entry inside it (all `DbError`). An
    /// error or panic rolls the transaction back.
    pub fn write<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T, DbError>) -> Result<T, DbError> {
        let _lock = self
            .write_lock
            .lock()
            .map_err(|_| DbError::Pool("write lock poisoned".to_owned()))?;
        let mut connection = self
            .pool
            .get()
            .map_err(|error| DbError::Pool(error.to_string()))?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }

    fn apply_migrations(&self) -> Result<(), DbError> {
        let newest = MIGRATIONS.last().map_or(0, |(version, _)| *version);
        let current: u32 = self.read(|connection| {
            connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(DbError::from)
        })?;
        if current > newest {
            return Err(DbError::Migration {
                version: current,
                message: "db.error.newer_version".to_owned(),
            });
        }
        for &(version, sql) in MIGRATIONS {
            if version <= current {
                continue;
            }
            self.write(|tx| {
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", version)?;
                Ok(())
            })
            .map_err(|error| DbError::Migration {
                version,
                message: error.to_string(),
            })?;
        }
        Ok(())
    }
}

fn reject_plaintext_file(path: &Path) -> Result<(), DbError> {
    if !path.exists() {
        return Ok(());
    }
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() < 16 {
        return Ok(());
    }
    let mut header = [0u8; 16];
    file.read_exact(&mut header)?;
    if header == *b"SQLite format 3\0" {
        return Err(DbError::NotEncrypted);
    }
    Ok(())
}

fn init_connection(connection: &Connection, key: &[u8; 32]) -> rusqlite::Result<()> {
    let mut hex = Zeroizing::new(String::with_capacity(64));
    for byte in key {
        use std::fmt::Write;
        write!(hex, "{byte:02x}").map_err(|_| rusqlite::Error::InvalidQuery)?;
    }
    let statement = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", hex.as_str()));
    connection.execute_batch(&statement)?;
    drop(statement);
    drop(hex);
    connection.execute_batch("PRAGMA cipher_memory_security = ON;")?;
    let _: i64 = connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    Ok(())
}

/// Inserts school-independent defaults during setup, never during database open.
///
/// The setup service may call this inside its authorized transaction. It is
/// idempotent and returns a SQLite error if a seed insert fails. `now` is kept
/// in the signature for setup's common timestamp interface; these rows use HLC.
pub fn seed_defaults(tx: &Transaction<'_>, _now: &str, hlc: &str) -> Result<(), DbError> {
    for (grade, min_percent) in [("A", 80), ("B", 65), ("C", 50), ("D", 33), ("E", 0)] {
        tx.execute(
            "INSERT OR IGNORE INTO grade_scale (grade, min_percent, updated_hlc) VALUES (?1, ?2, ?3)",
            rusqlite::params![grade, min_percent, hlc],
        )?;
    }
    for name in ["adm_no", "tc_no", "receipt:PC"] {
        tx.execute(
            "INSERT OR IGNORE INTO counters (name, value) VALUES (?1, 0)",
            [name],
        )?;
    }
    for (key, value) in [
        ("session_timeout_minutes", "30"),
        ("receipt_paper", "a4"),
        ("print_language", "en"),
        ("keep_running_in_tray", "1"),
        ("start_at_login", "1"),
        ("keep_awake", "1"),
        ("backup_staff_can_run", "1"),
        ("backup_destinations", "[]"),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
    }
    Ok(())
}

/// Returns the SQLCipher version and cryptographic provider linked into this build.
pub fn sqlcipher_version() -> rusqlite::Result<(String, String)> {
    let connection = Connection::open_in_memory()?;
    connection.execute_batch(concat!(
        "PRAGMA key = \"x'",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "'\";"
    ))?;
    let version = connection.query_row("PRAGMA cipher_version", [], |row| row.get(0))?;
    let provider = connection.query_row("PRAGMA cipher_provider", [], |row| row.get(0))?;
    Ok((version, provider))
}

#[cfg(test)]
mod tests {
    #[test]
    fn migration_matches_doc() {
        let document = include_str!("../../../docs/DATA_MODEL.md").replace("\r\n", "\n");
        let migration = include_str!("../migrations/0001_init.sql").replace("\r\n", "\n");
        let after_heading = document
            .split_once("## 0001_init.sql\n")
            .expect("document must contain the migration heading")
            .1;
        let after_fence = after_heading
            .split_once("```sql\n")
            .expect("document must contain the SQL fence")
            .1;
        let sql = after_fence
            .split_once("```")
            .expect("document must close the SQL fence")
            .0;
        assert_eq!(migration, sql);
    }

    #[test]
    fn sqlcipher_is_linked() {
        let (version, provider) = super::sqlcipher_version().expect("SQLCipher version should be available");
        assert!(!version.is_empty());
        assert!(!provider.is_empty());
        println!("SQLCipher {version}, provider {provider}");
    }
}
