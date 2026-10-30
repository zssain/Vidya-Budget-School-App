import { ownerEnv } from "@/lib/env";
import { deny, json, sameOrigin, seeOther } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";
import { createSupabaseServerClient } from "@/lib/supabase/server";

export async function POST(req: Request) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const env = ownerEnv();
  if (!sameOrigin(req, env.OWNER_ORIGIN)) return deny("csrf");
  const supabase = await createSupabaseServerClient();
  await supabase.auth.signOut({ scope: "global" }); // revoke every session of the owner
  return seeOther(env.OWNER_ORIGIN, "/login?signed_out=1");
}
