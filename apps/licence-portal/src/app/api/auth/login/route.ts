// Owner password sign-in (step 1 of 2; TOTP follows at /mfa). There is NO signup
// endpoint anywhere in this app, and public signups are disabled in Supabase.
import { audit, hitRateLimit, RULES, safeOwnerRedirect } from "@vidya/billing-core";
import { db } from "@/lib/db";
import { ownerEnv } from "@/lib/env";
import { clientKey, deny, json, requestId, sameOrigin, seeOther } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";
import { createSupabaseServerClient } from "@/lib/supabase/server";

export async function POST(req: Request) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const env = ownerEnv();
  if (!sameOrigin(req, env.OWNER_ORIGIN)) return deny("csrf");
  const form = await req.formData();
  const email = String(form.get("email") ?? "").trim().toLowerCase().slice(0, 320);
  const password = String(form.get("password") ?? "").slice(0, 1024);
  const next = safeOwnerRedirect(form.get("next"));
  const back = (error: string) => seeOther(env.OWNER_ORIGIN, `/login?error=${error}&next=${encodeURIComponent(next)}`);
  if (!email || !password) return back("invalid");

  const pool = db();
  const ipOk = await hitRateLimit(pool, RULES.loginIp, clientKey(req));
  const emailOk = await hitRateLimit(pool, RULES.loginEmail, email);
  if (!ipOk || !emailOk) return back("rate_limited");

  const supabase = await createSupabaseServerClient();
  const { data, error } = await supabase.auth.signInWithPassword({ email, password });
  if (error || !data.user) return back("invalid");
  if (data.user.id !== env.OWNER_AUTH_USER_ID) {
    // A valid account that is not THE owner: drop its session, same generic error.
    await supabase.auth.signOut({ scope: "local" });
    return back("invalid");
  }
  await audit(pool, { actor: data.user.id, action: "owner_password_login", requestId: requestId(req) });
  return seeOther(env.OWNER_ORIGIN, `/mfa?next=${encodeURIComponent(next)}`);
}
