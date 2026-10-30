// Webhook ingestion + verified-payment provisioning against real Postgres 16,
// using the real least-privilege roles (vidya_worker / vidya_portal / vidya_signer).
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createManualOrder, approveManualPayment, completeMachineCode, signWebhookBody } from "@vidya/billing-core";
import {
  counts,
  deps,
  drain,
  eventBody,
  issuanceFor,
  MC_A,
  newOrder,
  orderRow,
  paymentEntity,
  postWebhook,
  setup,
  WEBHOOK_SECRET,
  type Ctx,
} from "./helpers";

let ctx: Ctx;
beforeAll(async () => {
  ctx = await setup();
});
afterAll(async () => {
  await ctx?.db.drop();
});

async function receiptState(eventId: string) {
  const r = await ctx.admin.query(`SELECT processing_state, last_error FROM billing.webhook_receipts WHERE event_id = $1`, [eventId]);
  return r.rows[0];
}

describe("webhook signature (raw body, HMAC-SHA256, timing-safe)", () => {
  it("valid signature → stored receipt + one process_webhook job", async () => {
    const o = await newOrder(ctx);
    const res = await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })), { eventId: "evt_valid_1" });
    expect(res).toMatchObject({ ok: true, duplicate: false });
    const j = await ctx.admin.query(`SELECT count(*)::int AS n FROM billing.jobs WHERE dedupe_key = $1`, [`webhook:${(res as { receiptId: string }).receiptId}`]);
    expect(j.rows[0].n).toBe(1);
  });

  it("forged signature → 401, nothing stored", async () => {
    const body = eventBody("payment.captured", paymentEntity({ orderId: "order_x", amount: 1 }));
    const res = await postWebhook(ctx, body, { eventId: "evt_forged", signature: signWebhookBody(body, "attacker-secret") });
    expect(res).toEqual({ ok: false, status: 401, error: "invalid signature" });
    expect(await receiptState("evt_forged")).toBeUndefined();
  });

  it("re-serialised body with the original signature → 401", async () => {
    const body = eventBody("payment.captured", paymentEntity({ orderId: "order_x", amount: 1 }));
    const sig = signWebhookBody(body, WEBHOOK_SECRET);
    const reserialised = JSON.stringify(JSON.parse(body), null, 2);
    const res = await postWebhook(ctx, reserialised, { eventId: "evt_reser", signature: sig });
    expect(res).toMatchObject({ ok: false, status: 401 });
    expect(await receiptState("evt_reser")).toBeUndefined();
  });

  it("validly signed but malformed JSON → 400, nothing stored", async () => {
    const res = await postWebhook(ctx, "{not json", { eventId: "evt_bad_json" });
    expect(res).toMatchObject({ ok: false, status: 400 });
  });

  it("duplicate delivery → safe success, one receipt, no second fulfilment", async () => {
    const o = await newOrder(ctx);
    const body = eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise }));
    const a = await postWebhook(ctx, body, { eventId: "evt_dup" });
    const b = await postWebhook(ctx, body, { eventId: "evt_dup" });
    expect(a).toMatchObject({ ok: true, duplicate: false });
    expect(b).toMatchObject({ ok: true, duplicate: true });
    await drain(deps(ctx));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1, payments: 1 });
  });
});

