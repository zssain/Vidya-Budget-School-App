// Worker handler for `process_webhook` jobs. The receipt row is locked and its
// processing_state updated in the SAME transaction as the business effect, so a
// crash before COMMIT changes nothing (the job is retried) and a crash after COMMIT
// makes the retry a no-op (receipt is no longer 'pending').
import { audit, withTransaction, type PoolLike, type Queryable } from "./db";
import { enqueueJob } from "./jobs";
import { applyPaymentObservation, type ApplyOutcome } from "./provisioning";
import { eventEntity, normalisePaymentEntity, type PaymentObservation } from "./razorpay-events";
import type { Environment } from "./webhooks";

export interface ProcessContext {
  /** If set, events whose account_id differs are rejected (merchant check). */
  expectedAccountId?: string | null;
  /** Delay before re-checking an authorized-but-not-captured payment. */
  authorizedRecheckSeconds?: number;
}

export type ProcessResult =
  | { state: "done"; outcome?: ApplyOutcome }
  | { state: "ignored"; reason: string }
  | { state: "failed"; reason: string }
  | { state: "already_processed" };

const PAYMENT_EVENTS = new Set(["payment.captured", "order.paid", "payment.authorized", "payment.failed"]);
const REFUND_EVENTS = new Set(["refund.processed", "refund.created", "refund.failed"]);
const DISPUTE_STATE_EVENTS = new Set([
  "payment.dispute.created",
  "payment.dispute.lost",
  "payment.dispute.action_required",
  "payment.dispute.under_review",
]);
const DISPUTE_INFO_EVENTS = new Set(["payment.dispute.won", "payment.dispute.closed"]);

async function handleEvent(
  c: Queryable,
  env: Environment,
  eventType: string,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  event: any,
  ctx: ProcessContext,
): Promise<ProcessResult> {
  const accountOk =
    ctx.expectedAccountId && typeof event?.account_id === "string"
      ? event.account_id === ctx.expectedAccountId
      : ctx.expectedAccountId
        ? false
        : undefined;

  let obs: PaymentObservation | null = normalisePaymentEntity(eventEntity(event, "payment"));

  if (PAYMENT_EVENTS.has(eventType)) {
    if (!obs) return { state: "failed", reason: "malformed payment entity" };
    if ((eventType === "payment.captured" || eventType === "order.paid") && !obs.captureHappened) {
      return { state: "failed", reason: `${eventType} without captured payment` };
    }
  } else if (REFUND_EVENTS.has(eventType)) {
    if (eventType !== "refund.processed") {
      const refund = eventEntity(event, "refund") as { id?: string; amount?: number } | undefined;
      await audit(c, {
        actor: "worker",
        action: eventType === "refund.created" ? "refund_initiated_observed" : "refund_failed_observed",
        target: obs?.providerPaymentId ?? null,
        after: { refund_id: refund?.id ?? null, amount_paise: refund?.amount ?? null },
      });
      return { state: "done" };
    }
    if (!obs) return { state: "failed", reason: "refund without payment entity" };
  } else if (DISPUTE_STATE_EVENTS.has(eventType)) {
    if (!obs) return { state: "failed", reason: "dispute without payment entity" };
    obs = { ...obs, state: "disputed", captureHappened: true };
  } else if (DISPUTE_INFO_EVENTS.has(eventType)) {
    const d = eventEntity(event, "dispute") as { id?: string; status?: string } | undefined;
    await audit(c, {
      actor: "worker",
      action: "review_dispute",
      target: obs?.providerPaymentId ?? null,
      after: { dispute_id: d?.id ?? null, status: d?.status ?? eventType },
      reason: `${eventType} — owner review; commercial status is not changed automatically`,
    });
    return { state: "done" };
  } else {
    return { state: "ignored", reason: `unhandled event ${eventType}` };
  }

  const outcome = await applyPaymentObservation(c, obs, { environment: env, source: "webhook", accountOk });
  if (outcome.kind === "rejected") {
    return outcome.reason === "unknown_order"
      ? { state: "ignored", reason: "unknown_order" }
      : { state: "failed", reason: outcome.reason };
  }
  if (outcome.paymentState === "authorized") {
    // Authorized is NOT paid. Re-check later in case capture happens without a webhook.
    const order = await c.query<{ id: string }>(
      `SELECT id FROM billing.orders WHERE provider_order_id = $1`,
      [obs.providerOrderId],
    );
    if (order.rows[0]) {
      await enqueueJob(c, {
        kind: "reconcile_order",
        dedupeKey: `reconcile:${order.rows[0].id}:authorized:${obs.providerPaymentId}`,
        payload: { order_id: order.rows[0].id },
        delaySeconds: ctx.authorizedRecheckSeconds ?? 900,
      });
    }
  }
  return { state: "done", outcome };
}

export async function processWebhookReceipt(
  pool: PoolLike,
  receiptId: string,
  ctx: ProcessContext = {},
): Promise<ProcessResult> {
  return withTransaction(pool, async (c) => {
    const r = await c.query<{
      environment: Environment;
      event_type: string;
      payload: unknown;
      processing_state: string;
    }>(
      `SELECT environment, event_type, payload, processing_state
         FROM billing.webhook_receipts WHERE id = $1 FOR UPDATE`,
      [receiptId],
    );
    const rec = r.rows[0];
    if (!rec) return { state: "ignored", reason: "receipt not found" } as const;
    if (rec.processing_state !== "pending") return { state: "already_processed" } as const;

    const result = await handleEvent(c, rec.environment, rec.event_type, rec.payload, ctx);
    const dbState = result.state === "already_processed" ? "done" : result.state;
    const err = "reason" in result ? result.reason : null;
    await c.query(
      `UPDATE billing.webhook_receipts SET processing_state = $2, last_error = $3 WHERE id = $1`,
      [receiptId, dbState, err],
    );
    return result;
  });
}
