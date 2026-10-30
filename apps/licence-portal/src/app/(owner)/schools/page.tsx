import { PageHeader, Pager, SearchBar, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { listSchools, PAGE_SIZE, parseList } from "@/lib/queries";

export default async function SchoolsPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/schools");
  const p = parseList(await searchParams);
  const rows = await listSchools(db(), p);
  const qs = p.q ? `q=${encodeURIComponent(p.q)}` : "";
  return (
    <>
      <PageHeader title="Schools" subtitle="Commercial buyer metadata only — no student, attendance, fee or Drive data is stored here." />
      <SearchBar q={p.q} exportHref={`/api/owner/export/schools?${qs}`} />
      <Table head={["Since", "School", "Purchaser", "Email", "Phone", "Orders", "Issued licences"]} empty={rows.length === 0}>
        {rows.map((r, i) => (
          <tr key={i}>
            <Td>{when(r.created_at)}</Td><Td>{r.school_name}</Td><Td>{r.customer_name}</Td><Td>{r.email}</Td>
            <Td>{r.phone ?? "—"}</Td><Td>{r.orders}</Td><Td>{r.licences}</Td>
          </tr>
        ))}
      </Table>
      <Pager page={p.page ?? 1} total={rows[0]?.total ?? 0} pageSize={PAGE_SIZE} base={`/schools?${qs}`} />
    </>
  );
}
