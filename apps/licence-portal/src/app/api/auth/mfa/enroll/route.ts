// TOTP enrolment — the ONLY owner capability reachable at AAL1, and only:
//  * for the configured owner UUID,
//  * with a password sign-in in the last 10 minutes,
//  * when no verified TOTP factor exists yet (adding/replacing a factor once one
//    exists is an owner-recovery procedure, see docs/implementation/runbooks).
import { audit, hitRateLimit, RULES } from "@vidya/billing-core";
import { db } from "@/lib/db";
import { ownerEnv } from "@/lib/env";
import { deny, json, requestId, sameOrigin } from "@/lib/http";
import { passwordIsFresh, requireOwner } from "@/lib/owner-guard";
import { surfaceAllows } from "@/lib/surface";
import { createSupabaseServerClient } from "@/lib/supabase/server";

export async function POST(req: Request) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const env = ownerEnv();
  if (!sameOrigin(req, env.OWNER_ORIGIN)) return deny("csrf");
  const d = await requireOwner({ allowAal1: true });
  if (!d.ok) return deny(d.reason);
  if (d.owner.hasVerifiedTotp) return json({ error: "already_enrolled" }, 409);
  if (!passwordIsFresh(d.owner)) {
    return json({ error: "recent_password_required" }, 403);
  }
  if (!(await hitRateLimit(db(), RULES.mfaUser, `enroll:${d.owner.id}`))) return deny("rate_limited");

  const supabase = await createSupabaseServerClient();
  const list = await supabase.auth.mfa.listFactors();
  for (const f of list.data?.all ?? []) {
    if (f.factor_type === "totp" && f.status === "unverified") await supabase.auth.mfa.unenroll({ factorId: f.id });
  }
  const { data, error } = await supabase.auth.mfa.enroll({
    factorType: "totp",
    friendlyName: `vidya-owner-${new Date().toISOString().slice(0, 19)}`,
    issuer: "Vidya Owner Portal",
  });
  if (error || !data) return json({ error: "enroll_failed" }, 502);
  await audit(db(), { actor: d.owner.id, action: "owner_mfa_enrolment_started", requestId: requestId(req) });
  return json({ factorId: data.id, qrCode: data.totp.qr_code, secret: data.totp.secret });
}
