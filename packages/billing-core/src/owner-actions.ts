// Audited owner actions. The portal calls these ONLY after requireOwner() (AAL2,
// owner UUID) and, for sensitive ones, a fresh step-up. Each writes owner_audit in
// the same transaction as its effect.
import { randomUUID } from "node:crypto";
import { audit, withTransaction, type PoolLike } from "./db";
import { enqueueJob } from "./jobs";
import { canonicalMachineCode } from "./machine-code";
import { OrderActionError } from "./orders";
import { ensureEntitlement, lockOrder } from "./provisioning";
import type { Environment } from "./webhooks";

export interface Actor {
  ownerId: string;
  requestId?: string | null;
}

function requireReason(reason: string): string {
  const r = reason.trim();
  if (r.length < 5 || r.length > 1000) throw new OrderActionError("reason_required", "a reason (5–1000 chars) is required");
  return r;
}

/** Owner records an off-platform (imported) order, e.g. a bank transfer customer. */
export async function createManualOrder(
  pool: PoolLike,
  actor: Actor,
  input: {
    productId: string;
    customerName: string;
    email: string;
    phone: string | null;
    schoolName: string;
    machineCode: string | null;
    environment: Environment;
    reason: string;
  },
): Promise<string> {
  const reason = requireReason(input.reason);
  let code: string | null = null;
  if (input.machineCode && input.machineCode.trim() !== "") {
    code = canonicalMachineCode(input.machineCode);
    if (!code) throw new OrderActionError("invalid_machine_code", "machine code is not valid");
  }
  return withTransaction(pool, async (c) => {
    const p = await c.query<{ id: string; price_paise: string; currency: string; modules: string[] }>(
      `SELECT id, price_paise::text, currency, modules FROM billing.products WHERE id = $1`,
      [input.productId],
    );
    const product = p.rows[0];
    if (!product) throw new OrderActionError("unknown_product", "unknown product");
    const cust = await c.query<{ id: string }>(
      `INSERT INTO billing.billing_customers (name, email, phone) VALUES ($1, $2, $3) RETURNING id`,
      [input.customerName, input.email, input.phone],
    );
    const school = await c.query<{ id: string }>(
      `INSERT INTO billing.billing_schools (customer_id, school_name) VALUES ($1, $2) RETURNING id`,
      [cust.rows[0]!.id, input.schoolName],
    );
    const o = await c.query<{ id: string }>(
      `INSERT INTO billing.orders (school_id, product_id, amount_paise, currency, modules, machine_code,
                                   environment, provider, status)
       VALUES ($1, $2, $3::bigint, $4, $5::text[], $6, $7, 'manual', 'pending') RETURNING id`,
      [school.rows[0]!.id, product.id, product.price_paise, product.currency, product.modules, code, input.environment],
    );
    await audit(c, {
      actor: actor.ownerId,
      action: "manual_order_created",
      target: o.rows[0]!.id,
      after: { product_id: product.id, amount_paise: product.price_paise, machine_code: code },
      reason,
      requestId: actor.requestId,
    });
    return o.rows[0]!.id;
  });
}

/**
 * MANUAL PAYMENT APPROVAL — a separately labelled owner action, not a verified
 * provider payment. No payments row is created (verified-sales totals exclude it);
 * the entitlement source is 'owner_manual'.
 */
export async function approveManualPayment(
  pool: PoolLike,
  actor: Actor,
  input: { orderId: string; reference: string; reason: string },
) {
  const reason = requireReason(input.reason);
  const reference = input.reference.trim().slice(0, 200);
  if (!reference) throw new OrderActionError("reference_required", "a payment reference is required");
  return withTransaction(pool, async (c) => {
    const order = await lockOrder(c, { id: input.orderId });
    if (!order) throw new OrderActionError("not_found", "order not found");
    if (order.provider !== "manual") {
      throw new OrderActionError("not_manual", "only manual orders can be approved manually; reconcile Razorpay orders instead");
    }
    if (order.status !== "pending" && order.status !== "created") {
      throw new OrderActionError("bad_state", `order is ${order.status}`);
    }
    const status = order.machine_code ? "paid" : "paid_awaiting_machine_code";
    await c.query(`UPDATE billing.orders SET status = $2, updated_at = now() WHERE id = $1`, [order.id, status]);
    await audit(c, {
      actor: actor.ownerId,
      action: "manual_payment_approved",
      target: order.id,
      before: { status: order.status },
      after: { status, reference, label: "MANUAL APPROVAL (not a verified provider payment)" },
      reason,
      requestId: actor.requestId,
    });
    return ensureEntitlement(c, { ...order, status }, "owner_manual", actor.ownerId, actor.requestId);
  });
}

