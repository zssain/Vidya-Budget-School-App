// Purchase → licence rehearsal, cross-stack (docs/implementation/runbooks/purchase-rehearsal.md).
// Driven by scripts/rehearse-purchase.sh — NOT part of `npm test` (own vitest config,
// needs the env below). Everything is synthetic: FAKE schools, a Razorpay-format TEST
// webhook signed with a TEST secret, a throwaway ed25519 TEST signing key.
//
//   VIDYA_REHEARSAL_DIR    shared dir (machine codes in, artefacts + issued.json out)
//   LICENCE_SIGNER_BIN     the real services/licence-signer binary
//   BILLING_TEST_ADMIN_URL Postgres 16 admin URL (a fresh DB is created + dropped)
//
// Flow A (purchase): checkout order for the app-generated machine code → Razorpay
// checkout return (signed) → signed payment.captured webhook into the real ingest
// handler → the real billing-worker job loop provisions → the REAL Rust signer binary
// signs → the worker delivers (fake mailer) → the stored .vlic / licence key are
// written for the app's activation test (src-tauri/tests/purchase_activation.rs).
//
// Flow B (retry across languages): the Rust signer FAILS on its only attempt → issuance
// `failed`, order still paid → TS owner action retryFailedIssuance re-queues → the Rust
// signer issues with the SAME licence_id and byte-identical payload.
import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import { chmodSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import pg from "pg";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  attachProviderOrder,
  createCheckoutOrder,
  findOrderByStatusToken,
  fulfilmentState,
  ingestRazorpayWebhook,
  recordCheckoutReturn,
  retryFailedIssuance,
  signCheckout,
  signWebhookBody,
} from "@vidya/billing-core";
import { ADMIN_URL, createTestDatabase, seedTestProduct, type TestDb } from "@vidya/billing-core/test-support";
import { runOnce, type WorkerDeps } from "../src/worker";

const DIR = process.env.VIDYA_REHEARSAL_DIR ?? "";
const SIGNER_BIN = process.env.LICENCE_SIGNER_BIN ?? "";
const WEBHOOK_SECRET = "rehearsal-webhook-secret-TEST-ONLY";
const KEY_SECRET = "rehearsal-key-secret-TEST-ONLY";
const ACCOUNT_ID = "acc_REHEARSALTEST";
const OWNER = { ownerId: "00000000-0000-4000-8000-00000000beef", requestId: "rehearsal" };

let db: TestDb;
let product: { id: string; pricePaise: number };
let buyerMachine: string;
let keyFile: string;
let fingerprint: string;
const issued: Record<string, unknown> = {};

const sent: { to: string; attachments: { filename: string; content: string }[] }[] = [];
function deps(): WorkerDeps {
  return {
    pool: db.pool("vidya_worker"),
    razorpay: {
      async fetchOrderPayments() {
        return []; // reconcile finds nothing new; the webhook is the source of truth here
      },
      async createOrder() {
        return { id: `order_R${randomBytes(6).toString("hex")}` };
      },
    },
    mailer: {
      async send(m) {
        sent.push({ to: m.to, attachments: m.attachments });
        return { id: `email_${sent.length}` };
      },
    },
    workerId: "rehearsal-billing-worker",
    environment: "test",
    fromEmail: "Vidya Rehearsal <licences@example.test>",
    leaseSeconds: 60,
    expectedAccountId: ACCOUNT_ID,
    webhookRetentionDays: 30,
  };
}

async function drain(max = 100) {
  let n = 0;
  while (n < max && (await runOnce(deps()))) n++;
  return n;
}

function run(cmd: string, args: string[], env: Record<string, string>): Promise<{ code: number | null; out: string }> {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { env: { PATH: process.env.PATH ?? "", ...env } });
    let out = "";
    p.stdout.on("data", (d) => (out += d));
    p.stderr.on("data", (d) => (out += d));
    p.on("error", reject);
    p.on("close", (code) => resolve({ code, out }));
  });
}

