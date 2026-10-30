// Owner read models. Every function here is called only AFTER requireOwner().
// No card data, access tokens or signing material is selected: the payments table
// holds none, and licence `payload`/`signature` bytes are never sent to the browser
// (the licence key / .vlic already encode them).
import "server-only";
import { paiseFromDb, REVIEW_ACTIONS, type Queryable } from "@vidya/billing-core";

export const PAGE_SIZE = 50;

export interface ListParams {
  q?: string;
  status?: string;
  page?: number;
}

export function parseList(sp: Record<string, string | string[] | undefined>): ListParams {
  const one = (k: string) => {
    const v = sp[k];
    return typeof v === "string" ? v.slice(0, 100) : undefined;
  };
  const page = Math.max(1, Math.min(10_000, Number.parseInt(one("page") ?? "1", 10) || 1));
  return { q: one("q")?.trim() || undefined, status: one("status") || undefined, page };
}

function like(q: string | undefined) {
  return q ? `%${q.replace(/[\\%_]/g, (m) => `\\${m}`)}%` : null;
}
function offset(p: ListParams) {
  return ((p.page ?? 1) - 1) * PAGE_SIZE;
}

// ------------------------------------------------------------------ dashboard ---
export interface DashboardMetrics {
  days: number;
  capturedPaise: number;
  capturedCount: number;
  refundedPaise: number;
  netPaise: number;
  manualApprovals: number;
  paidAwaitingIssuance: number;
  awaitingMachineCode: number;
  issuedLicences: number;
  issuedLicencesInRange: number;
  deliveryFailures: number;
  pendingTransfers: number;
  deadJobs: number;
  retryingJobs: number;
  openReviews: number;
  workerLastSeen: Date | null;
  signerLastSeen: Date | null;
}

export async function dashboard(q: Queryable, days: number): Promise<DashboardMetrics> {
  const r = await q.query(
    `WITH range AS (SELECT now() - make_interval(days => $1) AS since)
     SELECT
       (SELECT coalesce(sum(amount_paise), 0)::text FROM billing.payments, range WHERE captured_at >= range.since) AS captured,
       (SELECT count(*)::int FROM billing.payments, range WHERE captured_at >= range.since) AS captured_n,
       (SELECT coalesce(sum(refunded_paise), 0)::text FROM billing.payments, range WHERE captured_at >= range.since) AS refunded,
       (SELECT count(*)::int FROM billing.owner_audit, range WHERE action = 'manual_payment_approved' AND at >= range.since) AS manual_n,
       (SELECT count(*)::int FROM billing.entitlements e
          WHERE e.commercial_status = 'active' AND NOT EXISTS (
            SELECT 1 FROM billing.licence_issuances i WHERE i.entitlement_id = e.id AND i.status = 'issued'
              AND i.generation = (SELECT max(generation) FROM billing.licence_issuances WHERE entitlement_id = e.id))) AS awaiting_issue,
       (SELECT count(*)::int FROM billing.orders WHERE status = 'paid_awaiting_machine_code') AS awaiting_code,
       (SELECT count(*)::int FROM billing.licence_issuances WHERE status = 'issued') AS issued,
       (SELECT count(*)::int FROM billing.licence_issuances, range WHERE status = 'issued' AND issued_ts >= range.since) AS issued_range,
       (SELECT count(*)::int FROM billing.deliveries WHERE state = 'failed')
         + (SELECT count(*)::int FROM billing.jobs WHERE kind = 'deliver_licence' AND state = 'dead') AS delivery_failures,
       (SELECT count(*)::int FROM billing.licence_transfers t JOIN billing.licence_issuances i ON i.id = t.replacement_issuance_id
          WHERE i.status = 'pending') AS pending_transfers,
       (SELECT count(*)::int FROM billing.jobs WHERE state = 'dead') AS dead_jobs,
       (SELECT count(*)::int FROM billing.jobs WHERE state = 'ready' AND attempts > 0) AS retrying_jobs,
       (SELECT count(*)::int FROM billing.owner_audit a WHERE a.action = ANY($2::text[])
          AND NOT EXISTS (SELECT 1 FROM billing.owner_audit k WHERE k.action = 'review_acknowledged' AND k.target = a.id::text)) AS open_reviews,
       (SELECT max(last_seen) FROM billing.worker_heartbeats WHERE service = 'billing-worker') AS worker_seen,
       (SELECT max(last_seen) FROM billing.worker_heartbeats WHERE service = 'licence-signer') AS signer_seen`,
    [days, REVIEW_ACTIONS],
  );
  const x = r.rows[0]!;
  const captured = paiseFromDb(x.captured);
  const refunded = paiseFromDb(x.refunded);
  return {
    days,
    capturedPaise: captured,
    capturedCount: x.captured_n,
    refundedPaise: refunded,
    netPaise: captured - refunded,
    manualApprovals: x.manual_n,
    paidAwaitingIssuance: x.awaiting_issue,
    awaitingMachineCode: x.awaiting_code,
    issuedLicences: x.issued,
    issuedLicencesInRange: x.issued_range,
    deliveryFailures: x.delivery_failures,
    pendingTransfers: x.pending_transfers,
    deadJobs: x.dead_jobs,
    retryingJobs: x.retrying_jobs,
    openReviews: x.open_reviews,
    workerLastSeen: x.worker_seen,
    signerLastSeen: x.signer_seen,
  };
}

