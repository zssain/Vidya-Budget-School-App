// Verified-payment provisioning. Every function here runs INSIDE a caller's
// transaction and serialises on the order row (SELECT … FOR UPDATE), so duplicate
// webhooks, out-of-order events, reconciliation and concurrent workers all converge:
//
//   * payments.provider_payment_id UNIQUE + monotonic state rank → one payment row
//   * entitlements.order_id UNIQUE                            → one entitlement
//   * licence_issuances (entitlement_id, generation) UNIQUE   → one gen-1 issuance
//   * jobs.dedupe_key 'issue:<issuance_id>' UNIQUE           → one signer job
//
// Only a CAPTURED payment whose provider order, amount, currency and environment
// match the stored internal order creates an entitlement. Authorized/failed never do.
import { randomUUID } from "node:crypto";
import { audit, type Queryable } from "./db";
import { enqueueJob } from "./jobs";
import type { PaymentObservation, PaymentState } from "./razorpay-events";
import type { Environment } from "./webhooks";

export type OrderStatus =
  | "created"
  | "pending"
  | "paid"
  | "paid_awaiting_machine_code"
  | "refunded"
  | "partially_refunded"
  | "disputed"
  | "cancelled";

export interface OrderRow {
  id: string;
  school_id: string;
  product_id: string;
  amount_paise: string; // bigint as text
  currency: string;
  modules: string[];
  machine_code: string | null;
  environment: Environment;
  provider: "razorpay" | "manual";
  provider_order_id: string | null;
  status: OrderStatus;
}

export async function lockOrder(q: Queryable, where: { id?: string; providerOrderId?: string }) {
  const r = where.id
    ? await q.query<OrderRow>(`SELECT * FROM billing.orders WHERE id = $1 FOR UPDATE`, [where.id])
    : await q.query<OrderRow>(`SELECT * FROM billing.orders WHERE provider_order_id = $1 FOR UPDATE`, [
        where.providerOrderId,
      ]);
  return r.rows[0] ?? null;
}

/** RFC 3339 UTC, second precision — the exact string the signer puts in the payload. */
export function rfc3339UtcNow(now = new Date()): string {
  return now.toISOString().replace(/\.\d{3}Z$/, "Z");
}

const STATE_RANK_SQL = (x: string) =>
  `array_position(ARRAY['failed','authorized','captured','partially_refunded','refunded','disputed']::text[], ${x})`;

/** Pure: the order status after observing a payment state (never moves "backwards"). */
export function nextOrderStatus(
  current: OrderStatus,
  payment: PaymentState,
  captureHappened: boolean,
  hasMachineCode: boolean,
): OrderStatus {
  if (!captureHappened) return current; // authorized / failed: no fulfilment change
  switch (payment) {
    case "captured":
      if (current === "created" || current === "pending" || current === "cancelled") {
        return hasMachineCode ? "paid" : "paid_awaiting_machine_code";
      }
      return current;
    case "partially_refunded":
      return current === "refunded" || current === "disputed" ? current : "partially_refunded";
    case "refunded":
      return current === "disputed" ? current : "refunded";
    case "disputed":
      return "disputed";
    default:
      return current;
  }
}

export interface IssuanceRef {
  entitlementId: string;
  issuanceId: string;
  licenceId: string;
}

/**
 * Create the order's single entitlement + generation-1 issuance + issue job, iff it
 * does not exist yet. Caller must hold the order row lock and have verified payment
 * (or recorded an audited owner manual approval).
 */
export async function ensureEntitlement(
  q: Queryable,
  order: OrderRow,
  source: "verified_payment" | "owner_manual",
  actor: string,
  requestId?: string | null,
): Promise<IssuanceRef | null> {
  if (!order.machine_code) return null;
  const school = await q.query<{ school_name: string }>(
    `SELECT school_name FROM billing.billing_schools WHERE id = $1`,
    [order.school_id],
  );
  const schoolName = school.rows[0]?.school_name;
  if (!schoolName) throw new Error("order has no school");
  const ent = await q.query<{ id: string }>(
    `INSERT INTO billing.entitlements (order_id, modules, machine_code, school_name, source)
     VALUES ($1, $2::text[], $3, $4, $5)
     ON CONFLICT (order_id) DO NOTHING
     RETURNING id`,
    [order.id, order.modules, order.machine_code, schoolName, source],
  );
  const entitlementId = ent.rows[0]?.id;
  if (!entitlementId) return null; // already provisioned — converge
  const licenceId = `VID-${randomUUID()}`;
  const iss = await q.query<{ id: string }>(
    `INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at, status)
     VALUES ($1, 1, $2, $3, $4, 'pending')
     RETURNING id`,
    [entitlementId, licenceId, order.machine_code, rfc3339UtcNow()],
  );
  const issuanceId = iss.rows[0]!.id;
  await enqueueJob(q, {
    kind: "issue_licence",
    dedupeKey: `issue:${issuanceId}`,
    payload: { issuance_id: issuanceId },
  });
  await audit(q, {
    actor,
    action: "entitlement_created",
    target: order.id,
    after: { entitlement_id: entitlementId, issuance_id: issuanceId, licence_id: licenceId, source },
    requestId,
  });
  return { entitlementId, issuanceId, licenceId };
}

export type ApplyOutcome =
  | { kind: "rejected"; reason: string }
  | {
      kind: "recorded";
      paymentState: PaymentState;
      orderStatus: OrderStatus;
      issuance: IssuanceRef | null;
    };

