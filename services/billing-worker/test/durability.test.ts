// Concurrency, crash recovery, lease recovery, dead-letter, reconciliation,
// delivery/email outage and customer capability isolation.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  claimJob,
  completeMachineCodeByToken,
  findOrderByStatusToken,
  processWebhookReceipt,
  recordCheckoutReturn,
  requestResend,
  signCheckout,
  transferLicence,
  reissueFailedIssuance,
  withTransaction,
  ensureEntitlement,
  lockOrder,
} from "@vidya/billing-core";
import { runOnce, sweep } from "../src/worker";
import {
  counts,
  deps,
  drain,
  eventBody,
  fakeSigner,
  issuanceFor,
  MC_A,
  MC_B,
  newOrder,
  orderRow,
  paymentEntity,
  postWebhook,
  setup,
  type Ctx,
} from "./helpers";

let ctx: Ctx;
beforeAll(async () => {
  ctx = await setup();
});
afterAll(async () => {
  await ctx?.db.drop();
});

describe("concurrent workers converge", () => {
  it("8 workers × (captured, order.paid, duplicate, reconcile) → ONE entitlement, ONE issuance, ONE issue job", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    for (let i = 0; i < 3; i++) await postWebhook(ctx, eventBody("payment.captured", p));
    for (let i = 0; i < 3; i++) await postWebhook(ctx, eventBody("order.paid", p));
    const ds = Array.from({ length: 8 }, () => deps(ctx));
    for (const d of ds) d.razorpay.payments.set(o.providerOrderId, [p]);
    // Also reconcile concurrently.
    await recordCheckoutReturn(ctx.portal, {
      statusToken: o.statusToken, paymentId: p.id, signature: signCheckout(o.providerOrderId, p.id, "ks"), keySecret: "ks",
    });
    await Promise.all(ds.map((d) => drain(d)));
    expect(await counts(ctx, o.orderId)).toEqual({ entitlements: 1, issuances: 1, issueJobs: 1, payments: 1 });
    const jobs = await ctx.admin.query(`SELECT state, count(*)::int AS n FROM billing.jobs WHERE kind = 'process_webhook' GROUP BY state`);
    expect(jobs.rows.every((r) => r.state === "done")).toBe(true);
  });

  it("concurrent direct provisioning calls on fresh connections serialise on the order row", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    const receipts: string[] = [];
    for (let i = 0; i < 10; i++) {
      const r = await postWebhook(ctx, eventBody("payment.captured", p));
      if (r.ok && r.receiptId) receipts.push(r.receiptId);
    }
    await Promise.all(receipts.map((id) => processWebhookReceipt(ctx.worker, id, { expectedAccountId: "acc_TESTFIXTURE" })));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1 });
  });
});

describe("crash safety", () => {
  it("crash BEFORE commit: nothing persists, the retried job provisions exactly once", async () => {
    const o = await newOrder(ctx);
    const r = await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    const receiptId = (r as { receiptId: string }).receiptId;
    // Simulate the process dying mid-transaction: do the work, then throw before COMMIT.
    await expect(
      withTransaction(ctx.worker, async (c) => {
        const order = await lockOrder(c, { id: o.orderId });
        await c.query(`UPDATE billing.orders SET status = 'paid' WHERE id = $1`, [o.orderId]);
        await ensureEntitlement(c, { ...order!, status: "paid" }, "verified_payment", "worker");
        throw new Error("simulated crash before commit");
      }),
    ).rejects.toThrow(/simulated crash/);
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 0, issuances: 0 });
    const st = await ctx.admin.query(`SELECT processing_state FROM billing.webhook_receipts WHERE id = $1`, [receiptId]);
    expect(st.rows[0].processing_state).toBe("pending");
    await drain(deps(ctx));
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1 });
  });

  it("crash AFTER commit but before job completion: lease expires, job reclaimed, re-run is a no-op", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    const d1 = deps(ctx, { leaseSeconds: 1 });
    const job = await claimJob(ctx.worker, { workerId: d1.workerId, kinds: ["process_webhook"], leaseSeconds: 1 });
    expect(job).not.toBeNull();
    await processWebhookReceipt(ctx.worker, job!.payload.receipt_id, { expectedAccountId: "acc_TESTFIXTURE" }); // committed
    // …worker dies here: completeJob never runs.
    await ctx.admin.query(`UPDATE billing.jobs SET lease_until = now() - interval '1 second' WHERE id = $1`, [job!.id]);
    const d2 = deps(ctx);
    expect(await runOnce(d2)).toBe(true); // reclaims the abandoned job
    const j = await ctx.admin.query(`SELECT state, attempts, locked_by FROM billing.jobs WHERE id = $1`, [job!.id]);
    expect(j.rows[0]).toMatchObject({ state: "done", attempts: 2 });
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1 });
  });

  it("an exhausted, abandoned lease is dead-lettered (visible), not retried forever", async () => {
    await ctx.admin.query(
      `INSERT INTO billing.jobs (kind, dedupe_key, payload, state, attempts, max_attempts, locked_by, lease_until)
       VALUES ('reconcile_order', 'test:exhausted', '{"order_id":"00000000-0000-4000-8000-000000000000"}', 'running', 3, 3, 'dead-worker', now() - interval '1 minute')`,
    );
    await sweep(deps(ctx));
    const j = await ctx.admin.query(`SELECT state, last_error FROM billing.jobs WHERE dedupe_key = 'test:exhausted'`);
    expect(j.rows[0].state).toBe("dead");
    expect(j.rows[0].last_error).toMatch(/lease expired/);
  });

  it("a malformed job payload dead-letters immediately (permanent error)", async () => {
    await ctx.admin.query(`INSERT INTO billing.jobs (kind, dedupe_key, payload) VALUES ('deliver_licence', 'test:bad-payload', '{"issuance_id":"nope"}')`);
    await drain(deps(ctx));
    const j = await ctx.admin.query(`SELECT state, attempts FROM billing.jobs WHERE dedupe_key = 'test:bad-payload'`);
    expect(j.rows[0]).toEqual({ state: "dead", attempts: 1 });
  });
});

