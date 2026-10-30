// Minimal structural types for node-postgres so this package has ZERO runtime
// dependencies: the portal and the worker each own their `pg` install and pass a
// Pool/PoolClient in. Every multi-row state change goes through `withTransaction`.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Row = Record<string, any>;

export interface QueryResultLike<R extends Row = Row> {
  rows: R[];
  rowCount: number | null;
}

export interface Queryable {
  query<R extends Row = Row>(text: string, values?: unknown[]): Promise<QueryResultLike<R>>;
}

export interface PoolClientLike extends Queryable {
  release(err?: Error | boolean): void;
}

export interface PoolLike extends Queryable {
  connect(): Promise<PoolClientLike>;
}

/**
 * Run `fn` inside BEGIN/COMMIT on a dedicated connection. Any throw → ROLLBACK and
 * rethrow, so a crash before commit leaves no partial state; the durable job that
 * invoked it is retried and converges thanks to the unique constraints.
 */
export async function withTransaction<T>(
  pool: PoolLike,
  fn: (client: PoolClientLike) => Promise<T>,
): Promise<T> {
  const client = await pool.connect();
  let broken = false;
  try {
    await client.query("BEGIN");
    const out = await fn(client);
    await client.query("COMMIT");
    return out;
  } catch (err) {
    try {
      await client.query("ROLLBACK");
    } catch {
      broken = true;
    }
    throw err;
  } finally {
    client.release(broken);
  }
}

export interface AuditEntry {
  actor: string; // owner auth uuid, 'worker', 'checkout', 'customer'
  action: string;
  target?: string | null;
  before?: unknown;
  after?: unknown;
  reason?: string | null;
  requestId?: string | null;
}

/** Append to billing.owner_audit (append-only by trigger). Never put secrets here. */
export async function audit(q: Queryable, e: AuditEntry): Promise<void> {
  await q.query(
    `INSERT INTO billing.owner_audit (actor, action, target, before_json, after_json, reason, request_id)
     VALUES ($1, $2, $3, $4::jsonb, $5::jsonb, $6, $7)`,
    [
      e.actor,
      e.action,
      e.target ?? null,
      e.before === undefined ? null : JSON.stringify(e.before),
      e.after === undefined ? null : JSON.stringify(e.after),
      e.reason ?? null,
      e.requestId ?? null,
    ],
  );
}

/** Audit actions that the dashboard surfaces as "needs owner review". */
export const REVIEW_ACTIONS = [
  "review_refund",
  "review_dispute",
  "review_payment_rejected",
  "review_duplicate_capture",
  "review_paid_cancelled_order",
  "review_held_disputed",
] as const;
