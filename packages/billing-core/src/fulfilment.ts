// Owner-facing fulfilment label: payment, issuance and delivery are separate state
// machines; this pure function combines them into the one line the owner reads
// (owner decisions 2026-11-05):
//   * paid, signing not done or FAILED  → "Paid · Licence pending" (+ Retry issuance)
//   * issued, email failed              → "Issued · Email failed"  (+ Resend licence)
//   * issued, email sent                → "Issued · Emailed"
// A refund/dispute never changes an issued licence (offline perpetual): it is shown
// as a review note on top of the fulfilment state.
import type { OrderStatus } from "./provisioning";

export type IssuanceStatus = "pending" | "issued" | "failed" | "superseded";
export type DeliveryState = "pending" | "sent" | "failed";

export interface FulfilmentInput {
  orderStatus: OrderStatus;
  /** Status of the LATEST generation's issuance, or null if none exists yet. */
  issuanceStatus: IssuanceStatus | null;
  /** Newest delivery row state for that issuance, or null if none yet. */
  deliveryState: DeliveryState | null;
  /** True when its delivery job dead-lettered (no further automatic retries). */
  deliveryDead?: boolean;
}

export interface Fulfilment {
  label: string;
  tone: "ok" | "warn" | "bad" | "neutral";
  /** The owner action that resolves this state, if any. */
  action: "retry_issuance" | "resend" | "set_machine_code" | null;
  hint: string | null;
}

const PAID_LIKE: ReadonlySet<OrderStatus> = new Set(["paid", "partially_refunded", "refunded", "disputed"]);

export function fulfilmentState(x: FulfilmentInput): Fulfilment {
  if (x.orderStatus === "paid_awaiting_machine_code") {
    return { label: "Paid · Awaiting machine code", tone: "warn", action: "set_machine_code", hint: null };
  }
  if (!PAID_LIKE.has(x.orderStatus)) {
    return { label: x.orderStatus === "cancelled" ? "Cancelled" : "Not paid", tone: "neutral", action: null, hint: null };
  }
  const review =
    x.orderStatus === "paid" ? null : `${x.orderStatus.replace(/_/g, " ")} — owner review; an issued offline licence keeps working`;
  if (x.issuanceStatus === "issued" || x.issuanceStatus === "superseded") {
    if (x.deliveryState === "sent") return { label: "Issued · Emailed", tone: review ? "warn" : "ok", action: "resend", hint: review };
    if (x.deliveryState === "failed") {
      return {
        label: "Issued · Email failed",
        tone: "bad",
        action: "resend",
        hint: review ?? (x.deliveryDead ? "automatic retries exhausted — use Resend licence" : "retrying automatically; Resend licence sends the same artefact"),
      };
    }
    return { label: "Issued · Email sending", tone: review ? "warn" : "ok", action: null, hint: review };
  }
  if (x.issuanceStatus === "failed") {
    // A partial refund leaves the entitlement active; full refund / dispute block it.
    const active = x.orderStatus === "paid" || x.orderStatus === "partially_refunded";
    return {
      label: "Paid · Licence pending",
      tone: "bad",
      action: active ? "retry_issuance" : null,
      hint: review ?? "signing failed — Retry issuance reuses the same licence ID",
    };
  }
  return { label: "Paid · Licence pending", tone: "warn", action: null, hint: review };
}
