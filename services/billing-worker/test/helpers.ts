// Shared fixtures for the integration suite. All data is synthetic: fake schools,
// fake contacts, TEST FIXTURE prices and fake (non-Razorpay) secrets.
import { createPrivateKey, createPublicKey, randomBytes, randomUUID, sign as edSign, verify as edVerify } from "node:crypto";
import pg from "pg";
import {
  attachProviderOrder,
  createCheckoutOrder,
  groupKey,
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

// ---------------------------------------------------------------------------------
// A REAL test-key signer (ed25519 via node:crypto) producing the exact v2 artefact
// format of services/licence-signer / tools/licence-maker. It is pinned to the
// cross-crate golden vector (seed [42;32]) in durability.test.ts, so artefacts it
// writes are byte-compatible with what vidya_core::licence::verify_v2 accepts.
// TEST KEY ONLY — never a production key.

/** The golden key pinned by licence-maker, vidya-core and services/licence-signer/tests/golden.rs. */
export const GOLDEN_KEY =
  "eyJ2IjoyLCJsaWNlbmNlX2lkIjoiMDE5M2I2YzAtMDAwMC03MDAwLTgwMDAtMDAwMDAwMDAwMDAxIiwic2Nob29sX25hbWUiOiJTYXJhc3dhdGkgUHVibGljIFNjaG9vbCIsIm1hY2hpbmVfY29kZSI6IjNOMFMtMTBZWC1KUUNSLVQiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTI1VDAwOjAwOjAwWiIsInBsYW4iOiJwZXJwZXR1YWwiLCJtb2R1bGVzIjpbImNvcmUiXX0sel8zbFpx8xCuDxYhHoM9kZ6raBqbSkf-O3JKmdQ5t2OK9Z0ZYQO3RhjJdKu2gdWndFijhL48Powb0xSDAIoD";

const PKCS8_ED25519 = Buffer.from("302e020100300506032b657004220420", "hex");
export const TEST_SIGNING_KEY = createPrivateKey({
  key: Buffer.concat([PKCS8_ED25519, Buffer.alloc(32, 42)]),
  format: "der",
  type: "pkcs8",
});
export const TEST_PUBLIC_KEY = createPublicKey(TEST_SIGNING_KEY);

export interface LicenceFields {
  licence_id: string;
  school_name: string;
  machine_code: string;
  issued_at: string;
  modules: string[];
}

/** serde_json::to_vec(&LicenceV2{…}) — same field order, compact. */
export function licencePayload(f: LicenceFields): Buffer {
  return Buffer.from(
    JSON.stringify({ v: 2, licence_id: f.licence_id, school_name: f.school_name, machine_code: f.machine_code, issued_at: f.issued_at, plan: "perpetual", modules: f.modules }),
    "utf8",
  );
}

export function signArtefact(f: LicenceFields) {
  const payload = licencePayload(f);
  const signature = edSign(null, payload, TEST_SIGNING_KEY);
  const vlic = Buffer.concat([payload, signature]).toString("base64url");
  return { payload, signature, vlic, licenceKey: groupKey(vlic) };
}

/** Parse a key/.vlic (whitespace ignored) and verify it like verify_v2: signature + machine binding. */
export function verifyArtefact(text: string, machineCode: string): LicenceFields & { v: number; plan: string } {
  const bytes = Buffer.from(text.replace(/\s+/g, ""), "base64url");
  const payload = bytes.subarray(0, bytes.length - 64);
  const signature = bytes.subarray(bytes.length - 64);
  if (!edVerify(null, payload, TEST_PUBLIC_KEY, signature)) throw new Error("signature does not verify");
  const lic = JSON.parse(payload.toString("utf8"));
  if (lic.v !== 2 || lic.plan !== "perpetual") throw new Error("not a v2 perpetual licence");
  if (lic.machine_code !== machineCode) throw new Error("licence is bound to another machine");
  return lic;
}

/** Sign a pending issuance as the REAL vidya_signer role (like the Rust signer's commit). */
export async function realSigner(ctx: Ctx, issuanceId: string) {
  const r = await ctx.signer.query<{ licence_id: string; machine_code: string; issued_at: string; school_name: string; modules: string[] }>(
    `SELECT i.licence_id, i.machine_code, i.issued_at, e.school_name, e.modules
       FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id WHERE i.id = $1`,
    [issuanceId],
  );
  const a = signArtefact({ ...r.rows[0]! });
  await ctx.signer.query(
    `UPDATE billing.licence_issuances
        SET payload = $2, signature = $3, licence_key = $4, vlic = $5, signer_key_id = 'test-golden-42',
            status = 'issued', issued_ts = now()
      WHERE id = $1 AND status = 'pending'`,
    [issuanceId, a.payload, a.signature, a.licenceKey, a.vlic],
  );
  await ctx.signer.query(
    `UPDATE billing.jobs SET state = 'done', completed_at = now()
      WHERE kind = 'issue_licence' AND payload->>'issuance_id' = $1 AND state IN ('ready', 'running')`,
    [issuanceId],
  );
  return a;
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
