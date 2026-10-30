// Owner download of the EXACT stored .vlic artefact (never regenerated).
import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { deny, json, NO_STORE } from "@/lib/http";
import { requireOwner } from "@/lib/owner-guard";
import { surfaceAllows } from "@/lib/surface";

export async function GET(_req: Request, ctx: { params: Promise<{ id: string }> }) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const d = await requireOwner();
  if (!d.ok) return deny(d.reason);
  const { id } = await ctx.params;
  if (!/^[0-9a-f-]{36}$/i.test(id)) return json({ error: "not_found" }, 404);
  const r = await db().query<{ vlic: string | null; licence_id: string }>(
    `SELECT vlic, licence_id FROM billing.licence_issuances WHERE id = $1 AND status = 'issued'`,
    [id],
  );
  const row = r.rows[0];
  if (!row?.vlic) return json({ error: "not_found" }, 404);
  return new NextResponse(row.vlic, {
    headers: {
      ...NO_STORE,
      "Content-Type": "application/octet-stream",
      "Content-Disposition": `attachment; filename="${row.licence_id.replace(/[^A-Za-z0-9-]/g, "")}.vlic"`,
    },
  });
}