async function flagOnce(q: Queryable, action: string, target: string, after: unknown, reason: string) {
  const seen = await q.query(
    `SELECT 1 FROM billing.owner_audit WHERE action = $1 AND target = $2 AND reason = $3 LIMIT 1`,
    [action, target, reason],
  );
  if (seen.rows.length === 0) await audit(q, { actor: "worker", action, target, after, reason });
}

/**
 * Apply one provider observation of a payment. Runs inside the caller's transaction.
 * `environment` is the environment whose secret/API credentials produced the
 * observation (the webhook endpoint's configured mode, or the reconciler's).
 */
export async function applyPaymentObservation(
  q: Queryable,
  obs: PaymentObservation,
  ctx: { environment: Environment; source: "webhook" | "reconciliation"; accountOk?: boolean },
): Promise<ApplyOutcome> {
  if (!obs.providerOrderId) return { kind: "rejected", reason: "payment_without_order" };
  const order = await lockOrder(q, { providerOrderId: obs.providerOrderId });
  if (!order) return { kind: "rejected", reason: "unknown_order" };

  const reject = async (reason: string): Promise<ApplyOutcome> => {
    await flagOnce(
      q,
      "review_payment_rejected",
      obs.providerPaymentId,
      {
        order_id: order.id,
        provider_order_id: obs.providerOrderId,
        observed_amount_paise: obs.amountPaise,
        observed_currency: obs.currency,
        observed_state: obs.state,
        source: ctx.source,
        environment: ctx.environment,
      },
      reason,
    );
    return { kind: "rejected", reason };
  };

  if (ctx.accountOk === false) return reject("merchant_account_mismatch");
  if (order.environment !== ctx.environment) return reject("environment_mismatch");
  if (order.provider !== "razorpay") return reject("order_not_razorpay");
  if (String(obs.amountPaise) !== String(order.amount_paise)) return reject("amount_mismatch");
  if (obs.currency !== order.currency) return reject("currency_mismatch");

  const up = await q.query<{ state: PaymentState; captured_at: Date | null; prev_state: PaymentState | null }>(
    `WITH prev AS (SELECT state FROM billing.payments WHERE provider_payment_id = $2)
     INSERT INTO billing.payments AS p
       (order_id, provider_payment_id, amount_paise, currency, state, refunded_paise, captured_at)
     VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7::boolean THEN now() END)
     ON CONFLICT (provider_payment_id) DO UPDATE SET
       state = CASE WHEN ${STATE_RANK_SQL("EXCLUDED.state")} > ${STATE_RANK_SQL("p.state")}
                    THEN EXCLUDED.state ELSE p.state END,
       refunded_paise = GREATEST(p.refunded_paise, EXCLUDED.refunded_paise),
       captured_at = COALESCE(p.captured_at, EXCLUDED.captured_at),
       updated_at = now()
     WHERE p.order_id = EXCLUDED.order_id
     RETURNING p.state, p.captured_at, (SELECT state FROM prev) AS prev_state`,
    [
      order.id,
      obs.providerPaymentId,
      obs.amountPaise,
      obs.currency,
      obs.state,
      obs.refundedPaise,
      obs.captureHappened,
    ],
  );
  const pay = up.rows[0];
  if (!pay) return reject("payment_belongs_to_other_order");
  const state = pay.state;
  const captured = pay.captured_at !== null;
  const changed = pay.prev_state !== state;

  const newStatus = nextOrderStatus(order.status, state, captured, order.machine_code !== null);
  if (newStatus !== order.status) {
    await q.query(`UPDATE billing.orders SET status = $2, updated_at = now() WHERE id = $1`, [
      order.id,
      newStatus,
    ]);
    if (order.status === "cancelled") {
      await flagOnce(q, "review_paid_cancelled_order", order.id, { payment: obs.providerPaymentId }, "captured after cancel");
    }
  }

  let issuance: IssuanceRef | null = null;
  if (captured && (state === "captured" || state === "partially_refunded")) {
    issuance = await ensureEntitlement(q, { ...order, status: newStatus }, "verified_payment", "worker");
    const captures = await q.query<{ n: string }>(
      `SELECT count(*)::text AS n FROM billing.payments WHERE order_id = $1 AND captured_at IS NOT NULL`,
      [order.id],
    );
    if (Number(captures.rows[0]!.n) > 1) {
      await flagOnce(q, "review_duplicate_capture", order.id, { payment: obs.providerPaymentId },
        "more than one captured payment for one order — consider refunding the extra payment");
    }
  }

  if (state === "refunded" || state === "partially_refunded" || state === "disputed") {
    const commercial = state === "refunded" ? "refunded" : state === "disputed" ? "disputed" : null;
    if (commercial) {
      await q.query(
        `UPDATE billing.entitlements SET commercial_status = $2
          WHERE order_id = $1 AND commercial_status <> $2`,
        [order.id, commercial],
      );
    }
    if (changed) {
      await flagOnce(
        q,
        state === "disputed" ? "review_dispute" : "review_refund",
        obs.providerPaymentId,
        { order_id: order.id, state, refunded_paise: obs.refundedPaise },
        state === "disputed"
          ? "dispute recorded — review; an issued offline perpetual licence cannot be remotely disabled"
          : `${state} — review; an issued offline perpetual licence cannot be remotely disabled`,
      );
    }
  }

  return { kind: "recorded", paymentState: state, orderStatus: newStatus, issuance };
}
