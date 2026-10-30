// Customer completes a paid order that is awaiting its machine code, using the
// order's status capability (form POST from /buy/status/[token]).
import { completeMachineCodeByToken, hitRateLimit, looksLikeStatusToken, OrderActionError, RULES } from "@vidya/billing-core";
import { db } from "@/lib/db";
import { checkoutEnv } from "@/lib/env";
import { clientKey, deny, json, sameOrigin, seeOther } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";

export async function POST(req: Request) {
  if (!surfaceAllows("checkout")) return json({ error: "not_found" }, 404);
  const env = checkoutEnv();
  if (!sameOrigin(req, env.CHECKOUT_ORIGIN)) return deny("csrf");
  if (!(await hitRateLimit(db(), RULES.statusIp, clientKey(req)))) return deny("rate_limited");
  const form = await req.formData();
  const token = String(form.get("token") ?? "");
  if (!looksLikeStatusToken(token)) return json({ error: "not_found" }, 404);
  try {
    await completeMachineCodeByToken(db(), token, String(form.get("machineCode") ?? "").slice(0, 40));
  } catch (err) {
    if (err instanceof OrderActionError) return seeOther(env.CHECKOUT_ORIGIN, `/buy/status/${token}?error=${err.code}`);
    throw err;
  }
  return seeOther(env.CHECKOUT_ORIGIN, `/buy/status/${token}`);
}
