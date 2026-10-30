// Official Razorpay Node SDK, server-only, used by the checkout surface to create
// the provider order for an internal order (amount from the server catalogue).
import "server-only";
import type { RazorpayApi } from "@vidya/billing-core";
import Razorpay from "razorpay";

let override: RazorpayApi | null = null;
export function setRazorpayForTests(api: RazorpayApi | null) {
  override = api;
}

export function razorpay(keyId: string, keySecret: string): RazorpayApi {
  if (override) return override;
  const rzp = new Razorpay({ key_id: keyId, key_secret: keySecret });
  return {
    async fetchOrderPayments(id) {
      const r = await rzp.orders.fetchPayments(id);
      return Array.isArray(r?.items) ? r.items : [];
    },
    async createOrder(input) {
      const o = await rzp.orders.create(input);
      return { id: o.id };
    },
  };
}
