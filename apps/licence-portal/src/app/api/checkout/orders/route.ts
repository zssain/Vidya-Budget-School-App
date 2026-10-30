// Public: create an internal order (amount/modules from the SERVER catalogue) and
// its Razorpay order. The response carries only public checkout values plus the
// customer's one-order status capability URL. Nothing is "paid" here.
import { attachProviderOrder, CheckoutError, createCheckoutOrder, hitRateLimit, RULES } from "@vidya/billing-core";
import { z } from "zod";
import { db } from "@/lib/db";
import { checkoutEnv } from "@/lib/env";
import { clientKey, deny, json, sameOrigin } from "@/lib/http";
import { razorpay } from "@/lib/razorpay";
import { surfaceAllows } from "@/lib/surface";

const Body = z.object({
  productId: z.string().min(1).max(100),
  customerName: z.string().trim().min(2).max(120),
  email: z.string().trim().toLowerCase().email().max(320),
  phone: z.string().trim().regex(/^\+?[0-9 ()-]{6,20}$/).optional().or(z.literal("").transform(() => undefined)),
  schoolName: z.string().trim().min(2).max(200),
  machineCode: z.string().max(40),
  confirm: z.literal(true),
});

export async function POST(req: Request) {
  if (!surfaceAllows("checkout")) return json({ error: "not_found" }, 404);
  const env = checkoutEnv();
  if (!sameOrigin(req, env.CHECKOUT_ORIGIN)) return deny("csrf");
  const pool = db();
  if (!(await hitRateLimit(pool, RULES.checkoutIp, clientKey(req)))) return deny("rate_limited");
  const parsed = Body.safeParse(await req.json().catch(() => null));
  if (!parsed.success) {
    return json({ error: "invalid_input", fields: [...new Set(parsed.error.issues.map((i) => i.path.join(".")))] }, 400);
  }
  const b = parsed.data;
  let created;
  try {
    created = await createCheckoutOrder(pool, {
      productId: b.productId,
      customerName: b.customerName,
      email: b.email,
      phone: b.phone ?? null,
      schoolName: b.schoolName,
      machineCode: b.machineCode,
      environment: env.RAZORPAY_MODE,
    });
  } catch (err) {
    if (err instanceof CheckoutError) return json({ error: err.code }, 400);
    throw err;
  }
  let providerOrderId: string;
  try {
    const o = await razorpay(env.RAZORPAY_KEY_ID, env.RAZORPAY_KEY_SECRET).createOrder({
      amount: created.amountPaise,
      currency: created.currency,
      receipt: created.orderId.slice(0, 40),
      notes: { internal_order_id: created.orderId },
    });
    providerOrderId = o.id;
  } catch {
    // Order stays 'created' (visible to the owner); the customer may retry.
    return json({ error: "provider_unavailable" }, 502);
  }
  await attachProviderOrder(pool, created.orderId, providerOrderId);
  return json({
    statusUrl: `/buy/status/${created.statusToken}`,
    statusToken: created.statusToken,
    checkout: {
      key: env.RAZORPAY_KEY_ID, // public identifier by design (Razorpay standard checkout)
      order_id: providerOrderId,
      amount: created.amountPaise,
      currency: created.currency,
      name: env.CHECKOUT_BRAND_NAME,
      description: `${created.productName} — machine ${created.machineCode}`,
      prefill: { name: b.customerName, email: b.email, contact: b.phone ?? "" },
    },
  });
}