describe("missed-webhook reconciliation (mocked Razorpay client)", () => {
  it("no webhook ever arrives → sweep reconciles the order and provisions once", async () => {
    const o = await newOrder(ctx);
    await ctx.admin.query(`UPDATE billing.orders SET created_at = now() - interval '30 minutes' WHERE id = $1`, [o.orderId]);
    const d = deps(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    d.razorpay.payments.set(o.providerOrderId, [p]);
    await sweep(d);
    await drain(d);
    expect((await orderRow(ctx, o.orderId)).status).toBe("paid");
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1, issueJobs: 1, payments: 1 });
    // a later real webhook for the same capture is a no-op
    await postWebhook(ctx, eventBody("payment.captured", p));
    await drain(d);
    expect(await counts(ctx, o.orderId)).toMatchObject({ entitlements: 1, issuances: 1 });
  });

  it("Razorpay API outage → job retried with backoff, order NOT marked paid", async () => {
    const o = await newOrder(ctx);
    const d = deps(ctx);
    d.razorpay.outage = true;
    await recordCheckoutReturn(ctx.portal, {
      statusToken: o.statusToken, paymentId: "pay_Toutage1", signature: signCheckout(o.providerOrderId, "pay_Toutage1", "ks"), keySecret: "ks",
    });
    await drain(d);
    const j = await ctx.admin.query(`SELECT state, attempts, last_error, available_at > now() AS later FROM billing.jobs WHERE dedupe_key = $1`, [`reconcile:${o.orderId}:checkout:pay_Toutage1`]);
    expect(j.rows[0]).toMatchObject({ state: "ready", attempts: 1, later: true });
    expect(j.rows[0].last_error).toMatch(/outage/);
    expect((await orderRow(ctx, o.orderId)).status).toBe("pending");
    // provider recovers
    d.razorpay.outage = false;
    d.razorpay.payments.set(o.providerOrderId, [paymentEntity({ id: "pay_Toutage1", orderId: o.providerOrderId, amount: o.amountPaise })]);
    await ctx.admin.query(`UPDATE billing.jobs SET available_at = now() WHERE dedupe_key = $1`, [`reconcile:${o.orderId}:checkout:pay_Toutage1`]);
    await drain(d);
    expect((await orderRow(ctx, o.orderId)).status).toBe("paid");
  });

  it("browser checkout return: forged signature is refused; repeated valid callback enqueues ONE job", async () => {
    const o = await newOrder(ctx);
    expect(await recordCheckoutReturn(ctx.portal, { statusToken: o.statusToken, paymentId: "pay_Tx", signature: "00".repeat(32), keySecret: "ks" })).toBe("invalid");
    const sig = signCheckout(o.providerOrderId, "pay_Tx", "ks");
    for (let i = 0; i < 3; i++) {
      expect(await recordCheckoutReturn(ctx.portal, { statusToken: o.statusToken, paymentId: "pay_Tx", signature: sig, keySecret: "ks" })).toBe("verifying");
    }
    const j = await ctx.admin.query(`SELECT count(*)::int AS n FROM billing.jobs WHERE kind = 'reconcile_order' AND payload->>'order_id' = $1`, [o.orderId]);
    expect(j.rows[0].n).toBe(1);
    expect((await orderRow(ctx, o.orderId)).status).toBe("pending"); // browser success is not payment
  });
});

