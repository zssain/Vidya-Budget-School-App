// Read-only configuration summary. Secrets are never displayed (not even masked
// prefixes of secrets); the catalogue is changed by SQL (see runbook) because the
// portal's DB role deliberately has no write access to billing.products.
import { Badge, inr, PageHeader, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { surface } from "@/lib/env";
import { requireOwnerPage } from "@/lib/owner-guard";
import { products } from "@/lib/queries";

export default async function SettingsPage() {
  const owner = await requireOwnerPage("/settings");
  const prods = await products(db());
  return (
    <>
      <PageHeader title="Settings" subtitle="Read-only. Configuration lives in the deployment's server environment; see docs/implementation/runbooks." />
      <dl className="mb-8 grid max-w-3xl grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <dt className="text-slate-500">This deployment</dt><dd>{surface()}</dd>
        <dt className="text-slate-500">Owner Auth user (allow-listed UUID)</dt><dd className="mono text-xs">{owner.id}</dd>
        <dt className="text-slate-500">Session assurance</dt><dd>{owner.aal} · MFA verified {owner.lastMfaAt ? when(new Date(owner.lastMfaAt * 1000)) : "—"}</dd>
        <dt className="text-slate-500">Owner origin</dt><dd className="mono text-xs">{process.env.OWNER_ORIGIN ?? "—"}</dd>
        <dt className="text-slate-500">Checkout origin</dt><dd className="mono text-xs">{process.env.CHECKOUT_ORIGIN ?? "— (set on the checkout deployment)"}</dd>
      </dl>
      <h2 className="mb-2 text-base font-semibold">Catalogue</h2>
      <p className="mb-2 text-xs text-slate-500">Prices are snapshotted onto each order, so editing a product never changes an existing purchase. No selling price ships with this code: the operator inserts the real product (runbook: production setup → catalogue).</p>
      <Table head={["SKU", "Name", "Price", "Modules", "Active", "Created"]} empty={prods.length === 0}>
        {prods.map((p) => (
          <tr key={p.id}><Td mono>{p.id}</Td><Td>{p.name}</Td><Td>{inr(p.price_paise)} {p.currency}</Td><Td>{p.modules.join(", ")}</Td><Td><Badge value={p.active ? "active" : "cancelled"} /></Td><Td>{when(p.created_at)}</Td></tr>
        ))}
      </Table>
    </>
  );
}
