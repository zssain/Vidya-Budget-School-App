"use client";
// Collects details, asks the SERVER to create the order, then opens Razorpay
// Standard Checkout with only the public values the server returned. Whatever the
// browser reports afterwards, the status page shows "paid" only after the worker
// verifies the capture with Razorpay.
import { ShoppingCart } from "lucide-react";
import { useState } from "react";

type P = { id: string; name: string; price: string; modules: string[] };

declare global {
  interface Window {
    Razorpay?: new (opts: Record<string, unknown>) => { open(): void; on(ev: string, cb: () => void): void };
  }
}

// Lightweight shape check only; the server re-validates with the full checksum.
const SHAPE = /^[0-9A-HJKMNP-TV-Z]{4}-?[0-9A-HJKMNP-TV-Z]{4}-?[0-9A-HJKMNP-TV-Z]{4}-?[0-9A-HJKMNP-TV-Z]$/i;

function loadCheckout(): Promise<void> {
  if (window.Razorpay) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = "https://checkout.razorpay.com/v1/checkout.js";
    s.onload = () => resolve();
    s.onerror = () => reject(new Error("checkout script failed to load"));
    document.head.appendChild(s);
  });
}

export function BuyClient({ products }: { products: P[] }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    const f = new FormData(e.currentTarget);
    const machineCode = String(f.get("machineCode") ?? "").trim();
    if (!SHAPE.test(machineCode.replace(/\s/g, ""))) {
      setError("That does not look like a machine code (XXXX-XXXX-XXXX-C).");
      return;
    }
    setBusy(true);
    const r = await fetch("/api/checkout/orders", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        productId: f.get("productId"),
        customerName: f.get("customerName"),
        email: f.get("email"),
        phone: f.get("phone"),
        schoolName: f.get("schoolName"),
        machineCode,
        confirm: f.get("confirm") === "on",
      }),
    });
    const body = await r.json().catch(() => ({}));
    if (!r.ok) {
      setBusy(false);
      setError(
        body.error === "invalid_machine_code" ? "The machine code's check character does not match. Re-check it on the school PC."
        : body.error === "rate_limited" ? "Too many attempts. Please try again later."
        : body.error === "provider_unavailable" ? "The payment provider is unavailable. Please try again shortly."
        : "Please check the form and try again.",
      );
      return;
    }
    const statusUrl: string = body.statusUrl;
    try {
      await loadCheckout();
    } catch {
      window.location.assign(statusUrl);
      return;
    }
    const rzp = new window.Razorpay!({
      ...body.checkout,
      handler: async (resp: Record<string, string>) => {
        await fetch("/api/checkout/verify", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ statusToken: body.statusToken, ...resp }),
        }).catch(() => undefined);
        window.location.assign(statusUrl);
      },
      modal: { ondismiss: () => window.location.assign(statusUrl) },
    });
    rzp.open();
  }

  return (
    <form onSubmit={submit} className="mt-8 space-y-4 rounded-lg border border-slate-200 bg-white p-6">
      <div>
        <label className="label" htmlFor="productId">Licence</label>
        <select id="productId" name="productId" className="input" required>
          {products.map((p) => <option key={p.id} value={p.id}>{p.name} — {p.price} (modules: {p.modules.join(", ")})</option>)}
        </select>
      </div>
      <div><label className="label" htmlFor="schoolName">School name (printed on the licence)</label><input id="schoolName" name="schoolName" required minLength={2} maxLength={200} className="input" /></div>
      <div><label className="label" htmlFor="machineCode">School PC machine code</label><input id="machineCode" name="machineCode" required className="input mono" placeholder="XXXX-XXXX-XXXX-C" autoComplete="off" /></div>
      <div className="grid grid-cols-2 gap-3">
        <div><label className="label" htmlFor="customerName">Your name</label><input id="customerName" name="customerName" required className="input" /></div>
        <div><label className="label" htmlFor="phone">Phone (optional)</label><input id="phone" name="phone" className="input" /></div>
      </div>
      <div><label className="label" htmlFor="email">Email (the licence is sent here)</label><input id="email" name="email" type="email" required className="input" /></div>
      <label className="flex items-start gap-2 text-sm">
        <input type="checkbox" name="confirm" required className="mt-1" />
        <span>I confirm the school name and machine code are correct. The licence will only work on the PC with this machine code.</span>
      </label>
      {error ? <p role="alert" className="text-sm text-rose-700">{error}</p> : null}
      <button className="btn" disabled={busy} type="submit"><ShoppingCart size={16} aria-hidden /> {busy ? "Please wait…" : "Continue to payment"}</button>
    </form>
  );
}
