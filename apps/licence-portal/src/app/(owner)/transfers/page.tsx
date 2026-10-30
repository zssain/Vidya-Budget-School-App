import Link from "next/link";
import { Badge, PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listTransfers, PAGE_SIZE, parseList } from "@/lib/queries";

export default async function TransfersPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/transfers");
  const p = parseList(await searchParams);
  const rows = await listTransfers(db(), p);
  const qs = p.q ? `q=${encodeURIComponent(p.q)}` : "";
  return (
    <>
      <PageHeader title="Transfers" subtitle="Owner-approved machine changes. Start a transfer from a licence's page. The old PC's offline licence is not remotely disabled." />
      <SearchBar q={p.q} exportHref={`/api/owner/export/transfers?${qs}`} />
      <Table head={["Approved", "School", "From licence", "From machine", "To machine", "Replacement", "Status", "Reason"]} empty={rows.length === 0}>
        {rows.map((r, i) => (
          <tr key={i}>
            <Td>{when(r.approved_at)}</Td><Td>{r.school_name}</Td>
            <Td mono><Link className="underline" href={`/licences/${r.source_id}`}>{r.source_licence_id}</Link></Td>
            <Td mono>{r.from_machine}</Td><Td mono>{r.target_machine_code}</Td>
            <Td mono>{r.replacement_id ? <Link className="underline" href={`/licences/${r.replacement_id}`}>{r.replacement_licence_id}</Link> : "—"}</Td>
            <Td><Badge value={r.replacement_status} /></Td><Td className="whitespace-normal">{r.reason}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/transfers?${qs}`} />
    </>
  );
}