/** Run the REAL signer binary against the rehearsal DB as vidya_signer until `done()`, then SIGTERM it. */
async function runSigner(done: () => Promise<boolean>): Promise<string> {
  const u = new URL(ADMIN_URL);
  u.pathname = `/${db.name}`;
  u.searchParams.set("options", "-c role=vidya_signer");
  const child = spawn(SIGNER_BIN, [], {
    env: {
      PATH: process.env.PATH ?? "",
      LICENCE_SIGNING_KEY_FILE: keyFile,
      LICENCE_SIGNING_KEY_ID: "rehearsal-test-key",
      EXPECTED_PUBLIC_KEY_FINGERPRINT: fingerprint,
      SIGNER_DATABASE_URL: u.toString(),
      SIGNER_POLL_INTERVAL_SECS: "1",
    },
  });
  let logs = "";
  child.stdout.on("data", (d) => (logs += d));
  child.stderr.on("data", (d) => (logs += d));
  const exited = new Promise<number | null>((r) => child.on("close", (c) => r(c)));
  let ok = false;
  for (let i = 0; i < 200 && child.exitCode === null; i++) {
    if (await done()) {
      ok = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  child.kill("SIGTERM");
  const code = await exited;
  expect(ok, `signer did not reach the expected state:\n${logs}`).toBe(true);
  expect(code, logs).toBe(0);
  expect(logs).toContain("signing key loaded and matches the expected fingerprint");
  expect(logs).not.toContain(readFileSync(keyFile, "utf8").trim()); // no key material in logs
  return logs;
}

async function issuance(orderId: string) {
  const r = await db.admin.query(
    `SELECT i.* FROM billing.licence_issuances i JOIN billing.entitlements e ON e.id = i.entitlement_id
      WHERE e.order_id = $1 ORDER BY generation`,
    [orderId],
  );
  return r.rows;
}

/** Customer checkout: order → Razorpay order → checkout return → signed capture webhook → worker. */
async function buy(schoolName: string) {
  const portal = db.pool("vidya_portal");
  const created = await createCheckoutOrder(portal, {
    productId: product.id,
    customerName: "Rehearsal Buyer (FAKE)",
    email: `rehearsal+${randomBytes(3).toString("hex")}@example.test`,
    phone: null,
    schoolName,
    machineCode: buyerMachine,
    environment: "test",
  });
  expect(created.machineCode).toBe(buyerMachine);
  const providerOrderId = (await deps().razorpay.createOrder({ amount: created.amountPaise, currency: "INR", receipt: "r", notes: {} })).id;
  await attachProviderOrder(portal, created.orderId, providerOrderId);

  const paymentId = `pay_R${randomBytes(6).toString("hex")}`;
  // Razorpay checkout handler → portal verify (HMAC over order_id|payment_id).
  expect(
    await recordCheckoutReturn(portal, {
      statusToken: created.statusToken,
      paymentId,
      signature: signCheckout(providerOrderId, paymentId, KEY_SECRET),
      keySecret: KEY_SECRET,
    }),
  ).toBe("verifying");

  // Razorpay-format payment.captured webhook, signed with the TEST webhook secret.
  const body = JSON.stringify({
    entity: "event",
    account_id: ACCOUNT_ID,
    event: "payment.captured",
    contains: ["payment"],
    payload: {
      payment: {
        entity: {
          id: paymentId,
          entity: "payment",
          amount: created.amountPaise,
          currency: "INR",
          status: "captured",
          order_id: providerOrderId,
          captured: true,
          amount_refunded: 0,
          refund_status: null,
          email: "rehearsal@example.test",
        },
      },
    },
    created_at: Math.floor(Date.now() / 1000),
  });
  const raw = Buffer.from(body, "utf8");
  // A forged signature is refused before anything is stored.
  expect(
    await ingestRazorpayWebhook(db.pool("vidya_worker"), {
      rawBody: raw,
      signature: signWebhookBody(raw, "attacker-secret"),
      eventIdHeader: `evt_forged_${paymentId}`,
      secret: WEBHOOK_SECRET,
      environment: "test",
    }),
  ).toMatchObject({ ok: false, status: 401 });
  const ingest = await ingestRazorpayWebhook(db.pool("vidya_worker"), {
    rawBody: raw,
    signature: signWebhookBody(raw, WEBHOOK_SECRET),
    eventIdHeader: `evt_${paymentId}`,
    secret: WEBHOOK_SECRET,
    environment: "test",
  });
  expect(ingest).toMatchObject({ ok: true, duplicate: false });

  await drain(); // process_webhook → payment + entitlement + issuance + issue_licence job
  const o = await db.admin.query(`SELECT status FROM billing.orders WHERE id = $1`, [created.orderId]);
  expect(o.rows[0].status).toBe("paid");
  const [iss] = await issuance(created.orderId);
  expect(iss).toMatchObject({ status: "pending", generation: 1, machine_code: buyerMachine });
  return { ...created, providerOrderId, paymentId, issuanceId: iss.id as string, licenceId: iss.licence_id as string };
}

/** After issuance: the worker delivers the email; the stored artefact goes to disk for the app. */
async function deliverAndExport(tag: string, o: { orderId: string; statusToken: string; issuanceId: string; licenceId: string }) {
  sent.length = 0;
  await drain();
  const [iss] = await issuance(o.orderId);
  expect(iss.status).toBe("issued");
  expect(iss.licence_id).toBe(o.licenceId);
  expect(iss.signer_key_id).toBe("rehearsal-test-key");
  expect(sent).toHaveLength(1);
  const att = sent[0]!.attachments.find((a) => a.filename.endsWith(".vlic"));
  expect(att?.content).toBe(iss.vlic);
  const view = await findOrderByStatusToken(db.pool("vidya_portal"), o.statusToken);
  expect(view?.licence?.licenceKey).toBe(iss.licence_key);
  writeFileSync(join(DIR, `${tag}.vlic`), att!.content);
  writeFileSync(join(DIR, `${tag}.licence-key.txt`), view!.licence!.licenceKey!);
  issued[tag] = { licence_id: iss.licence_id, issuance_id: iss.id, order_id: o.orderId };
  return iss;
}

beforeAll(async () => {
  if (!DIR || !SIGNER_BIN) throw new Error("run via scripts/rehearse-purchase.sh (VIDYA_REHEARSAL_DIR, LICENCE_SIGNER_BIN)");
  buyerMachine = readFileSync(join(DIR, "buyer.machine-code"), "utf8").trim();
  // Throwaway TEST signing key (licence-maker key-file format: base64 of the 32-byte seed).
  keyFile = join(DIR, "test-signing.key");
  writeFileSync(keyFile, randomBytes(32).toString("base64") + "\n");
  chmodSync(keyFile, 0o600);
  const fp = await run(SIGNER_BIN, ["fingerprint"], { LICENCE_SIGNING_KEY_FILE: keyFile });
  expect(fp.code, fp.out).toBe(0);
  const pub = /public_key_base64:\s*(\S+)/.exec(fp.out)?.[1];
  fingerprint = /fingerprint:\s*(\S+)/.exec(fp.out)?.[1] ?? "";
  expect(pub && fingerprint).toBeTruthy();
  writeFileSync(join(DIR, "test-public.key"), pub + "\n");

  db = await createTestDatabase(pg);
  product = await seedTestProduct(db.admin);
}, 60_000);

afterAll(async () => {
  if (DIR) writeFileSync(join(DIR, "issued.json"), JSON.stringify(issued, null, 2) + "\n");
  await db?.drop();
});

describe("purchase rehearsal", () => {
  it("A: checkout → signed webhook → worker → REAL Rust signer → stored .vlic", async () => {
    const o = await buy("Rehearsal Public School (FAKE)");
    const logs = await runSigner(async () => (await issuance(o.orderId))[0]?.status === "issued");
    expect(logs).toContain("licence issued");
    await deliverAndExport("purchase", o);
  }, 120_000);

  it("B: signer fails → TS retryFailedIssuance → Rust signer issues the SAME licence_id", async () => {
    const o = await buy("Rehearsal Retry Academy (FAKE)");
    // Fault injection: one allowed attempt, and the signer host loses its INSERT grant on
    // billing.jobs, so its commit transaction (artefact + enqueue delivery) fails and rolls back.
    await db.admin.query(`UPDATE billing.jobs SET max_attempts = 1 WHERE dedupe_key = $1`, [`issue:${o.issuanceId}`]);
    await db.admin.query(`REVOKE INSERT ON billing.jobs FROM vidya_signer`);
    try {
      await runSigner(async () => (await issuance(o.orderId))[0]?.status === "failed");
    } finally {
      await db.admin.query(`GRANT INSERT ON billing.jobs TO vidya_signer`);
    }
    const [failed] = await issuance(o.orderId);
    expect(failed).toMatchObject({ status: "failed", licence_id: o.licenceId, signature: null, vlic: null });
    expect(failed.payload).not.toBeNull(); // payload persisted before signing
    expect(failed.last_error).toMatch(/permission denied/);
    const ord = await db.admin.query(`SELECT status FROM billing.orders WHERE id = $1`, [o.orderId]);
    expect(ord.rows[0].status).toBe("paid");
    expect(fulfilmentState({ orderStatus: "paid", issuanceStatus: "failed", deliveryState: null })).toMatchObject({
      label: "Paid · Licence pending",
      action: "retry_issuance",
    });

    // Owner portal: Retry issuance (the real TS action, as vidya_portal).
    const r = await retryFailedIssuance(db.pool("vidya_portal"), OWNER, { issuanceId: o.issuanceId, reason: "rehearsal: signer grant restored" });
    expect(r).toMatchObject({ issuanceId: o.issuanceId, licenceId: o.licenceId });

    await runSigner(async () => (await issuance(o.orderId))[0]?.status === "issued");
    const done = await deliverAndExport("retry", o);
    expect(done.licence_id).toBe(o.licenceId);
    expect(Buffer.compare(done.payload, failed.payload)).toBe(0); // byte-identical payload
    const all = await issuance(o.orderId);
    expect(all).toHaveLength(1); // one licence for the purchase, never a second generation
    const jobs = await db.admin.query(
      `SELECT dedupe_key, state FROM billing.jobs WHERE kind = 'issue_licence' AND payload->>'issuance_id' = $1 ORDER BY created_at`,
      [o.issuanceId],
    );
    expect(jobs.rows).toEqual([
      { dedupe_key: `issue:${o.issuanceId}`, state: "dead" },
      { dedupe_key: `issue:${o.issuanceId}:retry:1`, state: "done" },
    ]);
  }, 120_000);
});
