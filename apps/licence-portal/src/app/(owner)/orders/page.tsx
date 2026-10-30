import Link from "next/link";
import { ActionForm, Badge, inr, Notice, PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listOrders, PAGE_SIZE, parseList, products } from "@/lib/queries";

const STATUSES = ["created", "pending", "paid", "paid_awaiting_machine_code", "partially_refunded", "refunded", "disputed", "cancelled"];

export default async function OrdersPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/orders");
  const sp = await searchParams;
  const p = parseList(sp);
  const [rows, prods] = await Promise.all([listOrders(db(), p), products(db())]);
  const qs = new URLSearchParams(Object.entries({ q: p.q, status: p.status }).filter(([, v]) => v) as [string, string][]).toString();
  return (
    <>
      <PageHeader title="Orders" subtitle="Every checkout attempt and imported order. 'Paid' appears only after a provider-verified capture (or a labelled manual approval)." />
      <Notice sp={sp} />
      <SearchBar q={p.q} status={p.status} statuses={STATUSES} exportHref={`/api/owner/export/orders?${qs}`} />
      <Table head={["Created", "School", "Purchaser", "Internal order", "Provider order", "Amount", "Status", "Machine"]} empty={rows.length === 0}>
        {rows.map((r) => (
          <tr key={r.order_id}>
            <Td>{when(r.created_at)}</Td>
            <Td>{r.school_name}</Td>
            <Td><div>{r.email}</div><div className="text-xs text-slate-500">{r.phone ?? ""}</div></Td>
            <Td mono><Link className="underline" href={`/orders/${r.order_id}`}>{r.order_id.slice(0, 8)}</Link></Td>
            <Td mono>{r.provider === "manual" ? <Badge value="manual" /> : r.provider_order_id ?? "—"}</Td>
            <Td>{inr(r.amount_paise)} {r.environment === "test" ? <Badge value="test" /> : null}</Td>
            <Td><Badge value={r.status} /></Td>
            <Td mono>{r.machine_code ?? "—"}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/orders?${qs}`} />

      <details className="mt-8 max-w-xl">
        <summary className="cursor-pointer text-sm font-medium">Record an imported / manual order (owner action)</summary>
        <p className="my-2 text-xs text-slate-500">For payments received outside Razorpay. This does NOT mark anything paid; approve the payment separately on the order page. Requires a fresh MFA code.</p>
        <ActionForm action="create_manual_order" returnTo="/orders" label="Create manual order">
          <div className="grid grid-cols-2 gap-2">
            <div><label className="label" htmlFor="productId">Product</label>
              <select id="productId" name="productId" className="input" required>
                {prods.map((x) => <option key={x.id} value={x.id}>{x.name} ({inr(x.price_paise)})</option>)}
              </select></div>
            <div><label className="label" htmlFor="environment">Environment</label>
              <select id="environment" name="environment" className="input"><option value="test">test</option><option value="live">live</option></select></div>
            <div><label className="label" htmlFor="schoolName">School name</label><input id="schoolName" name="schoolName" required className="input" /></div>
            <div><label className="label" htmlFor="customerName">Purchaser name</label><input id="customerName" name="customerName" required className="input" /></div>
            <div><label className="label" htmlFor="email">Email</label><input id="email" name="email" type="email" required className="input" /></div>
            <div><label className="label" htmlFor="phone">Phone</label><input id="phone" name="phone" className="input" /></div>
            <div className="col-span-2"><label className="label" htmlFor="machineCode">Machine code (optional — leave blank to collect later)</label><input id="machineCode" name="machineCode" className="input mono" placeholder="XXXX-XXXX-XXXX-C" /></div>
          </div>
        </ActionForm>
      </details>
    </>
  );
}