describe("verified capture → exactly one entitlement + issuance + issue job (signer contract)", () => {
  it("happy path matches the signer contract", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    await drain(deps(ctx));
    expect((await orderRow(ctx, o.orderId)).status).toBe("paid");
    const [iss] = await issuanceFor(ctx, o.orderId);
    expect(iss.status).toBe("pending");
    expect(iss.generation).toBe(1);
    expect(iss.licence_id).toMatch(/^VID-[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(iss.machine_code).toBe(MC_A);
    expect(iss.issued_at).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
    const job = await ctx.admin.query(`SELECT kind, payload, state FROM billing.jobs WHERE dedupe_key = $1`, [`issue:${iss.id}`]);
    expect(job.rows[0]).toEqual({ kind: "issue_licence", payload: { issuance_id: iss.id }, state: "ready" });
    const ent = await ctx.admin.query(`SELECT * FROM billing.entitlements WHERE order_id = $1`, [o.orderId]);
    expect(ent.rows[0]).toMatchObject({ source: "verified_payment", machine_code: MC_A, school_name: "Fake Test School", modules: ["core"], commercial_status: "active" });
  });

  it("payment.captured AND order.paid for the same capture converge on one entitlement", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await postWebhook(ctx, eventBody("order.paid", p));
    await drain(deps(ctx));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1, payments: 1 });
  });

  it.each([
    ["wrong amount", (o: { amountPaise: number; providerOrderId: string }) => paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise - 1 }), "amount_mismatch"],
    ["wrong currency", (o: { amountPaise: number; providerOrderId: string }) => paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise, currency: "USD" }), "currency_mismatch"],
  ])("%s → no payment, no entitlement, receipt failed + owner review flag", async (_n, mk, reason) => {
    const o = await newOrder(ctx);
    const p = mk(o);
    const r = await postWebhook(ctx, eventBody("payment.captured", p), { eventId: `evt_${reason}` });
    expect(r.ok).toBe(true);
    await drain(deps(ctx));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 0, payments: 0 });
    expect(await receiptState(`evt_${reason}`)).toEqual({ processing_state: "failed", last_error: reason });
    expect((await orderRow(ctx, o.orderId)).status).toBe("pending");
    const flag = await ctx.admin.query(`SELECT reason FROM billing.owner_audit WHERE action = 'review_payment_rejected' AND target = $1`, [p.id]);
    expect(flag.rows[0].reason).toBe(reason);
  });

  it("unknown provider order → ignored, nothing created", async () => {
    const r = await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: "order_NOTOURS", amount: 100 })), { eventId: "evt_unknown" });
    expect(r.ok).toBe(true);
    await drain(deps(ctx));
    expect(await receiptState("evt_unknown")).toEqual({ processing_state: "ignored", last_error: "unknown_order" });
  });

  it("test-mode event against a live order → rejected (environment_mismatch)", async () => {
    const o = await newOrder(ctx, { environment: "live" });
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })), { eventId: "evt_env", environment: "test" });
    await drain(deps(ctx));
    expect(await receiptState("evt_env")).toEqual({ processing_state: "failed", last_error: "environment_mismatch" });
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 0, payments: 0 });
  });

  it("event from another merchant account → rejected", async () => {
    const o = await newOrder(ctx);
    const body = eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })).replace("acc_TESTFIXTURE", "acc_OTHER");
    await postWebhook(ctx, body, { eventId: "evt_acct" });
    await drain(deps(ctx));
    expect(await receiptState("evt_acct")).toEqual({ processing_state: "failed", last_error: "merchant_account_mismatch" });
    expect((await counts(ctx, o.orderId)).entitlements).toBe(0);
  });

  it("authorized without capture → payment recorded as authorized, NO entitlement, delayed reconcile queued", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.authorized", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise, status: "authorized" })));
    await drain(deps(ctx));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 0, payments: 1 });
    expect((await orderRow(ctx, o.orderId)).status).toBe("pending");
    const pay = await ctx.admin.query(`SELECT state, captured_at FROM billing.payments WHERE order_id = $1`, [o.orderId]);
    expect(pay.rows[0]).toEqual({ state: "authorized", captured_at: null });
    const rj = await ctx.admin.query(`SELECT available_at > now() + interval '10 minutes' AS later FROM billing.jobs WHERE kind = 'reconcile_order' AND payload->>'order_id' = $1`, [o.orderId]);
    expect(rj.rows[0].later).toBe(true);
  });

  it("payment.captured whose entity is not captured → failed, no entitlement", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise, status: "authorized" })), { eventId: "evt_lie" });
    await drain(deps(ctx));
    expect((await receiptState("evt_lie")).processing_state).toBe("failed");
    expect((await counts(ctx, o.orderId)).entitlements).toBe(0);
  });

  it("failed then captured (customer retried) → paid; capture never downgraded by a late 'failed'", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await postWebhook(ctx, eventBody("payment.failed", { ...p, status: "failed", captured: false }));
    await drain(deps(ctx));
    const pay = await ctx.admin.query(`SELECT state FROM billing.payments WHERE provider_payment_id = $1`, [p.id]);
    expect(pay.rows[0].state).toBe("captured");
    expect((await counts(ctx, o.orderId)).entitlements).toBe(1);
  });
});

describe("refunds and disputes", () => {
  it("out of order: full refund arrives BEFORE capture → refunded, no entitlement, review flag", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    const refunded = { ...p, status: "refunded", amount_refunded: o.amountPaise, refund_status: "full" };
    await postWebhook(ctx, eventBody("refund.processed", refunded, { refund: { id: "rfnd_T1", amount: o.amountPaise, payment_id: p.id } }));
    await drain(deps(ctx));
    await postWebhook(ctx, eventBody("payment.captured", p));
    await drain(deps(ctx));
    expect((await orderRow(ctx, o.orderId)).status).toBe("refunded");
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 0, payments: 1 });
    const pay = await ctx.admin.query(`SELECT state, refunded_paise::int FROM billing.payments WHERE provider_payment_id = $1`, [p.id]);
    expect(pay.rows[0]).toEqual({ state: "refunded", refunded_paise: o.amountPaise });
    const f = await ctx.admin.query(`SELECT count(*)::int AS n FROM billing.owner_audit WHERE action = 'review_refund' AND target = $1`, [p.id]);
    expect(f.rows[0].n).toBe(1);
  });

  it("in order: paid → full refund → entitlement kept, marked refunded, never 'disabled'", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await drain(deps(ctx));
    await postWebhook(ctx, eventBody("refund.processed", { ...p, status: "refunded", amount_refunded: o.amountPaise, refund_status: "full" }));
    await drain(deps(ctx));
    expect((await orderRow(ctx, o.orderId)).status).toBe("refunded");
    const ent = await ctx.admin.query(`SELECT commercial_status FROM billing.entitlements WHERE order_id = $1`, [o.orderId]);
    expect(ent.rows[0].commercial_status).toBe("refunded");
    expect((await issuanceFor(ctx, o.orderId)).length).toBe(1); // nothing deleted / re-issued
    const f = await ctx.admin.query(`SELECT reason FROM billing.owner_audit WHERE action = 'review_refund' AND target = $1`, [p.id]);
    expect(f.rows[0].reason).toMatch(/cannot be remotely disabled/);
  });

  it("partial refund → partially_refunded, entitlement stays active, flagged", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await postWebhook(ctx, eventBody("refund.processed", { ...p, status: "captured", amount_refunded: 100, refund_status: "partial" }));
    await drain(deps(ctx));
    expect((await orderRow(ctx, o.orderId)).status).toBe("partially_refunded");
    const ent = await ctx.admin.query(`SELECT commercial_status FROM billing.entitlements WHERE order_id = $1`, [o.orderId]);
    expect(ent.rows[0].commercial_status).toBe("active");
  });

  it("dispute → disputed + review flag", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await postWebhook(ctx, eventBody("payment.dispute.created", p, { dispute: { id: "disp_T1", status: "open" } }));
    await drain(deps(ctx));
    expect((await orderRow(ctx, o.orderId)).status).toBe("disputed");
    const ent = await ctx.admin.query(`SELECT commercial_status FROM billing.entitlements WHERE order_id = $1`, [o.orderId]);
    expect(ent.rows[0].commercial_status).toBe("disputed");
  });
});

