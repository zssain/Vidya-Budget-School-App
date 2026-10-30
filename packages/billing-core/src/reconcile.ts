// Reconciliation: ask Razorpay (server credentials) about ONE known internal order
// and apply each payment it reports through the same provisioning path as webhooks.
// Only orders we created are polled — never arbitrary unrelated payments. A
// transient API failure throws → the job retries with backoff; it never marks paid.
import { withTransaction, type PoolLike } from "./db";
import { enqueueJob } from "./jobs";
import { applyPaymentObservation, type ApplyOutcome } from "./provisioning";
import { normalisePaymentEntity } from "./razorpay-events";
import type { Environment } from "./webhooks";

/** The subset of the official `razorpay` SDK we use (wrapped by the worker/portal). */
export interface RazorpayApi {
  /** razorpay.orders.fetchPayments(orderId) → { items: PaymentEntity[] } */
  fetchOrderPayments(providerOrderId: string): Promise<unknown[]>;
  /** razorpay.orders.create(...) → { id } */
  createOrder(input: {
    amount: number;
    currency: string;
    receipt: string;
    notes: Record<string, string>;
  }): Promise<{ id: string }>;
}

export interface ReconcileResult {
  orderId: string;
  checked: number;
  outcomes: ApplyOutcome[];
  skipped?: string;
}

export async function reconcileOrder(
  pool: PoolLike,
  api: RazorpayApi,
  orderId: string,
  environment: Environment,
): Promise<ReconcileResult> {
  const r = await pool.query<{ provider_order_id: string | null; environment: Environment; provider: string }>(
    `SELECT provider_order_id, environment, provider FROM billing.orders WHERE id = $1`,
    [orderId],
  );
  const o = r.rows[0];
  if (!o) return { orderId, checked: 0, outcomes: [], skipped: "unknown order" };
  if (o.provider !== "razorpay" || !o.provider_order_id) {
    return { orderId, checked: 0, outcomes: [], skipped: "no provider order" };
  }
  if (o.environment !== environment) {
    // The reconciler's credentials belong to a different environment; do not mix.
    return { orderId, checked: 0, outcomes: [], skipped: "environment mismatch" };
  }
  const items = await api.fetchOrderPayments(o.provider_order_id); // throws on outage → retry
  const outcomes: ApplyOutcome[] = [];
  for (const item of items) {
    const obs = normalisePaymentEntity(item);
    if (!obs) continue;
    const outcome = await withTransaction(pool, (c) =>
      applyPaymentObservation(c, obs, { environment, source: "reconciliation" }),
    );
    outcomes.push(outcome);
  }
  return { orderId, checked: items.length, outcomes };
}

/**
 * Sweep: enqueue reconcile jobs for our own unpaid orders that are old enough that a
 * webhook should have arrived, and young enough to still be payable. One job per
 * order per hour bucket (dedupe), so overlapping sweeps/workers do not pile up.
 */
export async function scheduleReconciliation(
  pool: PoolLike,
  opts: { environment: Environment; minAgeMinutes?: number; maxAgeHours?: number },
): Promise<number> {
  const r = await pool.query<{ id: string }>(
    `SELECT id FROM billing.orders
      WHERE environment = $1 AND provider = 'razorpay' AND provider_order_id IS NOT NULL
        AND status IN ('created', 'pending')
        AND created_at < now() - make_interval(mins => $2)
        AND created_at > now() - make_interval(hours => $3)
      ORDER BY created_at LIMIT 200`,
    [opts.environment, opts.minAgeMinutes ?? 10, opts.maxAgeHours ?? 72],
  );
  const bucket = new Date().toISOString().slice(0, 13); // YYYY-MM-DDTHH
  let n = 0;
  for (const { id } of r.rows) {
    const j = await enqueueJob(pool, {
      kind: "reconcile_order",
      dedupeKey: `reconcile:${id}:sweep:${bucket}`,
      payload: { order_id: id },
    });
    if (j.created) n++;
  }
  return n;
}
