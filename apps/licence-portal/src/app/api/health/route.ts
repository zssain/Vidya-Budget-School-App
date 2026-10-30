// Liveness/readiness: DB reachability only. Exposes no owner data or metrics.
import { db } from "@/lib/db";
import { json } from "@/lib/http";

export async function GET() {
  try {
    await db().query("SELECT 1");
    return json({ ok: true });
  } catch {
    return json({ ok: false }, 503);
  }
}
