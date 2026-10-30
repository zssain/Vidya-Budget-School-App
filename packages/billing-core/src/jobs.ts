// Durable job queue on billing.jobs.
//
// * enqueue: INSERT … ON CONFLICT (dedupe_key) DO NOTHING — duplicates are free.
// * claim:   one row via FOR UPDATE SKIP LOCKED, so concurrent workers never take
//            the same job; a `running` row whose lease expired (worker crashed) is
//            reclaimable — that is the lease-recovery path.
// * attempts is incremented at CLAIM time so a job that crashes its worker still
//   counts toward max_attempts and eventually dead-letters instead of looping.
// * fail:    bounded exponential backoff, then visible `dead` state.
// The signer (separate Rust worker) claims `issue_licence` from the same table with
// the same semantics; this worker never claims that kind.
import type { PoolLike, Queryable } from "./db";

export type JobKind =
  | "process_webhook"
  | "reconcile_order"
  | "issue_licence"
  | "deliver_licence"
  | "purge_webhooks";

export const WORKER_KINDS: readonly JobKind[] = [
  "process_webhook",
  "reconcile_order",
  "deliver_licence",
  "purge_webhooks",
];

export interface Job {
  id: string;
  kind: JobKind;
  dedupe_key: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  payload: Record<string, any>;
  state: "ready" | "running" | "done" | "dead";
  attempts: number;
  max_attempts: number;
  locked_by: string | null;
}

export interface EnqueueInput {
  kind: JobKind;
  dedupeKey: string;
  payload?: Record<string, unknown>;
  delaySeconds?: number;
  maxAttempts?: number;
}

export async function enqueueJob(
  q: Queryable,
  input: EnqueueInput,
): Promise<{ id: string | null; created: boolean }> {
  const r = await q.query<{ id: string }>(
    `INSERT INTO billing.jobs (kind, dedupe_key, payload, available_at, max_attempts)
     VALUES ($1, $2, $3::jsonb, now() + make_interval(secs => $4), $5)
     ON CONFLICT (dedupe_key) DO NOTHING
     RETURNING id`,
    [
      input.kind,
      input.dedupeKey,
      JSON.stringify(input.payload ?? {}),
      input.delaySeconds ?? 0,
      input.maxAttempts ?? 8,
    ],
  );
  return r.rows[0] ? { id: r.rows[0].id, created: true } : { id: null, created: false };
}

/** Dead-letter `running` jobs whose lease expired after their last allowed attempt. */
export async function deadLetterExhaustedLeases(
  q: Queryable,
  kinds: readonly JobKind[],
): Promise<number> {
  const r = await q.query(
    `UPDATE billing.jobs
        SET state = 'dead', locked_by = NULL, lease_until = NULL,
            last_error = 'lease expired after final attempt (worker crashed or timed out)'
      WHERE kind = ANY($1::text[]) AND state = 'running'
        AND lease_until < now() AND attempts >= max_attempts`,
    [kinds],
  );
  return r.rowCount ?? 0;
}

export async function claimJob(
  pool: PoolLike,
  opts: { workerId: string; kinds: readonly JobKind[]; leaseSeconds: number },
): Promise<Job | null> {
  const r = await pool.query<Job>(
    `WITH c AS (
       SELECT id FROM billing.jobs
        WHERE kind = ANY($1::text[])
          AND ((state = 'ready' AND available_at <= now())
            OR (state = 'running' AND lease_until < now() AND attempts < max_attempts))
        ORDER BY available_at
        LIMIT 1
        FOR UPDATE SKIP LOCKED)
     UPDATE billing.jobs j
        SET state = 'running', locked_by = $2,
            lease_until = now() + make_interval(secs => $3),
            attempts = j.attempts + 1
       FROM c WHERE j.id = c.id
     RETURNING j.id, j.kind, j.dedupe_key, j.payload, j.state, j.attempts, j.max_attempts, j.locked_by`,
    [opts.kinds, opts.workerId, opts.leaseSeconds],
  );
  return r.rows[0] ?? null;
}

/** Mark done — only if we still hold the lease (a reclaimed job is finished by its new owner). */
export async function completeJob(q: Queryable, job: Job, workerId: string): Promise<boolean> {
  const r = await q.query(
    `UPDATE billing.jobs SET state = 'done', completed_at = now(), locked_by = NULL,
            lease_until = NULL, last_error = NULL
      WHERE id = $1 AND locked_by = $2 AND state = 'running'`,
    [job.id, workerId],
  );
  return (r.rowCount ?? 0) > 0;
}

export const BACKOFF_BASE_SECONDS = 5;
export const BACKOFF_MAX_SECONDS = 3600;

/** Exponential backoff for the n-th attempt (1-based): 5s, 10s, 20s … capped at 1h. */
export function backoffSeconds(attempt: number): number {
  const exp = BACKOFF_BASE_SECONDS * 2 ** Math.max(0, attempt - 1);
  return Math.min(exp, BACKOFF_MAX_SECONDS);
}

/** A failure the job should NOT retry (bad input, permanent provider rejection). */
export class PermanentJobError extends Error {
  override name = "PermanentJobError";
}

/** Bounded, secret-free error text for jobs.last_error. */
export function safeErrorText(err: unknown): string {
  const raw = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
  return raw
    .replace(/(rzp_(test|live)_)[A-Za-z0-9]+/g, "$1***")
    .replace(/re_[A-Za-z0-9_]{8,}/g, "re_***")
    .replace(/postgres(ql)?:\/\/[^\s]+/g, "postgres://***")
    .slice(0, 500);
}

export async function failJob(
  q: Queryable,
  job: Job,
  workerId: string,
  err: unknown,
): Promise<"retry" | "dead"> {
  const permanent = err instanceof PermanentJobError;
  const dead = permanent || job.attempts >= job.max_attempts;
  await q.query(
    `UPDATE billing.jobs
        SET state = $3, locked_by = NULL, lease_until = NULL, last_error = $4,
            available_at = CASE WHEN $3 = 'ready' THEN now() + make_interval(secs => $5) ELSE available_at END
      WHERE id = $1 AND locked_by = $2`,
    [job.id, workerId, dead ? "dead" : "ready", safeErrorText(err), backoffSeconds(job.attempts)],
  );
  return dead ? "dead" : "retry";
}
