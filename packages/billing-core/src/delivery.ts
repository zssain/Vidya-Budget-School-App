// Licence delivery: email the EXISTING signed artefact. This module only READS
// licence_issuances — and the worker's DB role has no UPDATE on that table — so a
// delivery failure or retry can never re-issue or alter a licence.
import { enqueueJob } from "./jobs";
import { type PoolLike, type Queryable } from "./db";

export interface MailMessage {
  from: string;
  to: string;
  subject: string;
  text: string;
  attachments: { filename: string; content: string }[];
  idempotencyKey: string;
}

/** Implemented in the worker with the official `resend` SDK. */
export interface Mailer {
  send(msg: MailMessage): Promise<{ id: string }>;
}

export class RetryableDeliveryError extends Error {
  override name = "RetryableDeliveryError";
}

interface DeliverySource {
  issuance_id: string;
  licence_id: string;
  status: string;
  licence_key: string | null;
  vlic: string | null;
  machine_code: string;
  school_name: string;
  email: string;
  generation: number;
}

async function loadSource(q: Queryable, issuanceId: string): Promise<DeliverySource | null> {
  const r = await q.query<DeliverySource>(
    `SELECT i.id AS issuance_id, i.licence_id, i.status, i.licence_key, i.vlic, i.machine_code,
            i.generation, e.school_name, c.email
       FROM billing.licence_issuances i
       JOIN billing.entitlements e ON e.id = i.entitlement_id
       JOIN billing.orders o ON o.id = e.order_id
       JOIN billing.billing_schools s ON s.id = o.school_id
       JOIN billing.billing_customers c ON c.id = s.customer_id
      WHERE i.id = $1`,
    [issuanceId],
  );
  return r.rows[0] ?? null;
}

/**
 * Group a licence key in blocks of 5 for pasting (the app ignores whitespace).
 * Idempotent: the signer already stores `licence_key` grouped (licence-maker's
 * group5), so re-grouping must reproduce it exactly, not regroup the spaces.
 */
export function groupKey(key: string): string {
  return key.replace(/\s+/g, "").replace(/(.{5})/g, "$1 ").trim();
}

export function licenceEmail(src: DeliverySource): { subject: string; text: string } {
  return {
    subject: `Your Vidya licence for ${src.school_name}`,
    text: [
      `Hello,`,
      ``,
      `Your Vidya licence for ${src.school_name} has been issued.`,
      ``,
      `Licence ID: ${src.licence_id}`,
      `Bound to machine code: ${src.machine_code}`,
      ``,
      `To use it, on the school PC that shows this machine code in Vidya:`,
      `  1. Open Vidya → Welcome → Set up.`,
      `  2. Paste this key into the activation field (or load the attached vidya-licence.vlic file):`,
      ``,
      groupKey(src.licence_key ?? ""),
      ``,
      `Vidya checks the licence offline on that PC. It will not work on a different PC.`,
      `Keep this email: the same licence can be re-imported on that PC at any time.`,
    ].join("\n"),
  };
}

export async function deliverLicence(
  pool: PoolLike,
  mailer: Mailer,
  input: { issuanceId: string; idempotencyKey: string; from: string },
): Promise<{ state: "sent" | "already_sent"; messageId: string | null }> {
  const src = await loadSource(pool, input.issuanceId);
  if (!src) throw new Error("issuance not found");
  if (src.status !== "issued" || !src.licence_key || !src.vlic) {
    throw new RetryableDeliveryError("issuance is not issued yet");
  }
  await pool.query(
    `INSERT INTO billing.deliveries (issuance_id, channel, idempotency_key)
     VALUES ($1, 'email', $2) ON CONFLICT (idempotency_key) DO NOTHING`,
    [src.issuance_id, input.idempotencyKey],
  );
  const d = await pool.query<{ id: string; state: string; provider_message_id: string | null }>(
    `UPDATE billing.deliveries SET attempts = attempts + 1, updated_at = now()
      WHERE idempotency_key = $1 AND state <> 'sent'
      RETURNING id, state, provider_message_id`,
    [input.idempotencyKey],
  );
  if (!d.rows[0]) return { state: "already_sent", messageId: null };

  const mail = licenceEmail(src);
  try {
    const sent = await mailer.send({
      from: input.from,
      to: src.email,
      subject: mail.subject,
      text: mail.text,
      attachments: [{ filename: "vidya-licence.vlic", content: src.vlic }],
      // Provider-side idempotency: a retry after an ambiguous timeout is not re-sent.
      idempotencyKey: input.idempotencyKey,
    });
    await pool.query(
      `UPDATE billing.deliveries SET state = 'sent', provider_message_id = $2, last_error = NULL,
              updated_at = now() WHERE idempotency_key = $1`,
      [input.idempotencyKey, sent.id],
    );
    return { state: "sent", messageId: sent.id };
  } catch (err) {
    const msg = (err instanceof Error ? err.message : String(err)).slice(0, 300);
    await pool.query(
      `UPDATE billing.deliveries SET state = 'failed', last_error = $2, updated_at = now()
        WHERE idempotency_key = $1`,
      [input.idempotencyKey, msg],
    );
    throw new RetryableDeliveryError(`email provider error: ${msg}`);
  }
}

/** Sweep: every issued issuance gets exactly one initial delivery job. */
export async function scheduleInitialDeliveries(pool: PoolLike): Promise<number> {
  const r = await pool.query<{ id: string }>(
    `SELECT i.id FROM billing.licence_issuances i
      WHERE i.status = 'issued'
        AND NOT EXISTS (SELECT 1 FROM billing.jobs j WHERE j.dedupe_key = 'deliver:' || i.id::text)
      LIMIT 200`,
  );
  let n = 0;
  for (const { id } of r.rows) {
    const j = await enqueueJob(pool, {
      kind: "deliver_licence",
      dedupeKey: `deliver:${id}`,
      payload: { issuance_id: id },
    });
    if (j.created) n++;
  }
  return n;
}
