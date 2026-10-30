import Link from "next/link";
import { AutoRefresh } from "@/components/client";
import { ActionForm, Badge, Card, inr, PageHeader, Table, Td, when } from "@/components/ui";
import { db } from "@/lib/db";
import { requireOwnerPage } from "@/lib/owner-guard";
import { dashboard, failedJobs, openReviews } from "@/lib/queries";

const RANGES = [7, 30, 90, 365];

function stale(d: Date | null) {
  return !d || Date.now() - new Date(d).getTime() > 5 * 60_000;
}

export default async function DashboardPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  await requireOwnerPage("/dashboard");
  const sp = await searchParams;
  const days = RANGES.includes(Number(sp.days)) ? Number(sp.days) : 30;
  const pool = db();
  const [m, reviews, jobs] = await Promise.all([dashboard(pool, days), openReviews(pool), failedJobs(pool)]);
  return (
    <>
      <PageHeader title="Dashboard" subtitle={`Amounts in INR (₹), from integer paise. Range: last ${days} days, by capture time (UTC).`}>
        <AutoRefresh />
        <form method="get" className="flex gap-2">
          <select name="days" defaultValue={String(days)} className="input" aria-label="Date range">
            {RANGES.map((r) => <option key={r} value={r}>Last {r} days</option>)}
          </select>
          <button type="submit" className="btn-secondary">Apply</button>
        </form>
      </PageHeader>
      <section className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Card title="Verified captured sales" value={inr(m.capturedPaise)} hint={`${m.capturedCount} provider-verified captures`} />
        <Card title="Refunds" value={inr(m.refundedPaise)} hint="on payments captured in range" tone={m.refundedPaise > 0 ? "warn" : undefined} />
        <Card title="Net collected" value={inr(m.netPaise)} hint="captured − refunded" />
        <Card title="Manual approvals" value={m.manualApprovals} hint="owner-approved, NOT provider-verified; excluded from sales" />
        <Card title="Paid, awaiting issuance" value={m.paidAwaitingIssuance} hint={`+ ${m.awaitingMachineCode} awaiting machine code`} tone={m.paidAwaitingIssuance + m.awaitingMachineCode > 0 ? "warn" : undefined} />
        <Card title="Issued licences" value={m.issuedLicences} hint={`${m.issuedLicencesInRange} in range · issued ≠ installed/active (offline licences report nothing)`} />
        <Card title="Delivery failures" value={m.deliveryFailures} tone={m.deliveryFailures ? "bad" : undefined} />
        <Card title="Pending transfers" value={m.pendingTransfers} />
        <Card title="Failed / retrying jobs" value={`${m.deadJobs} / ${m.retryingJobs}`} hint="dead-lettered / waiting to retry" tone={m.deadJobs ? "bad" : m.retryingJobs ? "warn" : undefined} />
        <Card title="Needs review" value={m.openReviews} hint="refunds, disputes, rejected payments" tone={m.openReviews ? "warn" : undefined} />
        <Card title="Billing worker" value={stale(m.workerLastSeen) ? "not seen" : "running"} hint={`last heartbeat ${when(m.workerLastSeen)}`} tone={stale(m.workerLastSeen) ? "bad" : undefined} />
        <Card title="Licence signer" value={m.signerLastSeen ? (stale(m.signerLastSeen) ? "not seen" : "running") : "no heartbeat"} hint={m.signerLastSeen ? `last heartbeat ${when(m.signerLastSeen)}` : "the signer does not report a heartbeat; check its job queue"} />
      </section>

      <h2 className="mb-2 mt-8 text-base font-semibold">Needs owner review</h2>
      <p className="mb-2 text-xs text-slate-500">An issued offline perpetual licence cannot be remotely disabled. Refunds/disputes are recorded and future transfers/re-issues are blocked; nothing is changed on the school PC.</p>
      <Table head={["When", "Kind", "Target", "Detail", ""]} empty={reviews.length === 0}>
        {reviews.map((r) => (
          <tr key={r.id}>
            <Td>{when(r.at)}</Td>
            <Td><Badge value={r.action.replace(/^review_/, "")} /></Td>
            <Td mono>{r.target}</Td>
            <Td className="whitespace-normal">{r.reason}</Td>
            <Td>
              <ActionForm action="acknowledge_review" returnTo="/dashboard" fields={{ auditId: r.id }} label="Acknowledge" />
            </Td>
          </tr>
        ))}
      </Table>

      <h2 className="mb-2 mt-8 text-base font-semibold">Failed / retrying jobs</h2>
      <Table head={["Created", "Kind", "State", "Attempts", "Next try", "Last error"]} empty={jobs.length === 0}>
        {jobs.map((j) => (
          <tr key={j.id}>
            <Td>{when(j.created_at)}</Td>
            <Td mono>{j.kind}</Td>
            <Td><Badge value={j.state} /></Td>
            <Td>{j.attempts}/{j.max_attempts}</Td>
            <Td>{j.state === "ready" ? when(j.available_at) : "—"}</Td>
            <Td className="max-w-md truncate whitespace-normal text-xs">{j.last_error}</Td>
          </tr>
        ))}
      </Table>
      <p className="mt-6 text-xs text-slate-500"><Link className="underline" href="/audit">Full audit log</Link></p>
    </>
  );
}
