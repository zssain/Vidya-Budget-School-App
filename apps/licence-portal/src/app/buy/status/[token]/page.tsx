// Customer status / download page. Access = the unguessable, expiring capability in
// the URL (only its SHA-256 is stored). It reveals exactly one order and never any
// owner data. Never cached.
import { findOrderByStatusToken, formatInr, hitRateLimit, RULES } from "@vidya/billing-core";
import { headers } from "next/headers";
import { notFound } from "next/navigation";
import { CopyButton, AutoRefresh } from "@/components/client";
import { db } from "@/lib/db";
import { surfaceAllows } from "@/lib/surface";

export const metadata = { title: "Your Vidya order", referrer: "no-referrer" as const };

const STATUS_TEXT: Record<string, string> = {
  created: "Order created — payment not started.",
  pending: "Waiting for payment confirmation from the payment provider.",
  paid: "Payment received.",
  paid_awaiting_machine_code: "Payment received — we need your school PC's machine code to issue the licence.",
  refunded: "This order was refunded.",
  partially_refunded: "Payment received (partially refunded).",
  disputed: "This payment is under dispute.",
  cancelled: "This order was cancelled.",
};

export default async function StatusPage({ params, searchParams }: { params: Promise<{ token: string }>; searchParams: Promise<Record<string, string | undefined>> }) {
  if (!surfaceAllows("checkout")) notFound();
  const { token } = await params;
  const sp = await searchParams;
  const h = await headers();
  const ip = h.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
  if (!(await hitRateLimit(db(), RULES.statusIp, ip))) notFound();
  const v = await findOrderByStatusToken(db(), token);
  if (!v) notFound();
  const lic = v.licence;
  const issued = lic?.status === "issued" && lic.licenceKey;
  const preparing = !issued && (v.status === "paid" || v.status === "partially_refunded");
  return (
    <main className="mx-auto max-w-xl px-4 py-12">
      <h1 className="text-2xl font-semibold">Your Vidya order</h1>
      <p className="mt-1 text-sm text-slate-600">{v.schoolName} · {v.productName} · {formatInr(v.amountPaise)} {v.environment === "test" ? "(TEST MODE — no real money)" : ""}</p>
      <div className="mt-6 rounded-lg border border-slate-200 bg-white p-5">
        <p className="font-medium">{STATUS_TEXT[v.status] ?? v.status}</p>
        {v.status === "pending" || preparing ? <p className="mt-2 text-sm text-slate-600">{preparing ? "Payment received; preparing your licence. This usually takes a minute or two." : "If you just paid, this page will update shortly. A successful-looking browser screen is not yet a confirmed payment."} <AutoRefresh seconds={10} /></p> : null}
        {v.machineCode ? <p className="mt-2 text-sm">Machine code: <span className="mono">{v.machineCode}</span></p> : null}
      </div>

      {v.status === "paid_awaiting_machine_code" && !v.machineCode ? (
        <form method="post" action="/api/checkout/machine-code" className="mt-6 space-y-3 rounded-lg border border-amber-200 bg-amber-50 p-5">
          <input type="hidden" name="token" value={token} />
          <label className="label" htmlFor="machineCode">Machine code (shown by Vidya on the school PC during Welcome → Set up)</label>
          <input id="machineCode" name="machineCode" required className="input mono" placeholder="XXXX-XXXX-XXXX-C" />
          {sp.error ? <p role="alert" className="text-sm text-rose-700">That machine code was not accepted ({sp.error.replace(/_/g, " ")}).</p> : null}
          <button className="btn" type="submit">Issue my licence</button>
          <p className="text-xs text-slate-600">This can be set once. To move a licence to another PC later, contact the seller.</p>
        </form>
      ) : null}

      {issued ? (
        <section className="mt-6 space-y-3 rounded-lg border border-emerald-200 bg-emerald-50 p-5">
          <h2 className="font-semibold">Your licence is ready</h2>
          <p className="text-sm">Licence ID <span className="mono">{lic!.licenceId}</span>, bound to machine <span className="mono">{lic!.machineCode}</span>.</p>
          <ol className="list-decimal space-y-1 pl-5 text-sm">
            <li>On the school PC with this machine code, open Vidya → Welcome → Set up.</li>
            <li>Paste the licence key into the activation field (or load the <b>.vlic</b> file below), then activate.</li>
            <li>Vidya verifies it offline. It will not work on a different PC.</li>
          </ol>
          <pre className="mono whitespace-pre-wrap break-all rounded border bg-white p-3 text-xs">{lic!.licenceKey!.replace(/(.{5})/g, "$1 ").trim()}</pre>
          <div className="flex gap-2">
            <CopyButton text={lic!.licenceKey!} label="Copy licence key" />
            <a className="btn-secondary" href={`/api/checkout/licence/${token}`}>Download .vlic</a>
          </div>
          <p className="text-xs text-slate-600">We have also emailed it to you{v.deliveryState === "sent" ? "" : " (email pending)"}. Keep this page private: anyone with this link can download the licence file.</p>
        </section>
      ) : null}
    </main>
  );
}
