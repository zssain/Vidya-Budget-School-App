// Shared fixtures for the integration suite. All data is synthetic: fake schools,
// fake contacts, TEST FIXTURE prices and fake (non-Razorpay) secrets.
import { randomBytes, randomUUID } from "node:crypto";
import pg from "pg";
import {
  attachProviderOrder,
  createCheckoutOrder,
  ingestRazorpayWebhook,
  signWebhookBody,
  type Environment,
  type MailMessage,
  type Mailer,
  type PoolLike,
  type RazorpayApi,
} from "@vidya/billing-core";
import { createTestDatabase, seedTestProduct, type TestDb } from "@vidya/billing-core/test-support";
import { runOnce, type WorkerDeps } from "../src/worker";

export const WEBHOOK_SECRET = "test-webhook-secret-not-real";
export const ACCOUNT_ID = "acc_TESTFIXTURE";

export interface Ctx {
  db: TestDb;
  worker: PoolLike;
  portal: PoolLike;
  signer: PoolLike;
  admin: PoolLike;
  product: { id: string; pricePaise: number };
}

export async function setup(): Promise<Ctx> {
  const db = await createTestDatabase(pg);
  const product = await seedTestProduct(db.admin);
  return {
    db,
    worker: db.pool("vidya_worker"),
    portal: db.pool("vidya_portal"),
    signer: db.pool("vidya_signer"),
    admin: db.admin,
    product,
  };
}

export const MC_A = "R69C-090M-XG7X-1"; // from vidya-core machine_code("pc-1")
export const MC_B = "RD9F-FC95-VGVQ-3"; // machine_code("pc-2")

export async function newOrder(ctx: Ctx, opts: { environment?: Environment; school?: string } = {}) {
  const created = await createCheckoutOrder(ctx.portal, {
    productId: ctx.product.id,
    customerName: "Test Buyer",
    email: `buyer+${randomBytes(3).toString("hex")}@example.test`,
    phone: null,
    schoolName: opts.school ?? "Fake Test School",
    machineCode: MC_A,
    environment: opts.environment ?? "test",
  });
  const providerOrderId = `order_T${randomBytes(6).toString("hex")}`;
  await attachProviderOrder(ctx.portal, created.orderId, providerOrderId);
  return { ...created, providerOrderId };
}

export function paymentEntity(p: {
  id?: string;
  orderId: string;
  amount: number;
  currency?: string;
  status?: string;
  amountRefunded?: number;
  refundStatus?: "partial" | "full" | null;
}) {
  return {
    id: p.id ?? `pay_T${randomBytes(6).toString("hex")}`,
    entity: "payment",
    amount: p.amount,
    currency: p.currency ?? "INR",
    status: p.status ?? "captured",
    order_id: p.orderId,
    captured: p.status === undefined || p.status === "captured" || p.status === "refunded",
    amount_refunded: p.amountRefunded ?? 0,
    refund_status: p.refundStatus ?? null,
    email: "buyer@example.test",
  };
}

export function eventBody(type: string, payment: Record<string, unknown>, extra: Record<string, unknown> = {}) {
  return JSON.stringify({
    entity: "event",
    account_id: ACCOUNT_ID,
    event: type,
    contains: ["payment", ...Object.keys(extra)],
    payload: {
      payment: { entity: payment },
      ...Object.fromEntries(Object.entries(extra).map(([k, v]) => [k, { entity: v }])),
    },
    created_at: 1767225600,
  });
}

export async function postWebhook(
  ctx: Ctx,
  body: string,
  opts: { eventId?: string; environment?: Environment; signature?: string } = {},
) {
  const raw = Buffer.from(body, "utf8");
  return ingestRazorpayWebhook(ctx.worker, {
    rawBody: raw,
    signature: opts.signature ?? signWebhookBody(raw, WEBHOOK_SECRET),
    eventIdHeader: opts.eventId ?? `evt_${randomUUID()}`,
    secret: WEBHOOK_SECRET,
    environment: opts.environment ?? "test",
  });
}

