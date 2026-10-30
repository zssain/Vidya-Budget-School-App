// Webhook ingestion (called by the portal's /api/webhooks/razorpay route).
//
// 1. Verify HMAC over the EXACT raw bytes (no parse/re-serialise first).
// 2. Only then parse JSON.
// 3. In ONE transaction: insert the unique receipt (provider+environment+event_id)
//    and its process_webhook job. Duplicate delivery → ON CONFLICT no-op → the
//    caller returns 200 without re-fulfilment.
// The HTTP 200 is sent only after COMMIT, so Razorpay retries anything we did not
// durably store. Fulfilment happens in the worker, never in the request.
import { withTransaction, type PoolLike } from "./db";
import { enqueueJob } from "./jobs";
import { verifyWebhookSignature } from "./razorpay-signature";
import { sha256Hex } from "./tokens";

export type Environment = "test" | "live";

export const MAX_WEBHOOK_BYTES = 256 * 1024;

export type IngestResult =
  | { ok: true; duplicate: boolean; receiptId: string | null }
  | { ok: false; status: 400 | 401 | 413; error: string };

export async function ingestRazorpayWebhook(
  pool: PoolLike,
  input: {
    rawBody: Uint8Array;
    signature: string | null;
    eventIdHeader: string | null;
    secret: string;
    environment: Environment;
  },
): Promise<IngestResult> {
  if (input.rawBody.byteLength > MAX_WEBHOOK_BYTES) {
    return { ok: false, status: 413, error: "payload too large" };
  }
  if (!verifyWebhookSignature(input.rawBody, input.signature, input.secret)) {
    return { ok: false, status: 401, error: "invalid signature" };
  }
  let event: unknown;
  try {
    event = JSON.parse(Buffer.from(input.rawBody).toString("utf8"));
  } catch {
    return { ok: false, status: 400, error: "malformed json" };
  }
  if (
    typeof event !== "object" ||
    event === null ||
    typeof (event as { event?: unknown }).event !== "string"
  ) {
    return { ok: false, status: 400, error: "not a razorpay event" };
  }
  const eventType = (event as { event: string }).event.slice(0, 100);
  const payloadSha = sha256Hex(input.rawBody);
  const headerId = input.eventIdHeader?.trim();
  // Razorpay sends X-Razorpay-Event-Id; if absent, the body digest is a stable id
  // for exact redeliveries.
  const eventId =
    headerId && /^[A-Za-z0-9_.:-]{1,128}$/.test(headerId) ? headerId : `sha256:${payloadSha}`;

  return withTransaction(pool, async (c) => {
    const r = await c.query<{ id: string }>(
      `INSERT INTO billing.webhook_receipts
         (provider, environment, event_id, event_type, payload_sha256, payload, verified_at)
       VALUES ('razorpay', $1, $2, $3, $4, $5::jsonb, now())
       ON CONFLICT (provider, environment, event_id) DO NOTHING
       RETURNING id`,
      [input.environment, eventId, eventType, payloadSha, JSON.stringify(event)],
    );
    const receiptId = r.rows[0]?.id;
    if (!receiptId) return { ok: true as const, duplicate: true, receiptId: null };
    await enqueueJob(c, {
      kind: "process_webhook",
      dedupeKey: `webhook:${receiptId}`,
      payload: { receipt_id: receiptId },
    });
    return { ok: true as const, duplicate: false, receiptId };
  });
}

/** Bounded retention: drop raw payloads of settled receipts; keep id/hash for dedupe. */
export async function purgeWebhookPayloads(pool: PoolLike, retentionDays: number): Promise<number> {
  const r = await pool.query(
    `UPDATE billing.webhook_receipts
        SET payload = '{"purged": true}'::jsonb
      WHERE processing_state IN ('done', 'ignored', 'failed')
        AND verified_at < now() - make_interval(days => $1)
        AND payload <> '{"purged": true}'::jsonb`,
    [retentionDays],
  );
  return r.rowCount ?? 0;
}
