import Link from "next/link";
import { Badge, PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listLicences, PAGE_SIZE, parseList } from "@/lib/queries";

const STATES = ["pending", "issued", "failed", "superseded"];

export default async function LicencesPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/licences");
  const p = parseList(await searchParams);
  const rows = await listLicences(db(), p);
  const qs = new URLSearchParams(Object.entries({ q: p.q, status: p.status }).filter(([, v]) => v) as [string, string][]).toString();
  return (
    <>
      <PageHeader title="Licences" subtitle="Issued licence artefacts. 'Issued' means signed and available — not that it is installed or active: offline perpetual licences do not report activation (shown as 'not reported')." />
      <SearchBar q={p.q} status={p.status} statuses={STATES} exportHref={`/api/owner/export/licences?${qs}`} />
      <Table head={["Created", "School", "Licence", "Gen", "Bound machine", "Modules", "Issuance", "Commercial", "Issued", "Delivery", "Activation"]} empty={rows.length === 0}>
        {rows.map((r) => (
          <tr key={r.id}>
            <Td>{when(r.created_at)}</Td><Td>{r.school_name}</Td>
            <Td mono><Link className="underline" href={`/licences/${r.id}`}>{r.licence_id}</Link></Td>
            <Td>{r.generation}{r.superseded ? <> <Badge value="superseded" /></> : null}</Td>
            <Td mono>{r.machine_code}</Td><Td>{r.modules.join(", ")}</Td>
            <Td><Badge value={r.issuance_status} /></Td><Td><Badge value={r.commercial_status} /></Td>
            <Td>{when(r.issued_ts)}</Td><Td><Badge value={r.delivery_state ?? "not sent"} /></Td>
            <Td className="text-xs text-slate-500">not reported</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/licences?${qs}`} />
    </>
  );
}
