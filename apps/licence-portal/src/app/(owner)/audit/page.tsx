import { PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listAudit, PAGE_SIZE, parseList } from "@/lib/queries";

export default async function AuditPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/audit");
  const p = parseList(await searchParams);
  const rows = await listAudit(db(), p);
  const qs = p.q ? `q=${encodeURIComponent(p.q)}` : "";
  return (
    <>
      <PageHeader title="Audit log" subtitle="Append-only (enforced by a database trigger for every application role). Actors: owner UUID, worker, signer, checkout, customer." />
      <SearchBar q={p.q} exportHref={`/api/owner/export/audit?${qs}`} />
      <Table head={["When", "Actor", "Action", "Target", "Reason", "Request", "Detail"]} empty={rows.length === 0}>
        {rows.map((r) => (
          <tr key={r.id}>
            <Td>{when(r.at)}</Td><Td mono>{r.actor}</Td><Td>{r.action}</Td><Td mono>{r.target ?? "—"}</Td>
            <Td className="max-w-xs whitespace-normal">{r.reason}</Td><Td mono>{r.request_id?.slice(0, 8) ?? "—"}</Td>
            <Td className="max-w-sm whitespace-normal break-all text-xs">{r.after_json ? JSON.stringify(r.after_json) : ""}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/audit?${qs}`} />
    </>
  );
}
