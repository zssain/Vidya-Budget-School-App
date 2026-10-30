import Link from "next/link";
import { notFound } from "next/navigation";
import { ActionForm, Badge, inr, Notice, PageHeader, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { orderDetail } from "@/lib/queries";

export default async function OrderPage({ params, searchParams }: { params: Promise<{ id: string }>; searchParams: Promise<Record<string, string | undefined>> }) {
  const { id } = await params;
  const back = `/orders/${id}`;
  await requireOwnerPage(back);
  if (!/^[0-9a-f-]{36}$/i.test(id)) notFound();
  const sp = await searchParams;
  const d = await orderDetail(db(), id);
  if (!d) notFound();
  const o = d.order;
  return (
    <>
      <PageHeader title={`Order ${o.id.slice(0, 8)}`} subtitle={`${o.school_name} · ${o.product_name}`}>
        <Badge value={o.status} />
      </PageHeader>
      <Notice sp={sp} />
      <dl className="mb-6 grid max-w-3xl grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <dt className="text-slate-500">Internal order</dt><dd className="mono text-xs">{o.id}</dd>
        <dt className="text-slate-500">Provider</dt><dd>{o.provider} · <span className="mono text-xs">{o.provider_order_id ?? "—"}</span></dd>
        <dt className="text-slate-500">Amount (snapshot)</dt><dd>{inr(o.amount_paise)} {o.currency} · {o.environment}</dd>
        <dt className="text-slate-500">Modules (snapshot)</dt><dd>{o.modules.join(", ")}</dd>
        <dt className="text-slate-500">Machine code</dt><dd className="mono">{o.machine_code ?? "— (awaiting)"}</dd>
        <dt className="text-slate-500">Purchaser</dt><dd>{o.customer_name} · {o.email} · {o.phone ?? ""}</dd>
        <dt className="text-slate-500">Created</dt><dd>{when(o.created_at)}</dd>
      </dl>

      <h2 className="mb-2 text-base font-semibold">Payments (provider-verified)</h2>
      <Table head={["Payment", "Amount", "State", "Refunded", "Captured"]} empty={d.payments.length === 0}>
        {d.payments.map((p) => (
          <tr key={p.provider_payment_id}>
            <Td mono>{p.provider_payment_id}</Td><Td>{inr(p.amount_paise)}</Td><Td><Badge value={p.state} /></Td>
            <Td>{inr(p.refunded_paise)}</Td><Td>{when(p.captured_at)}</Td>
          </tr>
        ))}
      </Table>

      <h2 className="mb-2 mt-6 text-base font-semibold">Licences</h2>
      <Table head={["Generation", "Licence", "Machine", "Issuance", "Commercial"]} empty={d.issuances.length === 0}>
        {d.issuances.map((i) => (
          <tr key={i.id}>
            <Td>{i.generation}</Td><Td mono><Link className="underline" href={`/licences/${i.id}`}>{i.licence_id}</Link></Td>
            <Td mono>{i.machine_code}</Td><Td><Badge value={i.status} /></Td><Td><Badge value={i.commercial_status} /></Td>
          </tr>
        ))}
      </Table>

      <h2 className="mb-2 mt-6 text-base font-semibold">Owner actions</h2>
      <div className="grid max-w-4xl gap-3 md:grid-cols-2">
        {o.provider === "razorpay" && o.provider_order_id ? (
          <ActionForm action="reconcile" returnTo={back} fields={{ orderId: o.id }} needsReason={false} label="Reconcile with Razorpay now"
            note="Asks Razorpay for this order's payments using server credentials. Only a verified capture marks it paid." />
        ) : null}
        {o.status === "created" || o.status === "pending" ? (
          <ActionForm action="cancel_order" returnTo={back} fields={{ orderId: o.id }} label="Cancel order" danger />
        ) : null}
        {o.provider === "manual" && (o.status === "pending" || o.status === "created") ? (
          <ActionForm action="approve_manual_payment" returnTo={back} fields={{ orderId: o.id }} label="MANUAL payment approval" danger
            note="Separately labelled owner action — NOT an automatically verified payment. Recorded in the audit log; excluded from verified sales. Requires a fresh MFA code.">
            <div><label className="label" htmlFor="reference">Payment reference (UTR / bank ref)</label><input id="reference" name="reference" required className="input" /></div>
            <div><label className="label" htmlFor="confirm">Type MANUAL to confirm</label><input id="confirm" name="confirm" required pattern="MANUAL" className="input" /></div>
          </ActionForm>
        ) : null}
        {o.status === "paid_awaiting_machine_code" && !o.machine_code ? (
          <ActionForm action="set_machine_code" returnTo={back} fields={{ orderId: o.id }} label="Bind machine code and issue"
            note="One-time completion for a paid order without a machine code. Later changes require a transfer. Requires a fresh MFA code.">
            <div><label className="label" htmlFor="machineCode">Machine code (from Vidya on the school PC, Welcome → Set up)</label><input id="machineCode" name="machineCode" required className="input mono" placeholder="XXXX-XXXX-XXXX-C" /></div>
          </ActionForm>
        ) : null}
      </div>

      <h2 className="mb-2 mt-6 text-base font-semibold">Audit timeline</h2>
      <Table head={["When", "Actor", "Action", "Reason"]} empty={d.audit.length === 0}>
        {d.audit.map((a) => (
          <tr key={a.id}><Td>{when(a.at)}</Td><Td mono>{a.actor}</Td><Td>{a.action}</Td><Td className="whitespace-normal">{a.reason}</Td></tr>
        ))}
      </Table>
    </>
  );
}
