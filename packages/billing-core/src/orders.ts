// Public checkout domain: create internal orders from the SERVER catalogue, attach
// the provider order, and resolve the customer's status capability.
import { audit, withTransaction, type PoolLike, type Queryable } from "./db";
import { enqueueJob } from "./jobs";
import { canonicalMachineCode } from "./machine-code";
import { ensureEntitlement, lockOrder, type OrderStatus } from "./provisioning";
import { verifyCheckoutSignature } from "./razorpay-signature";
import { hashStatusToken, looksLikeStatusToken, newStatusToken, STATUS_TOKEN_TTL_DAYS } from "./tokens";
import type { Environment } from "./webhooks";

export interface Product {
  id: string;
  name: string;
  currency: string;
  price_paise: string;
  modules: string[];
}

export async function listActiveProducts(q: Queryable): Promise<Product[]> {
  const r = await q.query<Product>(
    `SELECT id, name, currency, price_paise::text AS price_paise, modules
       FROM billing.products WHERE active ORDER BY created_at, id`,
  );
  return r.rows;
}

export class CheckoutError extends Error {
  constructor(
    public code: "invalid_machine_code" | "unknown_product" | "invalid_input",
    message: string,
  ) {
    super(message);
    this.name = "CheckoutError";
  }
}

export interface CreateOrderInput {
  productId: string;
  customerName: string;
  email: string;
  phone: string | null;
  schoolName: string;
  machineCode: string;
  environment: Environment;
}

export interface CreatedOrder {
  orderId: string;
  statusToken: string;
  amountPaise: number;
  currency: string;
  productName: string;
  machineCode: string;
}

/**
 * Amount, currency and modules come ONLY from the products row (snapshotted onto the
 * order). The browser supplies a product id, contact details and the machine code.
 */
export async function createCheckoutOrder(pool: PoolLike, input: CreateOrderInput): Promise<CreatedOrder> {
  const machineCode = canonicalMachineCode(input.machineCode);
  if (!machineCode) throw new CheckoutError("invalid_machine_code", "machine code is not valid");
  return withTransaction(pool, async (c) => {
    const p = await c.query<Product>(
      `SELECT id, name, currency, price_paise::text AS price_paise, modules
         FROM billing.products WHERE id = $1 AND active`,
      [input.productId],
    );
    const product = p.rows[0];
    if (!product) throw new CheckoutError("unknown_product", "product is not available");
    const cust = await c.query<{ id: string }>(
      `INSERT INTO billing.billing_customers (name, email, phone) VALUES ($1, $2, $3) RETURNING id`,
      [input.customerName, input.email, input.phone],
    );
    const school = await c.query<{ id: string }>(
      `INSERT INTO billing.billing_schools (customer_id, school_name) VALUES ($1, $2) RETURNING id`,
      [cust.rows[0]!.id, input.schoolName],
    );
    const { token, hash } = newStatusToken();
    const o = await c.query<{ id: string }>(
      `INSERT INTO billing.orders (school_id, product_id, amount_paise, currency, modules, machine_code,
                                   environment, provider, status, status_token_hash, status_token_expires_at)
       VALUES ($1, $2, $3::bigint, $4, $5::text[], $6, $7, 'razorpay', 'created', $8,
               now() + make_interval(days => $9))
       RETURNING id`,
      [
        school.rows[0]!.id,
        product.id,
        product.price_paise,
        product.currency,
        product.modules,
        machineCode,
        input.environment,
        hash,
        STATUS_TOKEN_TTL_DAYS,
      ],
    );
    await audit(c, {
      actor: "checkout",
      action: "order_created",
      target: o.rows[0]!.id,
      after: { product_id: product.id, amount_paise: product.price_paise, environment: input.environment },
    });
    return {
      orderId: o.rows[0]!.id,
      statusToken: token,
      amountPaise: Number(product.price_paise),
      currency: product.currency,
      productName: product.name,
      machineCode,
    };
  });
}

export async function attachProviderOrder(q: Queryable, orderId: string, providerOrderId: string): Promise<void> {
  await q.query(
    `UPDATE billing.orders SET provider_order_id = $2, status = 'pending', updated_at = now()
      WHERE id = $1 AND status = 'created' AND provider_order_id IS NULL`,
    [orderId, providerOrderId],
  );
}

export interface CustomerOrderView {
  orderId: string;
  schoolName: string;
  productName: string;
  status: OrderStatus;
  amountPaise: number;
  currency: string;
  machineCode: string | null;
  providerOrderId: string | null;
  environment: Environment;
  licence: {
    licenceId: string;
    status: string;
    licenceKey: string | null;
    vlic: string | null;
    machineCode: string;
  } | null;
  deliveryState: string | null;
}

/**
 * Resolve a status capability to exactly ONE order. Expired, malformed or unknown
 * tokens → null. Never accepts an order id, payment id or school name.
 */
