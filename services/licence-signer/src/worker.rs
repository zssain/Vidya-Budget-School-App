//! The job loop against the `billing` schema (infra/licensing/migrations/0001_billing.sql).
//!
//! One job = three short transactions:
//! 1. **claim** — `SELECT … FOR UPDATE SKIP LOCKED` one ready `issue_licence` job,
//!    set `state='running'`, `attempts+1`, `locked_by`, `lease_until = now()+60s`;
//!    commit. (Attempts are counted at claim time so a job that crashes the
//!    signer still exhausts `max_attempts` instead of looping forever.)
//! 2. **prepare** (read-only) — load issuance + entitlement, revalidate, build the
//!    payload, sign, verify with `vidya_core::licence::verify_v2`.
//! 3. **commit** — lock the job row and require we still own it; lock the
//!    issuance row and require it is still `pending` and the entitlement still
//!    `active`; write the artefact, mark the job done, append `owner_audit`.
//!
//! Because `licence_id` and `issued_at` are persisted by the billing worker before
//! the job exists, and ed25519 is deterministic, every retry re-signs the same
//! bytes. The `status='pending'` guard under a row lock means at most ONE artefact
//! is ever written per issuance, however many signers or duplicate jobs race.
//!
//! Only columns `vidya_signer` is granted are written.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::artefact::{sign_licence, Artefact, LicenceInputs};
use crate::key::Signer;

/// Lease length for a claimed job.
pub const LEASE_SECS: f64 = 60.0;
/// First retry delay; doubles per attempt.
pub const BACKOFF_BASE_SECS: u64 = 30;
/// Retry delay cap.
pub const BACKOFF_CAP_SECS: u64 = 3600;
/// `last_error` is truncated to this many chars.
const MAX_ERROR_CHARS: usize = 500;

/// Backoff before the next attempt, given the attempt number that just failed (1-based).
pub fn backoff_secs(attempt: i32) -> u64 {
    let exp = attempt.saturating_sub(1).clamp(0, 20) as u32;
    BACKOFF_BASE_SECS.saturating_mul(1u64 << exp).min(BACKOFF_CAP_SECS)
}

/// A failure while processing a job. Messages are operational only — they never
/// contain key material and are what lands in `last_error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// Bad data (refunded entitlement, bad machine code, unknown module, …):
    /// retrying cannot help → the job goes `dead` immediately.
    Permanent(String),
    /// Infrastructure trouble (DB): retried with exponential backoff.
    Transient(String),
}

impl JobError {
    pub fn message(&self) -> &str {
        match self {
            JobError::Permanent(m) | JobError::Transient(m) => m,
        }
    }
}

impl From<sqlx::Error> for JobError {
    fn from(e: sqlx::Error) -> Self {
        JobError::Transient(format!("database error: {}", db_error_summary(&e)))
    }
}

/// A short, secret-free description of a sqlx error.
fn db_error_summary(e: &sqlx::Error) -> String {
    match e {
        sqlx::Error::Database(d) => format!("{} ({})", d.message(), d.code().unwrap_or_default()),
        sqlx::Error::PoolTimedOut => "pool timed out".into(),
        sqlx::Error::Io(_) => "connection i/o error".into(),
        sqlx::Error::Tls(_) => "tls error".into(),
        other => {
            let s = other.to_string();
            s.chars().take(200).collect()
        }
    }
}

fn truncate(s: &str) -> String {
    s.chars().take(MAX_ERROR_CHARS).collect()
}

/// A job this signer has claimed (holds the lease).
#[derive(Debug, Clone)]
pub struct ClaimedJob {
    pub job_id: Uuid,
    pub attempts: i32,
    pub max_attempts: i32,
    /// From `payload->>'issuance_id'`; `None` if missing or not a uuid.
    pub issuance_id: Option<Uuid>,
    pub worker_id: String,
}

/// What processing one job produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No ready job.
    Idle,
    /// Signed and persisted a new artefact.
    Issued { issuance_id: Uuid, licence_id: String },
    /// The issuance was already issued with this exact artefact; job closed.
    AlreadyIssued { issuance_id: Uuid },
    /// Failed; `dead` = no further retries.
    Failed { dead: bool, error: String },
    /// We lost the lease (it expired and was recovered); nothing written.
    LeaseLost,
}

