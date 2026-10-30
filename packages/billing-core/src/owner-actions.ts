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
 * Re-issue after the signer permanently FAILED an issuance (its job dead-lettered).
 * The signer never resets a failed row, so this inserts generation n+1 with a NEW
 * licence_id (licence_id is UNIQUE) and the same machine code, plus its issue job.
 */
export async function reissueFailedIssuance(pool: PoolLike, actor: Actor, input: { issuanceId: string; reason: string }) {
  const reason = requireReason(input.reason);
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
    await lockOrder(c, { id: s.order_id });
    if (s.status !== "failed") throw new OrderActionError("not_failed", "only a failed issuance can be re-issued");
    if (s.commercial_status !== "active") {
      throw new OrderActionError("blocked", `re-issue is blocked: purchase is ${s.commercial_status}`);
    }
    const latest = await c.query<{ generation: number }>(
      `SELECT max(generation) AS generation FROM billing.licence_issuances WHERE entitlement_id = $1`,
      [s.entitlement_id],
    );
    if (latest.rows[0]!.generation !== s.generation) {
      throw new OrderActionError("not_latest", "a newer generation already exists");
    }
    const licenceId = `VID-${randomUUID()}`;
    const issuedAt = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
    const ni = await c.query<{ id: string }>(
      `INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at, status)
       VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING id`,
      [s.entitlement_id, s.generation + 1, licenceId, s.machine_code, issuedAt],
    );
    const newId = ni.rows[0]!.id;
    await enqueueJob(c, { kind: "issue_licence", dedupeKey: `issue:${newId}`, payload: { issuance_id: newId } });
    await audit(c, {
      actor: actor.ownerId,
      action: "licence_reissued_after_failure",
      target: s.id,
      after: { issuance_id: newId, licence_id: licenceId, generation: s.generation + 1 },
      reason,
      requestId: actor.requestId,
    });
    return { issuanceId: newId, licenceId };
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
