//! Shared test harness: a throw-away Postgres database per test with the real
//! billing migration applied, a superuser "seed" pool (plays the billing worker)
//! and a signer pool that runs as the least-privilege `vidya_signer` role.
//!
//! Postgres: `SIGNER_TEST_ADMIN_URL` (default `postgres://postgres@localhost:55432/postgres`).

#![allow(dead_code)]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::SigningKey;
use licence_signer::key::{fingerprint, Signer};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor, PgPool, Row};
use uuid::Uuid;

/// Every billing migration, in order (the same files the billing worker's runner applies).
const MIGRATIONS: &[&str] = &[
    include_str!("../../../../infra/licensing/migrations/0001_billing.sql"),
    include_str!("../../../../infra/licensing/migrations/0002_portal_support.sql"),
    include_str!("../../../../infra/licensing/migrations/0003_issuance_retry.sql"),
];

/// Serialises migration application inside one test binary (the migration's
/// idempotent `CREATE ROLE` blocks are cluster-global and can race).
static MIGRATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn admin_url() -> String {
    std::env::var("SIGNER_TEST_ADMIN_URL").unwrap_or_else(|_| "postgres://postgres@localhost:55432/postgres".into())
}

pub struct TestDb {
    pub name: String,
    /// Superuser on the test DB (seeding / inspection / simulating time).
    pub seed: PgPool,
    /// Runs every statement as `vidya_signer` (the production grants).
    pub signer: PgPool,
}

impl TestDb {
    pub async fn new() -> Self {
        let name = format!("signer_test_{}", Uuid::new_v4().simple());
        let admin_opts: PgConnectOptions = admin_url().parse().expect("admin url");
        {
            let mut admin = admin_opts.connect().await.expect("connect to admin postgres");
            admin
                .execute(AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .await
                .expect("create test database");
            admin.close().await.ok();
        }
        let db_opts = admin_opts.clone().database(&name);
        {
            let _g = MIGRATE_LOCK.lock().await;
            let mut last_err = None;
            for _ in 0..5 {
                let mut c = db_opts.connect().await.expect("connect to test db");
                let mut applied = Ok(());
                for m in MIGRATIONS {
                    if let Err(e) = sqlx::raw_sql(*m).execute(&mut c).await {
                        applied = Err(e);
                        break;
                    }
                }
                match applied {
                    Ok(_) => {
                        last_err = None;
                        c.close().await.ok();
                        break;
                    }
                    Err(e) => {
                        // A concurrent CREATE ROLE elsewhere on the cluster: reset and retry.
                        last_err = Some(e.to_string());
                        c.execute("DROP SCHEMA IF EXISTS billing CASCADE").await.ok();
                        c.close().await.ok();
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                }
            }
            if let Some(e) = last_err {
                panic!("migration failed: {e}");
            }
        }
        let seed = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(db_opts.clone())
            .await
            .unwrap();
        let signer = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    conn.execute("SET ROLE vidya_signer").await?;
                    conn.execute("SET search_path = pg_catalog").await?;
                    Ok(())
                })
            })
            .connect_with(db_opts)
            .await
            .unwrap();
        TestDb { name, seed, signer }
    }

    /// Another `vidya_signer` pool (a second signer process).
    pub async fn another_signer_pool(&self) -> PgPool {
        self.role_pool("SET ROLE vidya_signer").await
    }

    /// A pool acting as the owner portal (`vidya_portal`, production grants).
    pub async fn portal_pool(&self) -> PgPool {
        self.role_pool("SET ROLE vidya_portal").await
    }

    /// A pool acting as the billing worker (`vidya_worker`: sends the emails).
    pub async fn worker_pool(&self) -> PgPool {
        self.role_pool("SET ROLE vidya_worker").await
    }

    async fn role_pool(&self, set_role: &'static str) -> PgPool {
        let opts: PgConnectOptions = admin_url().parse::<PgConnectOptions>().unwrap().database(&self.name);
        PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    conn.execute(set_role).await?;
                    Ok(())
                })
            })
            .connect_with(opts)
            .await
            .unwrap()
    }

    pub async fn drop_db(self) {
        self.seed.close().await;
        self.signer.close().await;
        let admin_opts: PgConnectOptions = admin_url().parse().unwrap();
        if let Ok(mut admin) = admin_opts.connect().await {
            admin
                .execute(AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                    self.name
                )))
                .await
                .ok();
            admin.close().await.ok();
        }
    }
}