export async function openReviews(q: Queryable) {
  const r = await q.query(
    `SELECT a.id::text, a.at, a.action, a.target, a.reason FROM billing.owner_audit a
      WHERE a.action = ANY($1::text[])
        AND NOT EXISTS (SELECT 1 FROM billing.owner_audit k WHERE k.action = 'review_acknowledged' AND k.target = a.id::text)
      ORDER BY a.at DESC LIMIT 20`,
    [REVIEW_ACTIONS],
  );
  return r.rows as { id: string; at: Date; action: string; target: string | null; reason: string | null }[];
}

export async function failedJobs(q: Queryable) {
  const r = await q.query(
    `SELECT id, kind, state, attempts, max_attempts, last_error, available_at, created_at FROM billing.jobs
      WHERE state = 'dead' OR (state = 'ready' AND attempts > 0) ORDER BY created_at DESC LIMIT 20`,
  );
  return r.rows;
}

// ------------------------------------------------------------------- orders ------
export const ORDER_COLUMNS = [
  "created_at", "school_name", "customer_name", "email", "phone", "order_id", "provider", "provider_order_id",
  "amount_paise", "currency", "status", "machine_code", "environment",
] as const;

export async function listOrders(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT o.created_at, s.school_name, c.name AS customer_name, c.email, c.phone, o.id AS order_id, o.provider,
            o.provider_order_id, o.amount_paise::text, o.currency, o.status, o.machine_code, o.environment,
            count(*) OVER()::int AS total
       FROM billing.orders o JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE ($1::text IS NULL OR s.school_name ILIKE $1 OR c.email ILIKE $1 OR coalesce(c.phone,'') ILIKE $1
             OR o.id::text ILIKE $1 OR coalesce(o.provider_order_id,'') ILIKE $1 OR coalesce(o.machine_code,'') ILIKE $1)
        AND ($2::text IS NULL OR o.status = $2)
      ORDER BY o.created_at DESC LIMIT $3 OFFSET $4`,
    [like(p.q), p.status ?? null, limit, offset(p)],
  );
  return r.rows;
}

export async function orderDetail(q: Queryable, id: string) {
  const o = await q.query(
    `SELECT o.*, o.amount_paise::text AS amount_paise, s.school_name, c.name AS customer_name, c.email, c.phone, p.name AS product_name
       FROM billing.orders o JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.billing_customers c ON c.id = s.customer_id JOIN billing.products p ON p.id = o.product_id
      WHERE o.id = $1`,
    [id],
  );
  if (!o.rows[0]) return null;
  const pays = await q.query(
    `SELECT provider_payment_id, amount_paise::text, currency, state, refunded_paise::text, captured_at, updated_at
       FROM billing.payments WHERE order_id = $1 ORDER BY created_at`,
    [id],
  );
  const iss = await q.query(
    `SELECT i.id, i.licence_id, i.generation, i.status, i.machine_code, e.commercial_status
       FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
      WHERE e.order_id = $1 ORDER BY i.generation`,
    [id],
  );
  const aud = await q.query(
    `SELECT id::text, at, actor, action, reason, after_json FROM billing.owner_audit WHERE target = $1 ORDER BY at`,
    [id],
  );
  return { order: o.rows[0], payments: pays.rows, issuances: iss.rows, audit: aud.rows };
}

// ----------------------------------------------------------------- payments ------
export const PAYMENT_COLUMNS = [
  "updated_at", "school_name", "email", "order_id", "provider_order_id", "provider_payment_id",
  "amount_paise", "currency", "state", "refunded_paise", "captured_at",
] as const;

export async function listPayments(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT pm.updated_at, s.school_name, c.email, o.id AS order_id, o.provider_order_id, pm.provider_payment_id,
            pm.amount_paise::text, pm.currency, pm.state, pm.refunded_paise::text, pm.captured_at,
            count(*) OVER()::int AS total
       FROM billing.payments pm JOIN billing.orders o ON o.id = pm.order_id
       JOIN billing.billing_schools s ON s.id = o.school_id JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE ($1::text IS NULL OR s.school_name ILIKE $1 OR c.email ILIKE $1 OR pm.provider_payment_id ILIKE $1
             OR coalesce(o.provider_order_id,'') ILIKE $1)
        AND ($2::text IS NULL OR pm.state = $2)
      ORDER BY pm.updated_at DESC LIMIT $3 OFFSET $4`,
    [like(p.q), p.status ?? null, limit, offset(p)],
  );
  return r.rows;
}