/// Claim one ready `issue_licence` job (transaction 1).
pub async fn claim_job(pool: &PgPool, worker_id: &str) -> Result<Option<ClaimedJob>, JobError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT id FROM billing.jobs \
         WHERE kind = 'issue_licence' AND state = 'ready' AND available_at <= now() \
         ORDER BY available_at, created_at \
         LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let job_id: Uuid = row.try_get("id")?;
    let row = sqlx::query(
        "UPDATE billing.jobs \
         SET state = 'running', attempts = attempts + 1, locked_by = $2, \
             lease_until = now() + make_interval(secs => $3) \
         WHERE id = $1 \
         RETURNING attempts, max_attempts, payload->>'issuance_id' AS issuance_id",
    )
    .bind(job_id)
    .bind(worker_id)
    .bind(LEASE_SECS)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let issuance: Option<String> = row.try_get("issuance_id")?;
    Ok(Some(ClaimedJob {
        job_id,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        issuance_id: issuance.and_then(|s| Uuid::parse_str(&s).ok()),
        worker_id: worker_id.to_string(),
    }))
}

/// Result of [`prepare`].
#[derive(Debug, Clone)]
pub enum Prepared {
    /// A fresh artefact to commit.
    Sign(Box<Artefact>),
    /// Issuance is already `issued` — nothing to sign; close the job.
    AlreadyIssued,
}

/// Load + revalidate + sign + verify (transaction-free, read-only).
pub async fn prepare(pool: &PgPool, job: &ClaimedJob, signer: &Signer) -> Result<Prepared, JobError> {
    let issuance_id = job
        .issuance_id
        .ok_or_else(|| JobError::Permanent("job payload has no valid issuance_id".into()))?;
    let row = sqlx::query(
        "SELECT i.licence_id, i.machine_code, i.issued_at, i.status, \
                e.school_name, e.modules, e.commercial_status \
         FROM billing.licence_issuances i \
         JOIN billing.entitlements e ON e.id = i.entitlement_id \
         WHERE i.id = $1",
    )
    .bind(issuance_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| JobError::Permanent("issuance not found".into()))?;

    let status: String = row.try_get("status")?;
    match status.as_str() {
        "pending" => {}
        "issued" => return Ok(Prepared::AlreadyIssued),
        other => {
            return Err(JobError::Permanent(format!(
                "issuance status is '{other}', not pending"
            )))
        }
    }
    let commercial: String = row.try_get("commercial_status")?;
    if commercial != "active" {
        return Err(JobError::Permanent(format!(
            "entitlement is '{commercial}', not active"
        )));
    }
    let inputs = LicenceInputs {
        licence_id: row.try_get("licence_id")?,
        school_name: row.try_get("school_name")?,
        machine_code: row.try_get("machine_code")?,
        issued_at: row.try_get("issued_at")?,
        modules: row.try_get("modules")?,
    };
    let artefact = sign_licence(&inputs, signer).map_err(JobError::Permanent)?;
    Ok(Prepared::Sign(Box::new(artefact)))
}

/// Lock the job row and confirm this worker still owns it.
async fn own_job(tx: &mut sqlx::PgConnection, job: &ClaimedJob) -> Result<bool, JobError> {
    let row = sqlx::query("SELECT state, locked_by FROM billing.jobs WHERE id = $1 FOR UPDATE")
        .bind(job.job_id)
        .fetch_optional(&mut *tx)
        .await?;
    Ok(match row {
        Some(r) => {
            let state: String = r.try_get("state")?;
            let locked_by: Option<String> = r.try_get("locked_by")?;
            state == "running" && locked_by.as_deref() == Some(job.worker_id.as_str())
        }
        None => false,
    })
}