/// A TEST-ONLY signing key from a fixed seed. Never a real key.
pub fn test_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub fn test_signer(seed: u8, key_id: &str) -> Signer {
    let k = test_key(seed);
    let fp = fingerprint(&k.verifying_key().to_bytes());
    Signer::new(k, key_id, &[fp]).unwrap()
}

/// The licence-maker key-file text for a seed.
pub fn key_file_text(seed: u8) -> String {
    STANDARD.encode([seed; 32])
}

pub struct Seeded {
    pub entitlement_id: Uuid,
    pub issuance_id: Uuid,
    pub job_id: Uuid,
    pub licence_id: String,
    pub machine_code: String,
    pub issued_at: String,
}

pub struct SeedSpec<'a> {
    pub school: &'a str,
    pub machine_code: &'a str,
    pub modules: &'a [&'a str],
    pub commercial_status: &'a str,
    pub max_attempts: i32,
}

impl Default for SeedSpec<'_> {
    fn default() -> Self {
        SeedSpec {
            school: "Saraswati Public School",
            machine_code: "3N0S-10YX-JQCR-T",
            modules: &["core"],
            commercial_status: "active",
            max_attempts: 8,
        }
    }
}

/// Insert what the billing worker inserts after a verified capture: an order
/// chain, ONE entitlement, ONE pending issuance (licence_id + issued_at fixed),
/// and ONE `issue_licence` job with dedupe_key `issue:<issuance_id>`.
pub async fn seed_issue(db: &TestDb, spec: &SeedSpec<'_>) -> Seeded {
    let p = &db.seed;
    sqlx::query(
        "INSERT INTO billing.products (id, name, price_paise, modules) \
         VALUES ('vidya-perpetual-v1', 'Vidya', 1499900, ARRAY['core']) ON CONFLICT DO NOTHING",
    )
    .execute(p)
    .await
    .unwrap();
    let cust: Uuid = sqlx::query(
        "INSERT INTO billing.billing_customers (name, email) VALUES ('Buyer', 'b@example.test') RETURNING id",
    )
    .fetch_one(p)
    .await
    .unwrap()
    .get(0);
    let school: Uuid =
        sqlx::query("INSERT INTO billing.billing_schools (customer_id, school_name) VALUES ($1, $2) RETURNING id")
            .bind(cust)
            .bind(spec.school)
            .fetch_one(p)
            .await
            .unwrap()
            .get(0);
    let modules: Vec<String> = spec.modules.iter().map(|s| s.to_string()).collect();
    let order: Uuid = sqlx::query(
        "INSERT INTO billing.orders (school_id, product_id, amount_paise, currency, modules, machine_code, environment, provider, status) \
         VALUES ($1, 'vidya-perpetual-v1', 1499900, 'INR', $2, $3, 'test', 'manual', 'paid') RETURNING id",
    )
    .bind(school)
    .bind(&modules)
    .bind(spec.machine_code)
    .fetch_one(p)
    .await
    .unwrap()
    .get(0);
    let ent: Uuid = sqlx::query(
        "INSERT INTO billing.entitlements (order_id, modules, machine_code, school_name, source, commercial_status) \
         VALUES ($1, $2, $3, $4, 'verified_payment', $5) RETURNING id",
    )
    .bind(order)
    .bind(&modules)
    .bind(spec.machine_code)
    .bind(spec.school)
    .bind(spec.commercial_status)
    .fetch_one(p)
    .await
    .unwrap()
    .get(0);
    let licence_id = format!("VID-{}", Uuid::new_v4());
    let issued_at = "2026-10-31T09:15:00Z".to_string();
    let iss: Uuid = sqlx::query(
        "INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at) \
         VALUES ($1, 1, $2, $3, $4) RETURNING id",
    )
    .bind(ent)
    .bind(&licence_id)
    .bind(spec.machine_code)
    .bind(&issued_at)
    .fetch_one(p)
    .await
    .unwrap()
    .get(0);
    let job = insert_job(db, iss, &format!("issue:{iss}"), spec.max_attempts).await;
    Seeded {
        entitlement_id: ent,
        issuance_id: iss,
        job_id: job,
        licence_id,
        machine_code: spec.machine_code.to_string(),
        issued_at,
    }
}