// ----------------------------------------------------------------- licences ------
export const LICENCE_COLUMNS = [
  "created_at", "school_name", "email", "licence_id", "generation", "machine_code", "modules",
  "issuance_status", "commercial_status", "superseded", "issued_ts", "delivery_state", "order_id",
] as const;

export async function listLicences(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT i.created_at, e.school_name, c.email, i.id, i.licence_id, i.generation, i.machine_code, e.modules,
            i.status AS issuance_status, e.commercial_status,
            EXISTS (SELECT 1 FROM billing.licence_issuances n WHERE n.entitlement_id = i.entitlement_id AND n.generation > i.generation) AS superseded,
            i.issued_ts,
            (SELECT d.state FROM billing.deliveries d WHERE d.issuance_id = i.id ORDER BY d.updated_at DESC LIMIT 1) AS delivery_state,
            e.order_id, count(*) OVER()::int AS total
       FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
       JOIN billing.orders o ON o.id = e.order_id JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE ($1::text IS NULL OR e.school_name ILIKE $1 OR c.email ILIKE $1 OR i.licence_id ILIKE $1 OR i.machine_code ILIKE $1)
        AND ($2::text IS NULL OR i.status = $2)
      ORDER BY i.created_at DESC LIMIT $3 OFFSET $4`,
    [like(p.q), p.status ?? null, limit, offset(p)],
  );
  return r.rows;
}

export async function licenceDetail(q: Queryable, id: string) {
  const r = await q.query(
    `SELECT i.id, i.licence_id, i.generation, i.machine_code, i.issued_at, i.issued_ts, i.status, i.last_error,
            i.signer_key_id, i.licence_key, i.vlic, i.entitlement_id,
            e.school_name, e.modules, e.commercial_status, e.source, e.order_id, e.created_at AS entitled_at,
            o.provider_order_id, o.amount_paise::text, o.currency, o.status AS order_status, c.email, c.phone,
            EXISTS (SELECT 1 FROM billing.licence_issuances n WHERE n.entitlement_id = i.entitlement_id AND n.generation > i.generation) AS superseded
       FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
       JOIN billing.orders o ON o.id = e.order_id JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE i.id = $1`,
    [id],
  );
  const lic = r.rows[0];
  if (!lic) return null;
  const [generations, transfers, deliveries, audit, payments] = await Promise.all([
    q.query(`SELECT id, generation, licence_id, machine_code, status, issued_ts FROM billing.licence_issuances WHERE entitlement_id = $1 ORDER BY generation`, [lic.entitlement_id]),
    q.query(
      `SELECT t.*, src.generation AS source_generation, rep.generation AS replacement_generation, rep.status AS replacement_status
         FROM billing.licence_transfers t JOIN billing.licence_issuances src ON src.id = t.source_issuance_id
         LEFT JOIN billing.licence_issuances rep ON rep.id = t.replacement_issuance_id
        WHERE src.entitlement_id = $1 ORDER BY t.approved_at`,
      [lic.entitlement_id],
    ),
    q.query(`SELECT idempotency_key, state, attempts, last_error, provider_message_id, updated_at FROM billing.deliveries WHERE issuance_id = $1 ORDER BY created_at`, [id]),
    q.query(
      `SELECT id::text, at, actor, action, target, reason, after_json FROM billing.owner_audit
        WHERE target = ANY($1::text[]) ORDER BY at`,
      [[id, lic.order_id]],
    ),
    q.query(`SELECT provider_payment_id, amount_paise::text, state, captured_at FROM billing.payments WHERE order_id = $1`, [lic.order_id]),
  ]);
  return { lic, generations: generations.rows, transfers: transfers.rows, deliveries: deliveries.rows, audit: audit.rows, payments: payments.rows };
}

// ------------------------------------------------------------ schools, etc. ------
export const SCHOOL_COLUMNS = ["created_at", "school_name", "customer_name", "email", "phone", "orders", "licences"] as const;

export async function listSchools(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT s.created_at, s.school_name, c.name AS customer_name, c.email, c.phone,
            (SELECT count(*)::int FROM billing.orders o WHERE o.school_id = s.id) AS orders,
            (SELECT count(*)::int FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
               JOIN billing.orders o ON o.id = e.order_id WHERE o.school_id = s.id AND i.status = 'issued') AS licences,
            count(*) OVER()::int AS total
       FROM billing.billing_schools s JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE ($1::text IS NULL OR s.school_name ILIKE $1 OR c.email ILIKE $1 OR c.name ILIKE $1 OR coalesce(c.phone,'') ILIKE $1)
      ORDER BY s.created_at DESC LIMIT $2 OFFSET $3`,
    [like(p.q), limit, offset(p)],
  );
  return r.rows;
}

