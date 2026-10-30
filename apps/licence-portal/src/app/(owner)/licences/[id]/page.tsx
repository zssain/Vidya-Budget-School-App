import Link from "next/link";
import { notFound } from "next/navigation";
import { CopyButton } from "@/components/client";
import { ActionForm, Badge, inr, Notice, PageHeader, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { licenceDetail } from "@/lib/queries";

function grouped(key: string) {
  return key.replace(/(.{5})/g, "$1 ").trim();
}

export default async function LicencePage({ params, searchParams }: { params: Promise<{ id: string }>; searchParams: Promise<Record<string, string | undefined>> }) {
  const { id } = await params;
  const back = `/licences/${id}`;
  await requireOwnerPage(back);
  if (!/^[0-9a-f-]{36}$/i.test(id)) notFound();
  const sp = await searchParams;
  const d = await licenceDetail(db(), id);
  if (!d) notFound();
  const l = d.lic;
  const issued = l.status === "issued";
  const isLatest = !l.superseded;
  return (
    <>
      <PageHeader title={l.licence_id} subtitle={`${l.school_name} · generation ${l.generation}`} />
      <Notice sp={sp} />
      <section className="mb-6 grid max-w-4xl grid-cols-2 gap-3 md:grid-cols-4">
        <div className="rounded-md border bg-white p-3"><div className="label">Commercial status</div><Badge value={l.commercial_status} /></div>
        <div className="rounded-md border bg-white p-3"><div className="label">Issuance status</div><Badge value={l.superseded ? "superseded" : l.status} /></div>
        <div className="rounded-md border bg-white p-3"><div className="label">Delivery</div><Badge value={d.deliveries.at(-1)?.state ?? "not sent"} /></div>
        <div className="rounded-md border bg-white p-3"><div className="label">Observed activation</div><span className="text-sm text-slate-600">unknown / not reported</span></div>
      </section>
      {l.superseded ? (
        <p className="mb-4 max-w-3xl rounded-md bg-amber-50 p-3 text-sm text-amber-900">
          Superseded by a later generation (transfer or re-issue). This is a commercial record only: an offline PC that already has this
          perpetual licence can still verify it. Remote revocation is not part of this licensing model.
        </p>
      ) : null}
      {l.commercial_status !== "active" ? (
        <p className="mb-4 max-w-3xl rounded-md bg-rose-50 p-3 text-sm text-rose-900">
          This purchase is <b>{l.commercial_status}</b>. Transfers and re-issues are blocked. The issued licence was NOT remotely disabled — that is
          not possible for an offline perpetual licence.
        </p>
      ) : null}

      <dl className="mb-6 grid max-w-3xl grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <dt className="text-slate-500">Bound machine</dt><dd className="mono">{l.machine_code}</dd>
        <dt className="text-slate-500">Modules</dt><dd>{l.modules.join(", ")}</dd>
        <dt className="text-slate-500">issued_at (signed)</dt><dd className="mono text-xs">{l.issued_at}</dd>
        <dt className="text-slate-500">Signed at</dt><dd>{when(l.issued_ts)} · key {l.signer_key_id ?? "—"}</dd>
        <dt className="text-slate-500">Entitlement source</dt><dd>{l.source === "owner_manual" ? <Badge value="MANUAL approval" /> : "verified payment"}</dd>
        <dt className="text-slate-500">Order</dt><dd><Link className="underline" href={`/orders/${l.order_id}`}>{l.order_id.slice(0, 8)}</Link> · {l.provider_order_id ?? "manual"} · {inr(l.amount_paise)} · <Badge value={l.order_status} /></dd>
        <dt className="text-slate-500">Purchaser</dt><dd>{l.email} · {l.phone ?? ""}</dd>
        {l.last_error ? (<><dt className="text-slate-500">Signer error</dt><dd className="text-rose-700">{l.last_error}</dd></>) : null}
      </dl>

      <h2 className="mb-2 text-base font-semibold">Issued artefact (exact stored bytes)</h2>
      {issued && l.licence_key ? (
        <div className="mb-6 max-w-3xl space-y-2">
          <pre className="mono whitespace-pre-wrap break-all rounded-md border bg-white p-3 text-xs">{grouped(l.licence_key)}</pre>
          <div className="flex gap-2">
            <CopyButton text={l.licence_key} label="Copy licence key" />
            <a className="btn-secondary" href={`/api/owner/licences/${l.id}/vlic`}>Download .vlic</a>
          </div>
        </div>
      ) : (
        <p className="mb-6 text-sm text-slate-600">{l.status === "failed" ? "The signer could not issue this licence." : "Payment received; preparing licence (the signer has not produced the artefact yet)."}</p>
      )}

      <h2 className="mb-2 text-base font-semibold">Owner actions</h2>
      <div className="mb-6 grid max-w-4xl gap-3 md:grid-cols-2">
        {issued ? (
          <ActionForm action="resend" returnTo={back} fields={{ issuanceId: l.id }} label="Resend existing licence"
            note="Emails the SAME stored artefact again. It never issues a new licence." />
        ) : null}
        {issued && isLatest && l.commercial_status === "active" ? (
          <ActionForm action="transfer" returnTo={back} fields={{ issuanceId: l.id }} label="Transfer to a new machine" danger
            note="Creates a new generation bound to the new machine code. The old PC keeps verifying its licence offline — this is not a remote disable. Requires a fresh MFA code.">
            <div><label className="label" htmlFor="newMachineCode">New machine code</label><input id="newMachineCode" name="newMachineCode" required className="input mono" placeholder="XXXX-XXXX-XXXX-C" /></div>
          </ActionForm>
        ) : null}
        {l.status === "failed" && isLatest && l.commercial_status === "active" ? (
          <ActionForm action="reissue_failed" returnTo={back} fields={{ issuanceId: l.id }} label="Re-issue (new generation)"
            note="Creates generation n+1 with a new licence ID for the same machine. Requires a fresh MFA code." />
        ) : null}
      </div>

      <h2 className="mb-2 text-base font-semibold">Generations / module history</h2>
      <Table head={["Gen", "Licence", "Machine", "Status", "Signed"]}>
        {d.generations.map((g) => (
          <tr key={g.id}><Td>{g.generation}</Td><Td mono><Link className="underline" href={`/licences/${g.id}`}>{g.licence_id}</Link></Td><Td mono>{g.machine_code}</Td><Td><Badge value={g.status} /></Td><Td>{when(g.issued_ts)}</Td></tr>
        ))}
      </Table>
      <p className="mt-1 text-xs text-slate-500">Modules are fixed per purchase ({l.modules.join(", ")}); every generation carries the same module set.</p>

      <h2 className="mb-2 mt-6 text-base font-semibold">Transfers</h2>
      <Table head={["Approved", "From gen", "To machine", "Replacement", "Reason"]} empty={d.transfers.length === 0}>
        {d.transfers.map((t) => (
          <tr key={t.id}><Td>{when(t.approved_at)}</Td><Td>{t.source_generation}</Td><Td mono>{t.target_machine_code}</Td><Td><Badge value={t.replacement_status} /> gen {t.replacement_generation}</Td><Td className="whitespace-normal">{t.reason}</Td></tr>
        ))}
      </Table>

      <h2 className="mb-2 mt-6 text-base font-semibold">Payments</h2>
      <Table head={["Payment", "Amount", "State", "Captured"]} empty={d.payments.length === 0}>
        {d.payments.map((p) => (<tr key={p.provider_payment_id}><Td mono>{p.provider_payment_id}</Td><Td>{inr(p.amount_paise)}</Td><Td><Badge value={p.state} /></Td><Td>{when(p.captured_at)}</Td></tr>))}
      </Table>

      <h2 className="mb-2 mt-6 text-base font-semibold">Delivery attempts</h2>
      <Table head={["Updated", "Key", "State", "Attempts", "Provider message", "Last error"]} empty={d.deliveries.length === 0}>
        {d.deliveries.map((x) => (<tr key={x.idempotency_key}><Td>{when(x.updated_at)}</Td><Td mono>{x.idempotency_key.split(":")[0]}</Td><Td><Badge value={x.state} /></Td><Td>{x.attempts}</Td><Td mono>{x.provider_message_id ?? "—"}</Td><Td className="whitespace-normal text-xs">{x.last_error}</Td></tr>))}
      </Table>

      <h2 className="mb-2 mt-6 text-base font-semibold">Audit timeline</h2>
      <Table head={["When", "Actor", "Action", "Reason"]} empty={d.audit.length === 0}>
        {d.audit.map((a) => (<tr key={a.id}><Td>{when(a.at)}</Td><Td mono>{a.actor}</Td><Td>{a.action}</Td><Td className="whitespace-normal">{a.reason}</Td></tr>))}
      </Table>
    </>
  );
}