async fn mark_done(tx: &mut sqlx::PgConnection, job: &ClaimedJob) -> Result<(), JobError> {
    sqlx::query(
        "UPDATE billing.jobs SET state = 'done', completed_at = now(), lease_until = NULL, last_error = NULL \
         WHERE id = $1",
    )
    .bind(job.job_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Persist the artefact (transaction 3). See module docs for the guards.
pub async fn commit(
    pool: &PgPool,
    job: &ClaimedJob,
    signer: &Signer,
    prepared: &Prepared,
) -> Result<Outcome, JobError> {
    let issuance_id = job
        .issuance_id
        .ok_or_else(|| JobError::Permanent("job payload has no valid issuance_id".into()))?;
    let mut tx = pool.begin().await?;
    if !own_job(&mut tx, job).await? {
        tx.rollback().await?;
        return Ok(Outcome::LeaseLost);
    }

    let row = sqlx::query(
        "SELECT i.status, i.payload, i.signature, e.commercial_status \
         FROM billing.licence_issuances i \
         JOIN billing.entitlements e ON e.id = i.entitlement_id \
         WHERE i.id = $1 FOR UPDATE OF i",
    )
    .bind(issuance_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| JobError::Permanent("issuance not found".into()))?;
    let status: String = row.try_get("status")?;
    let stored_payload: Option<Vec<u8>> = row.try_get("payload")?;
    let stored_sig: Option<Vec<u8>> = row.try_get("signature")?;
    let commercial: String = row.try_get("commercial_status")?;

    if status == "issued" {
        // Someone (a duplicate job / an earlier attempt) already wrote it. With a
        // fresh artefact in hand it must be byte-identical; otherwise refuse.
        if let Prepared::Sign(a) = prepared {
            if stored_payload.as_deref() != Some(a.payload.as_slice())
                || stored_sig.as_deref() != Some(a.signature.as_slice())
            {
                tx.rollback().await?;
                return Err(JobError::Permanent(
                    "issuance already issued with a different artefact".into(),
                ));
            }
        }
        mark_done(&mut tx, job).await?;
        tx.commit().await?;
        return Ok(Outcome::AlreadyIssued { issuance_id });
    }
    if status != "pending" {
        tx.rollback().await?;
        return Err(JobError::Permanent(format!(
            "issuance status is '{status}', not pending"
        )));
    }
    if commercial != "active" {
        tx.rollback().await?;
        return Err(JobError::Permanent(format!(
            "entitlement is '{commercial}', not active"
        )));
    }
    let Prepared::Sign(a) = prepared else {
        tx.rollback().await?;
        return Err(JobError::Transient(
            "issuance became pending again; will re-sign".into(),
        ));
    };

    sqlx::query(
        "UPDATE billing.licence_issuances \
         SET payload = $2, signature = $3, licence_key = $4, vlic = $5, status = 'issued', \
             issued_ts = now(), signer_key_id = $6, last_error = NULL \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(issuance_id)
    .bind(&a.payload)
    .bind(&a.signature)
    .bind(&a.licence_key)
    .bind(&a.vlic)
    .bind(signer.key_id())
    .execute(&mut *tx)
    .await?;
    mark_done(&mut tx, job).await?;
    sqlx::query(
        "INSERT INTO billing.owner_audit (actor, action, target, after_json) \
         VALUES ('signer', 'licence_issued', $1, jsonb_build_object( \
           'licence_id', $2::text, 'machine_code', $3::text, 'modules', $4::text[], \
           'signer_key_id', $5::text, 'public_key_fingerprint', $6::text, 'job_id', $7::text))",
    )
    .bind(issuance_id.to_string())
    .bind(&a.licence.licence_id)
    .bind(&a.licence.machine_code)
    .bind(&a.licence.modules)
    .bind(signer.key_id())
    .bind(signer.fingerprint())
    .bind(job.job_id.to_string())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Outcome::Issued {
        issuance_id,
        licence_id: a.licence.licence_id.clone(),
    })
}

/// Record a failure for a job we hold: backoff or `dead`; when dead, mark a
/// still-pending issuance `failed` and append an audit row. Returns
/// `Some(dead)`, or `None` if we no longer own the job.
pub async fn record_failure(pool: &PgPool, job: &ClaimedJob, error: &JobError) -> Result<Option<bool>, JobError> {
    let msg = truncate(error.message());
    let dead = matches!(error, JobError::Permanent(_)) || job.attempts >= job.max_attempts;
    let delay = backoff_secs(job.attempts) as f64;
    let mut tx = pool.begin().await?;
    if !own_job(&mut tx, job).await? {
        tx.rollback().await?;
        return Ok(None);
    }
    sqlx::query(
        "UPDATE billing.jobs \
         SET state = CASE WHEN $2 THEN 'dead' ELSE 'ready' END, \
             available_at = CASE WHEN $2 THEN available_at ELSE now() + make_interval(secs => $3) END, \
             locked_by = NULL, lease_until = NULL, last_error = $4 \
         WHERE id = $1",
    )
    .bind(job.job_id)
    .bind(dead)
    .bind(delay)
    .bind(&msg)
    .execute(&mut *tx)
    .await?;
    if dead {
        if let Some(iid) = job.issuance_id {
            fail_issuance(&mut tx, iid, job.job_id, &msg).await?;
        }
    }
    tx.commit().await?;
    Ok(Some(dead))
}

async fn fail_issuance(
    tx: &mut sqlx::PgConnection,
    issuance_id: Uuid,
    job_id: Uuid,
    msg: &str,
) -> Result<(), JobError> {
    let res = sqlx::query(
        "UPDATE billing.licence_issuances SET status = 'failed', last_error = $2 \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(issuance_id)
    .bind(msg)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() > 0 {
        sqlx::query(
            "INSERT INTO billing.owner_audit (actor, action, target, after_json) \
             VALUES ('signer', 'licence_issue_failed', $1, jsonb_build_object('job_id', $2::text, 'error', $3::text))",
        )
        .bind(issuance_id.to_string())
        .bind(job_id.to_string())
        .bind(msg)
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// Return abandoned leases (`running` with `lease_until < now()`) to `ready`, or
/// `dead` (+ issuance `failed`) if they have used all attempts. Returns the
/// number of jobs recovered.
pub async fn recover_expired_leases(pool: &PgPool) -> Result<u64, JobError> {
    let mut tx = pool.begin().await?;
    let rows = sqlx::query(
        "UPDATE billing.jobs \
         SET state = CASE WHEN attempts >= max_attempts THEN 'dead' ELSE 'ready' END, \
             locked_by = NULL, lease_until = NULL, available_at = now(), \
             last_error = 'lease expired: signer crashed or stalled' \
         WHERE kind = 'issue_licence' AND state = 'running' AND lease_until < now() \
         RETURNING id, state, payload->>'issuance_id' AS issuance_id",
    )
    .fetch_all(&mut *tx)
    .await?;
    for r in &rows {
        let state: String = r.try_get("state")?;
        if state == "dead" {
            let job_id: Uuid = r.try_get("id")?;
            let iid: Option<String> = r.try_get("issuance_id")?;
            if let Some(iid) = iid.and_then(|s| Uuid::parse_str(&s).ok()) {
                fail_issuance(&mut tx, iid, job_id, "lease expired after max attempts").await?;
            }
        }
    }
    tx.commit().await?;
    Ok(rows.len() as u64)
}

/// Claim → prepare → commit one job; record any failure.
pub async fn process_one(pool: &PgPool, signer: &Signer, worker_id: &str) -> Result<Outcome, JobError> {
    let Some(job) = claim_job(pool, worker_id).await? else {
        return Ok(Outcome::Idle);
    };
    let result = match prepare(pool, &job, signer).await {
        Ok(p) => commit(pool, &job, signer, &p).await,
        Err(e) => Err(e),
    };
    match result {
        Ok(outcome) => {
            match &outcome {
                Outcome::Issued {
                    issuance_id,
                    licence_id,
                } => {
                    tracing::info!(job_id = %job.job_id, %issuance_id, %licence_id, "licence issued")
                }
                Outcome::AlreadyIssued { issuance_id } => {
                    tracing::info!(job_id = %job.job_id, %issuance_id, "issuance already issued; job closed")
                }
                Outcome::LeaseLost => tracing::warn!(job_id = %job.job_id, "lease lost before commit; nothing written"),
                _ => {}
            }
            Ok(outcome)
        }
        Err(e) => {
            tracing::warn!(job_id = %job.job_id, attempt = job.attempts, error = e.message(), "issue_licence job failed");
            match record_failure(pool, &job, &e).await? {
                Some(dead) => Ok(Outcome::Failed {
                    dead,
                    error: truncate(e.message()),
                }),
                None => Ok(Outcome::LeaseLost),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_and_caps() {
        assert_eq!(backoff_secs(1), 30);
        assert_eq!(backoff_secs(2), 60);
        assert_eq!(backoff_secs(3), 120);
        assert_eq!(backoff_secs(8), 3600);
        assert_eq!(backoff_secs(1000), 3600);
        assert_eq!(backoff_secs(0), 30);
    }
}