export async function cancelOrder(pool: PoolLike, actor: Actor, input: { orderId: string; reason: string }) {
  const reason = requireReason(input.reason);
  return withTransaction(pool, async (c) => {
    const order = await lockOrder(c, { id: input.orderId });
    if (!order) throw new OrderActionError("not_found", "order not found");
    if (order.status !== "created" && order.status !== "pending") {
      throw new OrderActionError("bad_state", `cannot cancel a ${order.status} order`);
    }
    await c.query(`UPDATE billing.orders SET status = 'cancelled', updated_at = now() WHERE id = $1`, [order.id]);
    await audit(c, {
      actor: actor.ownerId,
      action: "order_cancelled",
      target: order.id,
      before: { status: order.status },
      after: { status: "cancelled" },
      reason,
      requestId: actor.requestId,
    });
  });
}

export async function requestReconcile(pool: PoolLike, actor: Actor, orderId: string) {
  const id = randomUUID();
  await withTransaction(pool, async (c) => {
    await enqueueJob(c, { kind: "reconcile_order", dedupeKey: `reconcile:${orderId}:owner:${id}`, payload: { order_id: orderId } });
    await audit(c, { actor: actor.ownerId, action: "reconcile_requested", target: orderId, requestId: actor.requestId });
  });
}

/** Resend the EXISTING artefact. Never creates an issuance. */
export async function requestResend(pool: PoolLike, actor: Actor, input: { issuanceId: string; reason: string }) {
  const reason = requireReason(input.reason);
  return withTransaction(pool, async (c) => {
    const i = await c.query<{ status: string }>(`SELECT status FROM billing.licence_issuances WHERE id = $1`, [input.issuanceId]);
    if (!i.rows[0]) throw new OrderActionError("not_found", "issuance not found");
    if (i.rows[0].status !== "issued") throw new OrderActionError("not_issued", "licence has not been issued yet");
    const key = `resend:${input.issuanceId}:${randomUUID()}`;
    await enqueueJob(c, {
      kind: "deliver_licence",
      dedupeKey: key,
      payload: { issuance_id: input.issuanceId, idempotency_key: key },
    });
    await audit(c, {
      actor: actor.ownerId,
      action: "licence_resend_requested",
      target: input.issuanceId,
      reason,
      requestId: actor.requestId,
    });
  });
}

/**
 * Transfer: a replacement issuance (generation n+1) bound to a new machine code.
 * The old licence keeps verifying offline on the old PC — this is recorded as a
 * commercial supersession, NOT a remote disable. Blocked for refunded/disputed.
 */
export async function transferLicence(
  pool: PoolLike,
  actor: Actor,
  input: { issuanceId: string; newMachineCode: string; reason: string },
) {
  const reason = requireReason(input.reason);
  const code = canonicalMachineCode(input.newMachineCode);
  if (!code) throw new OrderActionError("invalid_machine_code", "machine code is not valid");
  return withTransaction(pool, async (c) => {
    const src = await c.query<{
      id: string;
      entitlement_id: string;
      generation: number;
      status: string;
      machine_code: string;
      order_id: string;
      commercial_status: string;
    }>(
      `SELECT i.id, i.entitlement_id, i.generation, i.status, i.machine_code, e.order_id, e.commercial_status
         FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
        WHERE i.id = $1`,
      [input.issuanceId],
    );
    const s = src.rows[0];
    if (!s) throw new OrderActionError("not_found", "issuance not found");
    // Serialise all changes to this purchase on its order row.
    await lockOrder(c, { id: s.order_id });
    if (s.commercial_status !== "active") {
      throw new OrderActionError("blocked", `transfers are blocked: purchase is ${s.commercial_status}`);
    }
    const latest = await c.query<{ generation: number }>(
      `SELECT max(generation) AS generation FROM billing.licence_issuances WHERE entitlement_id = $1`,
      [s.entitlement_id],
    );
    if (latest.rows[0]!.generation !== s.generation) {
      throw new OrderActionError("not_latest", "only the latest generation can be transferred");
    }
    if (s.status !== "issued") throw new OrderActionError("not_issued", "the current licence is not issued yet");
    if (s.machine_code === code) throw new OrderActionError("same_machine", "new machine code equals the current one");
    const licenceId = `VID-${randomUUID()}`;
    const issuedAt = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
    const ni = await c.query<{ id: string }>(
      `INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at, status)
       VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING id`,
      [s.entitlement_id, s.generation + 1, licenceId, code, issuedAt],
    );
    const newId = ni.rows[0]!.id;
    const t = await c.query<{ id: string }>(
      `INSERT INTO billing.licence_transfers (source_issuance_id, target_machine_code, reason, replacement_issuance_id)
       VALUES ($1, $2, $3, $4) RETURNING id`,
      [s.id, code, reason, newId],
    );
    await enqueueJob(c, { kind: "issue_licence", dedupeKey: `issue:${newId}`, payload: { issuance_id: newId } });
    await audit(c, {
      actor: actor.ownerId,
      action: "licence_transferred",
      target: s.id,
      before: { machine_code: s.machine_code, generation: s.generation },
      after: { machine_code: code, generation: s.generation + 1, issuance_id: newId, licence_id: licenceId, transfer_id: t.rows[0]!.id },
      reason,
      requestId: actor.requestId,
    });
    return { issuanceId: newId, licenceId, transferId: t.rows[0]!.id };
  });
}

