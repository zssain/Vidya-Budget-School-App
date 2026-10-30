// Razorpay webhook endpoint (configure ONE per environment in the Razorpay
// dashboard; this deployment's RAZORPAY_MODE decides which secret verifies it).
//
// * Reads the exact raw bytes (arrayBuffer) — never req.json() before verifying.
// * HMAC-SHA256 + timing-safe compare (billing-core).
// * Receipt + process_webhook job committed in ONE transaction before we answer
//   200; any database error → 500 so Razorpay redelivers.
// * Duplicate event id → 200 {duplicate:true}; no second fulfilment.
// * No CSRF/origin check: this is a server-to-server call authenticated by HMAC.
import { ingestRazorpayWebhook, MAX_WEBHOOK_BYTES } from "@vidya/billing-core";
import { db } from "@/lib/db";
import { checkoutEnv } from "@/lib/env";
import { json } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";

export async function POST(req: Request) {
  if (!surfaceAllows("checkout")) return json({ error: "not_found" }, 404);
  const env = checkoutEnv();
  const len = Number(req.headers.get("content-length") ?? "0");
  if (len > MAX_WEBHOOK_BYTES) return json({ error: "payload too large" }, 413);
  const raw = new Uint8Array(await req.arrayBuffer());
  const res = await ingestRazorpayWebhook(db(), {
    rawBody: raw,
    signature: req.headers.get("x-razorpay-signature"),
    eventIdHeader: req.headers.get("x-razorpay-event-id"),
    secret: env.RAZORPAY_WEBHOOK_SECRET,
    environment: env.RAZORPAY_MODE,
  });
  if (!res.ok) return json({ error: res.error }, res.status);
  return json({ ok: true, duplicate: res.duplicate });
}