describe("signer outage + delivery", () => {
  it("signer down → order stays paid with a pending issuance; nothing is delivered", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    const d = deps(ctx);
    await drain(d);
    await sweep(d);
    await drain(d);
    expect((await orderRow(ctx, o.orderId)).status).toBe("paid");
    expect((await issuanceFor(ctx, o.orderId))[0].status).toBe("pending");
    expect(d.mailer.sent.length).toBe(0);
  });

  it("email outage → delivery retried, artefact unchanged, no re-issue; then sent once", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    const d = deps(ctx);
    await drain(d);
    const [iss] = await issuanceFor(ctx, o.orderId);
    await fakeSigner(ctx, iss.id);
    const before = (await issuanceFor(ctx, o.orderId))[0];
    d.mailer.failuresLeft = 2;
    await sweep(d); // schedules deliver:<issuance_id>
    for (let i = 0; i < 3; i++) {
      await drain(d);
      await ctx.admin.query(`UPDATE billing.jobs SET available_at = now() WHERE dedupe_key = $1 AND state = 'ready'`, [`deliver:${iss.id}`]);
    }
    const del = await ctx.admin.query(`SELECT state, attempts, provider_message_id FROM billing.deliveries WHERE issuance_id = $1`, [iss.id]);
    expect(del.rows).toHaveLength(1);
    expect(del.rows[0]).toMatchObject({ state: "sent", attempts: 3, provider_message_id: "email_1" });
    expect(d.mailer.sent).toHaveLength(1);
    expect(d.mailer.sent[0]!.attachments[0]!.content).toBe(before.vlic);
    expect(d.mailer.sent[0]!.idempotencyKey).toBe(`deliver:${iss.id}`);
    const after = await issuanceFor(ctx, o.orderId);
    expect(after).toHaveLength(1);
    expect(after[0].licence_key).toBe(before.licence_key);
    expect(await counts(ctx, o.orderId)).toMatchObject({ issuances: 1, issueJobs: 1 });
    // A second sweep never schedules a second initial delivery.
    await sweep(d);
    await drain(d);
    expect(d.mailer.sent).toHaveLength(1);
  });

  it("owner resend sends the SAME existing artefact again and never creates an issuance", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    const d = deps(ctx);
    await drain(d);
    const [iss] = await issuanceFor(ctx, o.orderId);
    await expect(requestResend(ctx.portal, { ownerId: "owner" }, { issuanceId: iss.id, reason: "customer lost email" })).rejects.toThrow(/not been issued/);
    await fakeSigner(ctx, iss.id);
    await requestResend(ctx.portal, { ownerId: "owner" }, { issuanceId: iss.id, reason: "customer lost email" });
    await drain(d);
    expect(d.mailer.sent).toHaveLength(1);
    expect(d.mailer.sent[0]!.idempotencyKey).toMatch(/^resend:/);
    expect(d.mailer.sent[0]!.text).toContain(iss.licence_id);
    expect(await counts(ctx, o.orderId)).toMatchObject({ issuances: 1, issueJobs: 1 });
  });
});

