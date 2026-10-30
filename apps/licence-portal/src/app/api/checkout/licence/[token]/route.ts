// Customer .vlic download, scoped by the unguessable, expiring, hash-stored
// capability to exactly ONE order. Returns the existing artefact only.
import { findOrderByStatusToken, hitRateLimit, RULES } from "@vidya/billing-core";
import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { clientKey, deny, json, NO_STORE } from "@/lib/http";
import { surfaceAllows } from "@/lib/surface";

export async function GET(req: Request, ctx: { params: Promise<{ token: string }> }) {
  if (!surfaceAllows("checkout")) return json({ error: "not_found" }, 404);
  if (!(await hitRateLimit(db(), RULES.statusIp, clientKey(req)))) return deny("rate_limited");
  const { token } = await ctx.params;
  const view = await findOrderByStatusToken(db(), token);
  if (!view?.licence?.vlic) return json({ error: "not_found" }, 404);
  return new NextResponse(view.licence.vlic, {
    headers: {
      ...NO_STORE,
      "Referrer-Policy": "no-referrer",
      "Content-Type": "application/octet-stream",
      "Content-Disposition": `attachment; filename="vidya-licence.vlic"`,
    },
  });
}
