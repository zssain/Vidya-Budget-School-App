// Razorpay signature verification with Node crypto + timing-safe comparison.
//
// * Webhooks: X-Razorpay-Signature = hex(HMAC-SHA256(webhook_secret, RAW body)).
//   We verify over the exact received bytes; the body is never parsed or
//   re-serialised before this check (Razorpay docs: "Validate Webhooks").
// * Checkout: razorpay_signature = hex(HMAC-SHA256(key_secret,
//   `${order_id}|${razorpay_payment_id}`)) where order_id is the provider order WE
//   stored — never the one the browser claims (Razorpay standard checkout docs).
//
// The official SDK's `validateWebhookSignature` computes the same HMAC but compares
// with `===`; we use crypto.timingSafeEqual over fixed-length digests instead.
import { createHmac, timingSafeEqual } from "node:crypto";

function safeHexEqual(expectedHex: string, presented: string): boolean {
  if (!/^[0-9a-f]{64}$/i.test(presented)) return false;
  const a = Buffer.from(expectedHex, "hex");
  const b = Buffer.from(presented.toLowerCase(), "hex");
  return a.length === b.length && timingSafeEqual(a, b);
}

export function verifyWebhookSignature(
  rawBody: Uint8Array,
  signatureHeader: string | null | undefined,
  secret: string,
): boolean {
  if (!secret || !signatureHeader) return false;
  const expected = createHmac("sha256", secret).update(rawBody).digest("hex");
  return safeHexEqual(expected, signatureHeader.trim());
}

export function verifyCheckoutSignature(
  expectedProviderOrderId: string,
  paymentId: string,
  signature: string,
  keySecret: string,
): boolean {
  if (!keySecret || !expectedProviderOrderId || !paymentId || !signature) return false;
  const expected = createHmac("sha256", keySecret)
    .update(`${expectedProviderOrderId}|${paymentId}`)
    .digest("hex");
  return safeHexEqual(expected, signature.trim());
}

/** Fixture producer for tests: sign a body exactly as Razorpay does. */
export function signWebhookBody(rawBody: Uint8Array | string, secret: string): string {
  return createHmac("sha256", secret).update(rawBody).digest("hex");
}

/** Fixture producer for tests: a checkout signature as Razorpay computes it. */
export function signCheckout(orderId: string, paymentId: string, keySecret: string): string {
  return createHmac("sha256", keySecret).update(`${orderId}|${paymentId}`).digest("hex");
}
