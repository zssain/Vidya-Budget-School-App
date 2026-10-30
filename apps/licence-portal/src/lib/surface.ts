// Deployment separation. The same codebase is deployed TWICE in production:
//   PORTAL_SURFACE=owner     → owner.<domain>  (owner pages + /api/auth + /api/owner)
//   PORTAL_SURFACE=checkout  → buy.<domain>    (/buy + /api/checkout + /api/webhooks)
// Each deployment holds only its own secrets (the checkout one has no Supabase
// config; the owner one has no Razorpay secrets). Routes of the other surface 404
// — in proxy.ts AND again in every route handler (defence in depth).
import "server-only";
import { surface } from "./env";

export { surfaceOfPath, type RouteSurface } from "./surface-path";

export function surfaceAllows(s: "owner" | "checkout"): boolean {
  const cur = surface();
  return cur === "both" || cur === s;
}
