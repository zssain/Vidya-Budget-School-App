// Browser return from Razorpay Checkout. Browser success is a PROMPT TO RECONCILE,
// not proof of payment: we verify the checkout signature against the provider
// order WE stored for this capability, then enqueue a reconcile job. Only the
// worker's verified capture marks the order paid.
import { recordCheckoutReturn } from "@vidya/billing-core";
import { z } from "zod";
import { db } from "@/lib/db";
import { checkoutEnv } from "@/lib/env";
import { deny, json, sameOrigin } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";

const Body = z.object({
  statusToken: z.string().max(64),
  razorpay_payment_id: z.string().max(64),
  razorpay_order_id: z.string().max(64).optional(), // ignored: we use the stored order id
  razorpay_signature: z.string().max(128),
});

export async function POST(req: Request) {
  if (!surfaceAllows("checkout")) return json({ error: "not_found" }, 404);
  const env = checkoutEnv();
  if (!sameOrigin(req, env.CHECKOUT_ORIGIN)) return deny("csrf");
  const parsed = Body.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return json({ error: "invalid_input" }, 400);
  const r = await recordCheckoutReturn(db(), {
    statusToken: parsed.data.statusToken,
    paymentId: parsed.data.razorpay_payment_id,
    signature: parsed.data.razorpay_signature,
    keySecret: env.RAZORPAY_KEY_SECRET,
  });
  return r === "verifying" ? json({ status: "verifying" }) : json({ error: "invalid_signature" }, 400);
}
