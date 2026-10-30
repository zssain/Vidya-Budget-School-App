// TOTP verification: raises the session to AAL2 (first sign-in) or refreshes the
// MFA timestamp (step-up before sensitive actions). Reachable at AAL1 by the owner
// UUID only; rate-limited per user.
import { audit, hitRateLimit, RULES, safeOwnerRedirect } from "@vidya/billing-core";
import { z } from "zod";
import { db } from "@/lib/db";
import { ownerEnv } from "@/lib/env";
import { deny, json, requestId, sameOrigin } from "@/lib/http";
import { requireOwner } from "@/lib/owner-guard";
import { surfaceAllows } from "@/lib/surface";
import { createSupabaseServerClient } from "@/lib/supabase/server";

const Body = z.object({
  code: z.string().regex(/^\d{6}$/),
  factorId: z.string().uuid().optional(),
  next: z.string().max(512).optional(),
});

export async function POST(req: Request) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const env = ownerEnv();
  if (!sameOrigin(req, env.OWNER_ORIGIN)) return deny("csrf");
  const d = await requireOwner({ allowAal1: true });
  if (!d.ok) return deny(d.reason);
  const parsed = Body.safeParse(await req.json().catch(() => null));
  if (!parsed.success) return json({ error: "invalid_code" }, 400);
  if (!(await hitRateLimit(db(), RULES.mfaUser, `verify:${d.owner.id}`))) return deny("rate_limited");

  const supabase = await createSupabaseServerClient();
  let factorId = parsed.data.factorId;
  const list = await supabase.auth.mfa.listFactors();
  const mine = (list.data?.all ?? []).filter((f) => f.factor_type === "totp");
  if (factorId && !mine.some((f) => f.id === factorId)) return json({ error: "unknown_factor" }, 400);
  factorId ??= mine.find((f) => f.status === "verified")?.id;
  if (!factorId) return json({ error: "no_factor" }, 400);

  const { error } = await supabase.auth.mfa.challengeAndVerify({ factorId, code: parsed.data.code });
  if (error) return json({ error: "invalid_code" }, 400);
  await audit(db(), { actor: d.owner.id, action: "owner_mfa_verified", requestId: requestId(req) });
  return json({ ok: true, next: safeOwnerRedirect(parsed.data.next) });
}
