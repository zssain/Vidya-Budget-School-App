import Link from "next/link";
import { Badge, PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listDeliveries, PAGE_SIZE, parseList } from "@/lib/queries";

export default async function DeliveryPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/delivery");
  const p = parseList(await searchParams);
  const rows = await listDeliveries(db(), p);
  const qs = new URLSearchParams(Object.entries({ q: p.q, status: p.status }).filter(([, v]) => v) as [string, string][]).toString();
  return (
    <>
      <PageHeader title="Delivery" subtitle="Licence emails. Failures are retried with backoff; a failed email never re-issues a licence. Resend from the licence page." />
      <SearchBar q={p.q} status={p.status} statuses={["pending", "sent", "failed"]} exportHref={`/api/owner/export/delivery?${qs}`} />
      <Table head={["Updated", "School", "Email", "Licence", "Channel", "State", "Attempts", "Last error", "Provider message"]} empty={rows.length === 0}>
        {rows.map((r, i) => (
          <tr key={i}>
            <Td>{when(r.updated_at)}</Td><Td>{r.school_name}</Td><Td>{r.email}</Td>
            <Td mono><Link className="underline" href={`/licences/${r.issuance_id}`}>{r.licence_id}</Link></Td>
            <Td>{r.channel}</Td><Td><Badge value={r.state} /></Td><Td>{r.attempts}</Td>
            <Td className="max-w-xs whitespace-normal text-xs">{r.last_error}</Td><Td mono>{r.provider_message_id ?? "—"}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/delivery?${qs}`} />
    </>
  );
}