describe("paid awaiting machine code + manual approval (owner_manual)", () => {
  it("manual order without code → approve → paid_awaiting_machine_code → completion issues once", async () => {
    const owner = { ownerId: "00000000-0000-4000-8000-00000000000a", requestId: "req-1" };
    const id = await createManualOrder(ctx.portal, owner, {
      productId: ctx.product.id, customerName: "Imported Buyer", email: "imported@example.test", phone: null,
      schoolName: "Imported Fake School", machineCode: null, environment: "test", reason: "bank transfer, imported",
    });
    const none = await approveManualPayment(ctx.portal, owner, { orderId: id, reference: "UTR-TEST-1", reason: "verified in bank statement" });
    expect(none).toBeNull();
    expect((await orderRow(ctx, id)).status).toBe("paid_awaiting_machine_code");
    expect((await counts(ctx, id)).entitlements).toBe(0); // never an unbound licence
    await expect(completeMachineCode(ctx.portal, { orderId: id, machineCode: "BAD-CODE", actor: owner.ownerId, reason: "x" })).rejects.toThrow(/not valid/);
    const ref = await completeMachineCode(ctx.portal, { orderId: id, machineCode: MC_A.toLowerCase(), actor: owner.ownerId, reason: "customer emailed code" });
    expect(ref?.licenceId).toMatch(/^VID-/);
    await expect(completeMachineCode(ctx.portal, { orderId: id, machineCode: MC_A, actor: owner.ownerId, reason: "again" })).rejects.toThrow(/not awaiting/);
    expect(await counts(ctx, id)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1, payments: 0 });
    const ent = await ctx.admin.query(`SELECT source FROM billing.entitlements WHERE order_id = $1`, [id]);
    expect(ent.rows[0].source).toBe("owner_manual");
    const a = await ctx.admin.query(`SELECT after_json FROM billing.owner_audit WHERE action = 'manual_payment_approved' AND target = $1`, [id]);
    expect(a.rows[0].after_json.label).toMatch(/MANUAL APPROVAL/);
  });

  it("manual approval is refused for Razorpay orders (no bypass of verification)", async () => {
    const o = await newOrder(ctx);
    await expect(
      approveManualPayment(ctx.portal, { ownerId: "00000000-0000-4000-8000-00000000000a" }, { orderId: o.orderId, reference: "x", reason: "trust me please" }),
    ).rejects.toThrow(/only manual orders/);
  });
});

describe("database roles", () => {
  it("owner_audit is append-only even for the worker role", async () => {
    await expect(ctx.worker.query(`UPDATE billing.owner_audit SET reason = 'x'`)).rejects.toThrow();
    await expect(ctx.worker.query(`DELETE FROM billing.owner_audit`)).rejects.toThrow();
  });
  it("worker cannot modify licence artefacts (delivery can never re-issue)", async () => {
    await expect(ctx.worker.query(`UPDATE billing.licence_issuances SET status = 'pending'`)).rejects.toThrow(/permission denied/);
  });
  it("signer cannot create payments or entitlements", async () => {
    await expect(ctx.signer.query(`INSERT INTO billing.payments (order_id, provider_payment_id, amount_paise, currency, state) VALUES (gen_random_uuid(), 'pay_x', 1, 'INR', 'captured')`)).rejects.toThrow(/permission denied/);
    await expect(ctx.signer.query(`INSERT INTO billing.entitlements (order_id, modules, machine_code, school_name, source) VALUES (gen_random_uuid(), '{core}', 'x', 'x', 'verified_payment')`)).rejects.toThrow(/permission denied/);
  });
  it("portal cannot write payments (only verified webhooks/reconciliation do)", async () => {
    await expect(ctx.portal.query(`UPDATE billing.payments SET state = 'captured'`)).rejects.toThrow(/permission denied/);
  });
});