export const TRANSFER_COLUMNS = ["approved_at", "school_name", "source_licence_id", "from_machine", "target_machine_code", "replacement_licence_id", "replacement_status", "reason"] as const;

export async function listTransfers(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT t.approved_at, e.school_name, src.id AS source_id, src.licence_id AS source_licence_id, src.machine_code AS from_machine,
            t.target_machine_code, rep.id AS replacement_id, rep.licence_id AS replacement_licence_id, rep.status AS replacement_status,
            t.reason, count(*) OVER()::int AS total
       FROM billing.licence_transfers t JOIN billing.licence_issuances src ON src.id = t.source_issuance_id
       JOIN billing.entitlements e ON e.id = src.entitlement_id
       LEFT JOIN billing.licence_issuances rep ON rep.id = t.replacement_issuance_id
      WHERE ($1::text IS NULL OR e.school_name ILIKE $1 OR src.licence_id ILIKE $1 OR t.target_machine_code ILIKE $1)
      ORDER BY t.approved_at DESC LIMIT $2 OFFSET $3`,
    [like(p.q), limit, offset(p)],
  );
  return r.rows;
}

export const DELIVERY_COLUMNS = ["updated_at", "school_name", "email", "licence_id", "channel", "state", "attempts", "last_error", "provider_message_id"] as const;

export async function listDeliveries(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT d.updated_at, e.school_name, c.email, i.id AS issuance_id, i.licence_id, d.channel, d.state, d.attempts,
            d.last_error, d.provider_message_id, count(*) OVER()::int AS total
       FROM billing.deliveries d JOIN billing.licence_issuances i ON i.id = d.issuance_id
       JOIN billing.entitlements e ON e.id = i.entitlement_id JOIN billing.orders o ON o.id = e.order_id
       JOIN billing.billing_schools s ON s.id = o.school_id JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE ($1::text IS NULL OR e.school_name ILIKE $1 OR c.email ILIKE $1 OR i.licence_id ILIKE $1)
        AND ($2::text IS NULL OR d.state = $2)
      ORDER BY d.updated_at DESC LIMIT $3 OFFSET $4`,
    [like(p.q), p.status ?? null, limit, offset(p)],
  );
  return r.rows;
}

export const AUDIT_COLUMNS = ["at", "actor", "action", "target", "reason", "request_id", "after_json"] as const;

export async function listAudit(q: Queryable, p: ListParams, limit = PAGE_SIZE) {
  const r = await q.query(
    `SELECT id::text, at, actor, action, target, reason, request_id, after_json, count(*) OVER()::int AS total
       FROM billing.owner_audit
      WHERE ($1::text IS NULL OR action ILIKE $1 OR coalesce(target,'') ILIKE $1 OR actor ILIKE $1 OR coalesce(reason,'') ILIKE $1)
        AND ($2::text IS NULL OR action = $2)
      ORDER BY at DESC LIMIT $3 OFFSET $4`,
    [like(p.q), p.status ?? null, limit, offset(p)],
  );
  return r.rows;
}

export async function products(q: Queryable) {
  const r = await q.query(`SELECT id, name, currency, price_paise::text, modules, active, created_at FROM billing.products ORDER BY created_at`);
  return r.rows;
}
