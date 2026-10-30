import Link from "next/link";
import { Badge, inr, PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listPayments, PAGE_SIZE, parseList } from "@/lib/queries";

const STATES = ["authorized", "captured", "failed", "partially_refunded", "refunded", "disputed"];

export default async function PaymentsPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/payments");
  const p = parseList(await searchParams);
  const rows = await listPayments(db(), p);
  const qs = new URLSearchParams(Object.entries({ q: p.q, status: p.status }).filter(([, v]) => v) as [string, string][]).toString();
  return (
    <>
      <PageHeader title="Payments" subtitle="Provider payments recorded from verified webhooks or reconciliation. No card data is received or stored." />
      <SearchBar q={p.q} status={p.status} statuses={STATES} exportHref={`/api/owner/export/payments?${qs}`} />
      <Table head={["Updated", "School", "Email", "Order", "Provider order", "Payment", "Amount", "State", "Refunded", "Captured"]} empty={rows.length === 0}>
        {rows.map((r) => (
          <tr key={r.provider_payment_id}>
            <Td>{when(r.updated_at)}</Td><Td>{r.school_name}</Td><Td>{r.email}</Td>
            <Td mono><Link className="underline" href={`/orders/${r.order_id}`}>{r.order_id.slice(0, 8)}</Link></Td>
            <Td mono>{r.provider_order_id}</Td><Td mono>{r.provider_payment_id}</Td>
            <Td>{inr(r.amount_paise)}</Td><Td><Badge value={r.state} /></Td><Td>{inr(r.refunded_paise)}</Td><Td>{when(r.captured_at)}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/payments?${qs}`} />
    </>
  );
}