export class FakeMailer implements Mailer {
  sent: MailMessage[] = [];
  failuresLeft = 0;
  async send(msg: MailMessage) {
    if (this.failuresLeft > 0) {
      this.failuresLeft--;
      throw new Error("simulated email provider outage (503)");
    }
    this.sent.push(msg);
    return { id: `email_${this.sent.length}` };
  }
}

export class FakeRazorpay implements RazorpayApi {
  payments = new Map<string, unknown[]>();
  outage = false;
  calls = 0;
  async fetchOrderPayments(id: string) {
    this.calls++;
    if (this.outage) throw new Error("simulated Razorpay API outage (502)");
    return this.payments.get(id) ?? [];
  }
  async createOrder() {
    return { id: `order_T${randomBytes(6).toString("hex")}` };
  }
}

export function deps(ctx: Ctx, over: Partial<WorkerDeps> = {}): WorkerDeps & { mailer: FakeMailer; razorpay: FakeRazorpay } {
  return {
    pool: ctx.worker,
    razorpay: new FakeRazorpay(),
    mailer: new FakeMailer(),
    workerId: `test-worker-${randomBytes(3).toString("hex")}`,
    environment: "test",
    fromEmail: "Vidya Test <licences@example.test>",
    leaseSeconds: 60,
    expectedAccountId: ACCOUNT_ID,
    webhookRetentionDays: 30,
    ...over,
  } as WorkerDeps & { mailer: FakeMailer; razorpay: FakeRazorpay };
}

export async function drain(d: WorkerDeps, max = 100): Promise<number> {
  let n = 0;
  while (n < max && (await runOnce(d))) n++;
  return n;
}

/**
 * Stand-in for the Rust signer, acting with the REAL vidya_signer role so the
 * contract's grants are exercised: fill the artefact columns and mark 'issued'.
 */
export async function fakeSigner(ctx: Ctx, issuanceId: string) {
  await ctx.signer.query(
    `UPDATE billing.licence_issuances
        SET payload = convert_to('{"v":2}', 'UTF8'), signature = '\\x00'::bytea,
            licence_key = 'TESTKEY' || replace(id::text, '-', ''), vlic = 'VLIC-' || licence_id,
            signer_key_id = 'test-key', status = 'issued', issued_ts = now()
      WHERE id = $1 AND status = 'pending'`,
    [issuanceId],
  );
  await ctx.signer.query(
    `UPDATE billing.jobs SET state = 'done', completed_at = now() WHERE dedupe_key = $1`,
    [`issue:${issuanceId}`],
  );
}

export async function counts(ctx: Ctx, orderId: string) {
  const r = await ctx.admin.query<{ ent: string; iss: string; jobs: string; pays: string }>(
    `SELECT (SELECT count(*) FROM billing.entitlements WHERE order_id = $1)::text AS ent,
            (SELECT count(*) FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
              WHERE e.order_id = $1)::text AS iss,
            (SELECT count(*) FROM billing.jobs j WHERE j.kind = 'issue_licence' AND j.payload->>'issuance_id' IN (
               SELECT i.id::text FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
                WHERE e.order_id = $1))::text AS jobs,
            (SELECT count(*) FROM billing.payments WHERE order_id = $1)::text AS pays`,
    [orderId],
  );
  const x = r.rows[0]!;
  return { entitlements: Number(x.ent), issuances: Number(x.iss), issueJobs: Number(x.jobs), payments: Number(x.pays) };
}

export async function orderRow(ctx: Ctx, orderId: string) {
  const r = await ctx.admin.query(`SELECT * FROM billing.orders WHERE id = $1`, [orderId]);
  return r.rows[0]!;
}

export async function issuanceFor(ctx: Ctx, orderId: string) {
  const r = await ctx.admin.query(
    `SELECT i.* FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
      WHERE e.order_id = $1 ORDER BY generation`,
    [orderId],
  );
  return r.rows;
}