describe("transfer + re-issue (new generation, new licence_id)", () => {
  it("transfer creates generation 2 bound to the new machine, linked, audited; old one untouched", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    await drain(deps(ctx));
    const [g1] = await issuanceFor(ctx, o.orderId);
    await fakeSigner(ctx, g1.id);
    await expect(transferLicence(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, newMachineCode: MC_A, reason: "same machine" })).rejects.toThrow(/equals/);
    const t = await transferLicence(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, newMachineCode: MC_B, reason: "school replaced its PC" });
    const all = await issuanceFor(ctx, o.orderId);
    expect(all.map((i) => [i.generation, i.machine_code, i.status])).toEqual([[1, MC_A, "issued"], [2, MC_B, "pending"]]);
    expect(all[1].licence_id).not.toBe(all[0].licence_id);
    const tr = await ctx.admin.query(`SELECT source_issuance_id, replacement_issuance_id, target_machine_code FROM billing.licence_transfers WHERE id = $1`, [t.transferId]);
    expect(tr.rows[0]).toEqual({ source_issuance_id: g1.id, replacement_issuance_id: t.issuanceId, target_machine_code: MC_B });
    const job = await ctx.admin.query(`SELECT payload FROM billing.jobs WHERE dedupe_key = $1`, [`issue:${t.issuanceId}`]);
    expect(job.rows[0].payload).toEqual({ issuance_id: t.issuanceId });
    // transferring the superseded generation again is refused
    await expect(transferLicence(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, newMachineCode: MC_B, reason: "again please" })).rejects.toThrow(/latest/);
  });

  it("transfer is blocked once the purchase is refunded", async () => {
    const o = await newOrder(ctx);
    const p = paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise });
    await postWebhook(ctx, eventBody("payment.captured", p));
    await drain(deps(ctx));
    const [g1] = await issuanceFor(ctx, o.orderId);
    await fakeSigner(ctx, g1.id);
    await postWebhook(ctx, eventBody("refund.processed", { ...p, status: "refunded", amount_refunded: o.amountPaise, refund_status: "full" }));
    await drain(deps(ctx));
    await expect(transferLicence(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, newMachineCode: MC_B, reason: "after refund" })).rejects.toThrow(/blocked/);
  });

  it("a signer-failed issuance can be re-issued as a new generation (same machine)", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    await drain(deps(ctx));
    const [g1] = await issuanceFor(ctx, o.orderId);
    await expect(reissueFailedIssuance(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, reason: "not failed yet" })).rejects.toThrow(/only a failed/);
    await ctx.signer.query(`UPDATE billing.licence_issuances SET status = 'failed', last_error = 'test' WHERE id = $1`, [g1.id]);
    const r = await reissueFailedIssuance(ctx.portal, { ownerId: "owner" }, { issuanceId: g1.id, reason: "signer key fixed" });
    const all = await issuanceFor(ctx, o.orderId);
    expect(all.map((i) => [i.generation, i.status, i.machine_code])).toEqual([[1, "failed", MC_A], [2, "pending", MC_A]]);
    expect(r.licenceId).not.toBe(g1.licence_id);
  });
});

describe("customer status capability", () => {
  it("token A sees only order A; B's token, a guessed token, an order id or an expired token see nothing", async () => {
    const a = await newOrder(ctx, { school: "Fake School A" });
    const b = await newOrder(ctx, { school: "Fake School B" });
    const va = await findOrderByStatusToken(ctx.portal, a.statusToken);
    const vb = await findOrderByStatusToken(ctx.portal, b.statusToken);
    expect(va?.orderId).toBe(a.orderId);
    expect(va?.schoolName).toBe("Fake School A");
    expect(vb?.orderId).toBe(b.orderId);
    expect(await findOrderByStatusToken(ctx.portal, a.orderId)).toBeNull();
    expect(await findOrderByStatusToken(ctx.portal, "A".repeat(43))).toBeNull();
    // the raw token is not stored anywhere
    const raw = await ctx.admin.query(`SELECT count(*)::int AS n FROM billing.orders WHERE status_token_hash = $1`, [a.statusToken]);
    expect(raw.rows[0].n).toBe(0);
    await ctx.admin.query(`UPDATE billing.orders SET status_token_expires_at = now() - interval '1 second' WHERE id = $1`, [a.orderId]);
    expect(await findOrderByStatusToken(ctx.portal, a.statusToken)).toBeNull();
    // B's capability cannot act on A
    await expect(completeMachineCodeByToken(ctx.portal, b.statusToken, MC_B)).rejects.toThrow(/not awaiting/);
  });

  it("licence artefact is withheld until issued, then shown", async () => {
    const o = await newOrder(ctx);
    await postWebhook(ctx, eventBody("payment.captured", paymentEntity({ orderId: o.providerOrderId, amount: o.amountPaise })));
    await drain(deps(ctx));
    let v = await findOrderByStatusToken(ctx.portal, o.statusToken);
    expect(v?.status).toBe("paid");
    expect(v?.licence?.status).toBe("pending");
    expect(v?.licence?.licenceKey).toBeNull();
    await fakeSigner(ctx, (await issuanceFor(ctx, o.orderId))[0].id);
    v = await findOrderByStatusToken(ctx.portal, o.statusToken);
    expect(v?.licence?.licenceKey).toMatch(/^TESTKEY/);
    expect(v?.licence?.vlic).toMatch(/^VLIC-VID-/);
  });
});
