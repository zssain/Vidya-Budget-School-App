// Narrow, defensive normalisation of Razorpay payment entities (from webhooks or
// from the Orders API during reconciliation) into one observation shape. Unknown
// or malformed input → null (the caller ignores/flags it; it never issues).

export type PaymentState =
  | "authorized"
  | "captured"
  | "failed"
  | "refunded"
  | "partially_refunded"
  | "disputed";

export interface PaymentObservation {
  providerPaymentId: string;
  providerOrderId: string | null;
  amountPaise: number;
  currency: string;
  state: PaymentState;
  /** Cumulative refunded amount reported by the provider (monotonic). */
  refundedPaise: number;
  /** True when the provider says the money was captured (even if later refunded). */
  captureHappened: boolean;
}

function str(v: unknown): string | null {
  return typeof v === "string" && v.length > 0 && v.length <= 200 ? v : null;
}
function int(v: unknown): number | null {
  return typeof v === "number" && Number.isSafeInteger(v) && v >= 0 ? v : null;
}

export function normalisePaymentEntity(entity: unknown): PaymentObservation | null {
  if (typeof entity !== "object" || entity === null) return null;
  const e = entity as Record<string, unknown>;
  const id = str(e.id);
  const amount = int(e.amount);
  const currency = str(e.currency);
  const status = str(e.status);
  if (!id || !id.startsWith("pay_") || amount === null || !currency || !status) return null;
  const refunded = int(e.amount_refunded) ?? 0;
  const refundStatus = str(e.refund_status);
  const capturedFlag = e.captured === true;

  let state: PaymentState;
  switch (status) {
    case "authorized":
      state = "authorized";
      break;
    case "failed":
      state = "failed";
      break;
    case "captured":
      state = refundStatus === "partial" ? "partially_refunded" : "captured";
      break;
    case "refunded":
      state = refundStatus === "partial" ? "partially_refunded" : "refunded";
      break;
    default:
      return null; // 'created' etc. — nothing to record
  }
  return {
    providerPaymentId: id,
    providerOrderId: str(e.order_id),
    amountPaise: amount,
    currency,
    state,
    refundedPaise: refunded,
    captureHappened: status === "captured" || status === "refunded" || capturedFlag,
  };
}

/** Pull `payload.<name>.entity` out of a webhook event body. */
export function eventEntity(event: unknown, name: string): unknown {
  if (typeof event !== "object" || event === null) return undefined;
  const p = (event as { payload?: Record<string, { entity?: unknown }> }).payload;
  return p?.[name]?.entity;
}