export async function findOrderByStatusToken(q: Queryable, token: string): Promise<CustomerOrderView | null> {
  if (!looksLikeStatusToken(token)) return null;
  const r = await q.query<{
    id: string;
    school_name: string;
    product_name: string;
    status: OrderStatus;
    amount_paise: string;
    currency: string;
    machine_code: string | null;
    provider_order_id: string | null;
    environment: Environment;
  }>(
    `SELECT o.id, s.school_name, p.name AS product_name, o.status, o.amount_paise::text, o.currency,
            o.machine_code, o.provider_order_id, o.environment
       FROM billing.orders o
       JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.products p ON p.id = o.product_id
      WHERE o.status_token_hash = $1 AND o.status_token_expires_at > now()`,
    [hashStatusToken(token)],
  );
  const o = r.rows[0];
  if (!o) return null;
  // Latest generation that is not superseded by a later one.
  const i = await q.query<{
    id: string;
    licence_id: string;
    status: string;
    licence_key: string | null;
    vlic: string | null;
    machine_code: string;
  }>(
    `SELECT i.id, i.licence_id, i.status, i.licence_key, i.vlic, i.machine_code
       FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
      WHERE e.order_id = $1 ORDER BY i.generation DESC LIMIT 1`,
    [o.id],
  );
  const iss = i.rows[0];
  let deliveryState: string | null = null;
  if (iss) {
    const d = await q.query<{ state: string }>(
      `SELECT state FROM billing.deliveries WHERE issuance_id = $1 ORDER BY created_at DESC LIMIT 1`,
      [iss.id],
    );
    deliveryState = d.rows[0]?.state ?? null;
  }
  const issued = iss?.status === "issued";
  return {
    orderId: o.id,
    schoolName: o.school_name,
    productName: o.product_name,
    status: o.status,
    amountPaise: Number(o.amount_paise),
    currency: o.currency,
    machineCode: o.machine_code,
    providerOrderId: o.provider_order_id,
    environment: o.environment,
    licence: iss
      ? {
          licenceId: iss.licence_id,
          status: iss.status,
          // The artefact is only revealed once the signer has produced it.
          licenceKey: issued ? iss.licence_key : null,
          vlic: issued ? iss.vlic : null,
          machineCode: iss.machine_code,
        }
      : null,
    deliveryState,
  };
}

/**
 * Browser checkout return. Verifies the Razorpay checkout signature against the
 * provider order WE stored for the token's order, then enqueues reconciliation.
 * It NEVER marks anything paid: only the worker's verified capture does.
 */
export async function recordCheckoutReturn(
  pool: PoolLike,
  input: { statusToken: string; paymentId: string; signature: string; keySecret: string },
): Promise<"verifying" | "invalid"> {
  if (!looksLikeStatusToken(input.statusToken)) return "invalid";
  const r = await pool.query<{ id: string; provider_order_id: string | null }>(
    `SELECT id, provider_order_id FROM billing.orders
      WHERE status_token_hash = $1 AND status_token_expires_at > now()`,
    [hashStatusToken(input.statusToken)],
  );
  const o = r.rows[0];
  if (!o?.provider_order_id) return "invalid";
  if (!/^pay_[A-Za-z0-9]{1,40}$/.test(input.paymentId)) return "invalid";
  if (!verifyCheckoutSignature(o.provider_order_id, input.paymentId, input.signature, input.keySecret)) {
    return "invalid";
  }
  await enqueueJob(pool, {
    kind: "reconcile_order",
    dedupeKey: `reconcile:${o.id}:checkout:${input.paymentId}`,
    payload: { order_id: o.id },
  });
  return "verifying";
}

export class OrderActionError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "OrderActionError";
  }
}

/**
 * Supply the machine code for a paid order that lacks one (imported/manual order).
 * Allowed once; afterwards changes go through an audited transfer.
 */
export async function completeMachineCode(
  pool: PoolLike,
  input: { orderId: string; machineCode: string; actor: string; reason: string; requestId?: string | null },
) {
  const code = canonicalMachineCode(input.machineCode);
  if (!code) throw new OrderActionError("invalid_machine_code", "machine code is not valid");
  return withTransaction(pool, async (c) => {
    const order = await lockOrder(c, { id: input.orderId });
    if (!order) throw new OrderActionError("not_found", "order not found");
    if (order.machine_code !== null || order.status !== "paid_awaiting_machine_code") {
      throw new OrderActionError("not_awaiting", "order is not awaiting a machine code");
    }
    const captured = await c.query(
      `SELECT 1 FROM billing.payments WHERE order_id = $1 AND captured_at IS NOT NULL
         AND state IN ('captured', 'partially_refunded') LIMIT 1`,
      [order.id],
    );
    const manual = await c.query(
      `SELECT 1 FROM billing.owner_audit WHERE action = 'manual_payment_approved' AND target = $1 LIMIT 1`,
      [order.id],
    );
    const source = captured.rows[0] ? "verified_payment" : manual.rows[0] ? "owner_manual" : null;
    if (!source) throw new OrderActionError("not_paid", "no verified payment or manual approval");
    await c.query(
      `UPDATE billing.orders SET machine_code = $2, status = 'paid', updated_at = now() WHERE id = $1`,
      [order.id, code],
    );
    await audit(c, {
      actor: input.actor,
      action: "machine_code_completed",
      target: order.id,
      before: { machine_code: null },
      after: { machine_code: code },
      reason: input.reason,
      requestId: input.requestId,
    });
    return ensureEntitlement(c, { ...order, machine_code: code, status: "paid" }, source, input.actor, input.requestId);
  });
}

/** Completion via the customer's status capability (scoped to that one order). */
export async function completeMachineCodeByToken(pool: PoolLike, token: string, machineCode: string) {
  if (!looksLikeStatusToken(token)) throw new OrderActionError("not_found", "unknown link");
  const r = await pool.query<{ id: string }>(
    `SELECT id FROM billing.orders WHERE status_token_hash = $1 AND status_token_expires_at > now()`,
    [hashStatusToken(token)],
  );
  if (!r.rows[0]) throw new OrderActionError("not_found", "unknown link");
  return completeMachineCode(pool, {
    orderId: r.rows[0].id,
    machineCode,
    actor: "customer",
    reason: "customer supplied machine code via status link",
  });
}