/**
 * RETRY ISSUANCE after the signer FAILED it (its job dead-lettered). Owner decision
 * 2026-11-05: the order stays "Paid · Licence pending" and the retry reuses the SAME
 * issuance row — same entitlement, generation, licence_id, machine code and
 * issued_at (and the payload bytes, if the signer already persisted them). No new
 * charge, no new licence, no new generation. Only a transfer creates a new issuance.
 *
 * Effect (one transaction, serialised on the order row, then the issuance row):
 *   status failed → pending, last_error cleared (the 0003 trigger allows this only
 *   while no signature exists); a FRESH issue_licence job `issue:<id>:retry:<n>`.
 *   The dead job rows are never reused or rewritten, and the owner_audit row
 *   `licence_issue_retry_requested` records the failed attempts being retried, so
 *   every failed attempt stays in the history.
 *
 * services/licence-signer/tests/common/mod.rs (`portal_retry_issuance`) mirrors
 * these statements as the vidya_portal role for the signer's end-to-end tests.
 */
export async function retryFailedIssuance(pool: PoolLike, actor: Actor, input: { issuanceId: string; reason: string }) {
  const reason = requireReason(input.reason);
  return withTransaction(pool, async (c) => {
    const ref = await c.query<{ order_id: string }>(
      `SELECT e.order_id FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
        WHERE i.id = $1`,
      [input.issuanceId],
    );
    if (!ref.rows[0]) throw new OrderActionError("not_found", "issuance not found");
    // Serialise all changes to this purchase on its order row, then lock the issuance.
    await lockOrder(c, { id: ref.rows[0].order_id });
    const src = await c.query<{
      id: string;
      entitlement_id: string;
      generation: number;
      status: string;
      licence_id: string;
      machine_code: string;
      last_error: string | null;
      signed: boolean;
      commercial_status: string;
    }>(
      `SELECT i.id, i.entitlement_id, i.generation, i.status, i.licence_id, i.machine_code, i.last_error,
              i.signature IS NOT NULL AS signed, e.commercial_status
         FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
        WHERE i.id = $1 FOR UPDATE OF i`,
      [input.issuanceId],
    );
    const s = src.rows[0]!;
    if (s.status !== "failed" || s.signed) {
      throw new OrderActionError("not_failed", "only a failed, unsigned issuance can be retried");
    }
    if (s.commercial_status !== "active") {
      throw new OrderActionError("blocked", `retry is blocked: purchase is ${s.commercial_status}`);
    }
    const latest = await c.query<{ generation: number }>(
      `SELECT max(generation) AS generation FROM billing.licence_issuances WHERE entitlement_id = $1`,
      [s.entitlement_id],
    );
    if (latest.rows[0]!.generation !== s.generation) {
      throw new OrderActionError("not_latest", "a newer generation already exists");
    }
    const history = await c.query<{ id: string; dedupe_key: string; state: string; attempts: number; last_error: string | null }>(
      `SELECT id, dedupe_key, state, attempts, last_error FROM billing.jobs
        WHERE kind = 'issue_licence' AND payload->>'issuance_id' = $1 ORDER BY created_at`,
      [s.id],
    );
    if (history.rows.some((j) => j.state === "ready" || j.state === "running")) {
      throw new OrderActionError("retry_in_progress", "an issue job for this licence is already queued");
    }
    await c.query(
      `UPDATE billing.licence_issuances SET status = 'pending', last_error = NULL WHERE id = $1 AND status = 'failed'`,
      [s.id],
    );
    const job = await enqueueJob(c, {
      kind: "issue_licence",
      dedupeKey: `issue:${s.id}:retry:${history.rows.length}`,
      payload: { issuance_id: s.id },
    });
    if (!job.created) throw new OrderActionError("retry_in_progress", "a retry for this licence is already queued");
    await audit(c, {
      actor: actor.ownerId,
      action: "licence_issue_retry_requested",
      target: s.id,
      before: {
        status: "failed",
        last_error: s.last_error,
        failed_jobs: history.rows.map((j) => ({ job_id: j.id, dedupe_key: j.dedupe_key, state: j.state, attempts: j.attempts, last_error: j.last_error })),
      },
      after: { status: "pending", licence_id: s.licence_id, generation: s.generation, machine_code: s.machine_code, job_id: job.id },
      reason,
      requestId: actor.requestId,
    });
    return { issuanceId: s.id, licenceId: s.licence_id, jobId: job.id! };
  });
}

/** Owner acknowledges a review flag (refund/dispute/rejection) — audit only. */
export async function acknowledgeReview(pool: PoolLike, actor: Actor, input: { auditId: string; note: string }) {
  const reason = requireReason(input.note);
  await audit(pool, {
    actor: actor.ownerId,
    action: "review_acknowledged",
    target: input.auditId,
    reason,
    requestId: actor.requestId,
  });
}