pub async fn insert_job(db: &TestDb, issuance_id: Uuid, dedupe_key: &str, max_attempts: i32) -> Uuid {
    sqlx::query(
        "INSERT INTO billing.jobs (kind, dedupe_key, payload, max_attempts) \
         VALUES ('issue_licence', $1, jsonb_build_object('issuance_id', $2::text), $3) RETURNING id",
    )
    .bind(dedupe_key)
    .bind(issuance_id.to_string())
    .bind(max_attempts)
    .fetch_one(&db.seed)
    .await
    .unwrap()
    .get(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuanceRow {
    pub status: String,
    pub payload: Option<Vec<u8>>,
    pub signature: Option<Vec<u8>>,
    pub licence_key: Option<String>,
    pub vlic: Option<String>,
    pub signer_key_id: Option<String>,
    pub last_error: Option<String>,
    pub issued_ts_set: bool,
}

pub async fn issuance(db: &TestDb, id: Uuid) -> IssuanceRow {
    let r = sqlx::query(
        "SELECT status, payload, signature, licence_key, vlic, signer_key_id, last_error, issued_ts IS NOT NULL AS ts \
         FROM billing.licence_issuances WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&db.seed)
    .await
    .unwrap();
    IssuanceRow {
        status: r.get("status"),
        payload: r.get("payload"),
        signature: r.get("signature"),
        licence_key: r.get("licence_key"),
        vlic: r.get("vlic"),
        signer_key_id: r.get("signer_key_id"),
        last_error: r.get("last_error"),
        issued_ts_set: r.get("ts"),
    }
}

#[derive(Debug, Clone)]
pub struct JobRow {
    pub state: String,
    pub attempts: i32,
    pub locked_by: Option<String>,
    pub lease_set: bool,
    pub last_error: Option<String>,
    pub completed: bool,
    pub available_in_future: bool,
}

pub async fn job(db: &TestDb, id: Uuid) -> JobRow {
    let r = sqlx::query(
        "SELECT state, attempts, locked_by, lease_until IS NOT NULL AS lease, last_error, \
                completed_at IS NOT NULL AS done, available_at > now() AS future \
         FROM billing.jobs WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&db.seed)
    .await
    .unwrap();
    JobRow {
        state: r.get("state"),
        attempts: r.get("attempts"),
        locked_by: r.get("locked_by"),
        lease_set: r.get("lease"),
        last_error: r.get("last_error"),
        completed: r.get("done"),
        available_in_future: r.get("future"),
    }
}

pub async fn audit_count(db: &TestDb, action: &str, target: Uuid) -> i64 {
    sqlx::query("SELECT count(*) FROM billing.owner_audit WHERE actor = 'signer' AND action = $1 AND target = $2")
        .bind(action)
        .bind(target.to_string())
        .fetch_one(&db.seed)
        .await
        .unwrap()
        .get(0)
}

/// Pretend the lease ran out (instead of sleeping 60 s).
pub async fn expire_lease(db: &TestDb, job_id: Uuid) {
    sqlx::query("UPDATE billing.jobs SET lease_until = now() - interval '1 second' WHERE id = $1")
        .bind(job_id)
        .execute(&db.seed)
        .await
        .unwrap();
}

/// The owner portal's "Retry issuance": the same statements and state transitions as
/// `retryFailedIssuance` in packages/billing-core/src/owner-actions.ts (the TS
/// action itself is tested in services/billing-worker/test; this audit JSON is
/// abbreviated). Runs as `vidya_portal`, so the 0003 grant and trigger are
/// exercised. Returns the new job id, or the refusal code.
pub async fn portal_retry_issuance(
    portal: &PgPool,
    issuance_id: Uuid,
    owner: &str,
    reason: &str,
) -> Result<Uuid, String> {
    let mut tx = portal.begin().await.unwrap();
    let order_id: Uuid = match sqlx::query(
        "SELECT e.order_id FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id \
         WHERE i.id = $1",
    )
    .bind(issuance_id)
    .fetch_optional(&mut *tx)
    .await
    .unwrap()
    {
        Some(r) => r.get(0),
        None => return Err("not_found".into()),
    };
    sqlx::query("SELECT * FROM billing.orders WHERE id = $1 FOR UPDATE")
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let s = sqlx::query(
        "SELECT i.entitlement_id, i.generation, i.status, i.licence_id, i.machine_code, i.last_error, \
                i.signature IS NOT NULL AS signed, e.commercial_status \
         FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id \
         WHERE i.id = $1 FOR UPDATE OF i",
    )
    .bind(issuance_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let status: String = s.get("status");
    let signed: bool = s.get("signed");
    if status != "failed" || signed {
        return Err("not_failed".into());
    }
    let commercial: String = s.get("commercial_status");
    if commercial != "active" {
        return Err("blocked".into());
    }
    let generation: i32 = s.get("generation");
    let latest: i32 = sqlx::query("SELECT max(generation) FROM billing.licence_issuances WHERE entitlement_id = $1")
        .bind(s.get::<Uuid, _>("entitlement_id"))
        .fetch_one(&mut *tx)
        .await
        .unwrap()
        .get(0);
    if latest != generation {
        return Err("not_latest".into());
    }
    let history = sqlx::query(
        "SELECT id, dedupe_key, state, attempts, last_error FROM billing.jobs \
         WHERE kind = 'issue_licence' AND payload->>'issuance_id' = $1 ORDER BY created_at",
    )
    .bind(issuance_id.to_string())
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    if history
        .iter()
        .any(|r| matches!(r.get::<String, _>("state").as_str(), "ready" | "running"))
    {
        return Err("retry_in_progress".into());
    }
    sqlx::query(
        "UPDATE billing.licence_issuances SET status = 'pending', last_error = NULL WHERE id = $1 AND status = 'failed'",
    )
    .bind(issuance_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    let job_id: Uuid = sqlx::query(
        "INSERT INTO billing.jobs (kind, dedupe_key, payload) \
         VALUES ('issue_licence', $1, jsonb_build_object('issuance_id', $2::text)) RETURNING id",
    )
    .bind(format!("issue:{issuance_id}:retry:{}", history.len()))
    .bind(issuance_id.to_string())
    .fetch_one(&mut *tx)
    .await
    .unwrap()
    .get(0);
    let failed_jobs: Vec<String> = history.iter().map(|r| r.get::<Uuid, _>("id").to_string()).collect();
    sqlx::query(
        "INSERT INTO billing.owner_audit (actor, action, target, before_json, after_json, reason) \
         VALUES ($1, 'licence_issue_retry_requested', $2, \
                 jsonb_build_object('status', 'failed', 'last_error', $3::text, 'failed_jobs', $4::text[]), \
                 jsonb_build_object('status', 'pending', 'licence_id', $5::text, 'generation', $6::int, 'job_id', $7::text), $8)",
    )
    .bind(owner)
    .bind(issuance_id.to_string())
    .bind(s.get::<Option<String>, _>("last_error"))
    .bind(&failed_jobs)
    .bind(s.get::<String, _>("licence_id"))
    .bind(generation)
    .bind(job_id.to_string())
    .bind(reason)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    Ok(job_id)
}

/// Any-actor audit count for (action, target).
pub async fn audit_count_any(db: &TestDb, action: &str, target: Uuid) -> i64 {
    sqlx::query("SELECT count(*) FROM billing.owner_audit WHERE action = $1 AND target = $2")
        .bind(action)
        .bind(target.to_string())
        .fetch_one(&db.seed)
        .await
        .unwrap()
        .get(0)
}

/// (orders paid, issuances, issued issuances, downloadable keys) for one entitlement's order.
pub async fn order_summary(db: &TestDb, entitlement_id: Uuid) -> (i64, i64, i64, i64) {
    let r = sqlx::query(
        "SELECT \
           (SELECT count(*) FROM billing.orders o JOIN billing.entitlements e ON e.order_id = o.id \
             WHERE e.id = $1 AND o.status = 'paid') AS paid, \
           (SELECT count(*) FROM billing.licence_issuances WHERE entitlement_id = $1) AS iss, \
           (SELECT count(*) FROM billing.licence_issuances WHERE entitlement_id = $1 AND status = 'issued') AS issued, \
           (SELECT count(*) FROM billing.licence_issuances WHERE entitlement_id = $1 AND vlic IS NOT NULL) AS keys",
    )
    .bind(entitlement_id)
    .fetch_one(&db.seed)
    .await
    .unwrap();
    (r.get("paid"), r.get("iss"), r.get("issued"), r.get("keys"))
}
