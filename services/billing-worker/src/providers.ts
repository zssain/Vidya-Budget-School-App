// Adapters from the official SDKs to the narrow interfaces billing-core depends on.
import type { Mailer, RazorpayApi } from "@vidya/billing-core";
import Razorpay from "razorpay";
import { Resend } from "resend";

export function razorpayApi(keyId: string, keySecret: string): RazorpayApi {
  const rzp = new Razorpay({ key_id: keyId, key_secret: keySecret });
  return {
    async fetchOrderPayments(providerOrderId) {
      const res = await rzp.orders.fetchPayments(providerOrderId);
      return Array.isArray(res?.items) ? res.items : [];
    },
    async createOrder(input) {
      const o = await rzp.orders.create({
        amount: input.amount,
        currency: input.currency,
        receipt: input.receipt,
        notes: input.notes,
      });
      return { id: o.id };
    },
  };
}

export function resendMailer(apiKey: string): Mailer {
  const resend = new Resend(apiKey);
  return {
    async send(msg) {
      const { data, error } = await resend.emails.send(
        {
          from: msg.from,
          to: [msg.to],
          subject: msg.subject,
          text: msg.text,
          attachments: msg.attachments.map((a) => ({
            filename: a.filename,
            content: Buffer.from(a.content, "utf8"),
            contentType: "application/octet-stream",
          })),
        },
        { idempotencyKey: msg.idempotencyKey },
      );
      // The SDK returns errors instead of throwing — convert so the job retries.
      if (error || !data) throw new Error(`resend: ${error?.name ?? "unknown"}: ${error?.message ?? "no data"}`);
      return { id: data.id };
    },
  };
}
