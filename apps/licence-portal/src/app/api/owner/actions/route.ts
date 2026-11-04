// Every owner mutation goes through this one handler:
//   surface check → same-origin (CSRF) → requireOwner (UUID + AAL2) → step-up for
//   sensitive actions → zod validation → audited billing-core action → 303 back.
import {
  acknowledgeReview,
  approveManualPayment,
  cancelOrder,
  completeMachineCode,
  createManualOrder,
  OrderActionError,
  requestReconcile,
  requestResend,
  retryFailedIssuance,
  safeOwnerRedirect,
  transferLicence,
} from "@vidya/billing-core";
import { z } from "zod";
import { db } from "@/lib/db";
import { ownerEnv } from "@/lib/env";
import { deny, json, requestId, sameOrigin, seeOther } from "@/lib/http";
import { requireOwner, STEP_UP_SECONDS } from "@/lib/owner-guard";
import { surfaceAllows } from "@/lib/surface";

const id = z.string().uuid();
const reason = z.string().trim().min(5).max(1000);

const ACTIONS = {
  reconcile: { sensitive: false, schema: z.object({ orderId: id }) },
  cancel_order: { sensitive: false, schema: z.object({ orderId: id, reason }) },
  resend: { sensitive: false, schema: z.object({ issuanceId: id, reason }) },
  acknowledge_review: { sensitive: false, schema: z.object({ auditId: z.string().regex(/^\d{1,18}$/), note: reason }) },
  set_machine_code: { sensitive: true, schema: z.object({ orderId: id, machineCode: z.string().max(40), reason }) },
  approve_manual_payment: {
    sensitive: true,
    schema: z.object({ orderId: id, reference: z.string().trim().min(1).max(200), reason, confirm: z.literal("MANUAL") }),
  },
  create_manual_order: {
    sensitive: true,
    schema: z.object({
      productId: z.string().min(1).max(100),
      customerName: z.string().trim().min(2).max(120),
      email: z.string().trim().email().max(320),
      phone: z.string().trim().max(20).optional(),
      schoolName: z.string().trim().min(2).max(200),
      machineCode: z.string().max(40).optional(),
      environment: z.enum(["test", "live"]),
      reason,
    }),
  },
  transfer: { sensitive: true, schema: z.object({ issuanceId: id, newMachineCode: z.string().max(40), reason }) },
  // Retry a FAILED issuance in place (same licence_id, same generation, no new licence).
  retry_issuance: { sensitive: true, schema: z.object({ issuanceId: id, reason }) },
} as const;

type ActionName = keyof typeof ACTIONS;

export async function POST(req: Request) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const env = ownerEnv();
  if (!sameOrigin(req, env.OWNER_ORIGIN)) return deny("csrf");
  const form = Object.fromEntries((await req.formData()).entries());
  const returnTo = safeOwnerRedirect(form.returnTo);
  const name = String(form.action ?? "") as ActionName;
  const spec = ACTIONS[name];
  if (!spec) return json({ error: "unknown_action" }, 400);

  const d = await requireOwner(spec.sensitive ? { freshSeconds: STEP_UP_SECONDS } : {});
  if (!d.ok) {
    if (d.reason === "step_up_required") {
      return seeOther(env.OWNER_ORIGIN, `/mfa?step_up=1&next=${encodeURIComponent(returnTo)}`);
    }
    return deny(d.reason);
  }
  const clean = Object.fromEntries(Object.entries(form).filter(([, v]) => typeof v === "string" && v !== ""));
  const parsed = spec.schema.safeParse(clean);
  const sep = returnTo.includes("?") ? "&" : "?";
  if (!parsed.success) return seeOther(env.OWNER_ORIGIN, `${returnTo}${sep}error=invalid_input`);

  const actor = { ownerId: d.owner.id, requestId: requestId(req) };
  const pool = db();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const v = parsed.data as any;
  let target = returnTo;
  try {
    switch (name) {
      case "reconcile":
        await requestReconcile(pool, actor, v.orderId);
        break;
      case "cancel_order":
        await cancelOrder(pool, actor, v);
        break;
      case "resend":
        await requestResend(pool, actor, v);
        break;
      case "acknowledge_review":
        await acknowledgeReview(pool, actor, v);
        break;
      case "set_machine_code":
        await completeMachineCode(pool, { orderId: v.orderId, machineCode: v.machineCode, actor: d.owner.id, reason: v.reason, requestId: actor.requestId });
        break;
      case "approve_manual_payment":
        await approveManualPayment(pool, actor, v);
        break;
      case "create_manual_order": {
        const orderId = await createManualOrder(pool, actor, { ...v, phone: v.phone ?? null, machineCode: v.machineCode ?? null });
        target = `/orders/${orderId}`;
        break;
      }
      case "transfer": {
        const t = await transferLicence(pool, actor, v);
        target = `/licences/${t.issuanceId}`;
        break;
      }
      case "retry_issuance": {
        const r = await retryFailedIssuance(pool, actor, v);
        target = `/licences/${r.issuanceId}`;
        break;
      }
    }
  } catch (err) {
    if (err instanceof OrderActionError) return seeOther(env.OWNER_ORIGIN, `${returnTo}${sep}error=${encodeURIComponent(err.code)}`);
    throw err;
  }
  const s2 = target.includes("?") ? "&" : "?";
  return seeOther(env.OWNER_ORIGIN, `${target}${s2}done=${name}`);
}
