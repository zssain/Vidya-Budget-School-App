// Server-renderable UI primitives (no client JS).
import { formatInr } from "@vidya/billing-core";
import Link from "next/link";

export function inr(paise: string | number | null | undefined): string {
  if (paise === null || paise === undefined) return "—";
  return formatInr(Number(paise));
}

export function when(d: Date | string | null | undefined): string {
  if (!d) return "—";
  const x = typeof d === "string" ? new Date(d) : d;
  return x.toISOString().replace("T", " ").slice(0, 16) + " UTC";
}

const TONES: Record<string, string> = {
  paid: "bg-emerald-100 text-emerald-800",
  captured: "bg-emerald-100 text-emerald-800",
  issued: "bg-emerald-100 text-emerald-800",
  sent: "bg-emerald-100 text-emerald-800",
  active: "bg-emerald-100 text-emerald-800",
  done: "bg-emerald-100 text-emerald-800",
  pending: "bg-amber-100 text-amber-800",
  created: "bg-slate-100 text-slate-700",
  authorized: "bg-amber-100 text-amber-800",
  ready: "bg-amber-100 text-amber-800",
  running: "bg-sky-100 text-sky-800",
  paid_awaiting_machine_code: "bg-amber-100 text-amber-800",
  refunded: "bg-rose-100 text-rose-800",
  partially_refunded: "bg-orange-100 text-orange-800",
  disputed: "bg-rose-100 text-rose-800",
  failed: "bg-rose-100 text-rose-800",
  dead: "bg-rose-100 text-rose-800",
  cancelled: "bg-slate-200 text-slate-600",
  superseded: "bg-slate-200 text-slate-600",
};

export function Badge({ value }: { value: string | null | undefined }) {
  const v = value ?? "—";
  return (
    <span className={`inline-block rounded px-1.5 py-0.5 text-xs font-medium ${TONES[v] ?? "bg-slate-100 text-slate-700"}`}>
      {v.replace(/_/g, " ")}
    </span>
  );
}

export function PageHeader({ title, subtitle, children }: { title: string; subtitle?: string; children?: React.ReactNode }) {
  return (
    <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 className="text-xl font-semibold">{title}</h1>
        {subtitle ? <p className="mt-1 text-sm text-slate-600">{subtitle}</p> : null}
      </div>
      <div className="flex items-center gap-2">{children}</div>
    </div>
  );
}

export function Card({ title, value, hint, tone }: { title: string; value: React.ReactNode; hint?: string; tone?: "warn" | "bad" }) {
  const ring = tone === "bad" ? "border-rose-300" : tone === "warn" ? "border-amber-300" : "border-slate-200";
  return (
    <div className={`rounded-lg border bg-white p-4 ${ring}`}>
      <div className="text-xs font-medium uppercase tracking-wide text-slate-500">{title}</div>
      <div className="mt-1 text-2xl font-semibold">{value}</div>
      {hint ? <div className="mt-1 text-xs text-slate-500">{hint}</div> : null}
    </div>
  );
}

export function Table({ head, children, empty }: { head: string[]; children: React.ReactNode; empty?: boolean }) {
  return (
    <div className="overflow-x-auto rounded-lg border border-slate-200 bg-white">
      <table className="min-w-full text-sm">
        <thead className="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
          <tr>{head.map((h) => <th key={h} className="whitespace-nowrap px-3 py-2 font-medium">{h}</th>)}</tr>
        </thead>
        <tbody className="divide-y divide-slate-100">
          {empty ? (
            <tr><td className="px-3 py-6 text-center text-slate-500" colSpan={head.length}>No records.</td></tr>
          ) : children}
        </tbody>
      </table>
    </div>
  );
}

export function Td({ children, mono, className }: { children: React.ReactNode; mono?: boolean; className?: string }) {
  return <td className={`whitespace-nowrap px-3 py-2 align-top ${mono ? "mono text-xs" : ""} ${className ?? ""}`}>{children}</td>;
}

/** GET search/filter form — keeps state in the URL so pages stay server-rendered. */
export function SearchBar({ q, status, statuses, exportHref }: { q?: string; status?: string; statuses?: string[]; exportHref?: string }) {
  return (
    <form className="mb-4 flex flex-wrap items-end gap-2" method="get">
      <div className="min-w-64 flex-1">
        <label className="label" htmlFor="q">Search</label>
        <input id="q" name="q" defaultValue={q} className="input" placeholder="School, email, phone, order / payment / licence id, machine code" />
      </div>
      {statuses ? (
        <div>
          <label className="label" htmlFor="status">Status</label>
          <select id="status" name="status" defaultValue={status ?? ""} className="input">
            <option value="">All</option>
            {statuses.map((s) => <option key={s} value={s}>{s.replace(/_/g, " ")}</option>)}
          </select>
        </div>
      ) : null}
      <button className="btn-secondary" type="submit">Apply</button>
      {exportHref ? <a className="btn-secondary" href={exportHref}>Export CSV</a> : null}
    </form>
  );
}

export function Pager({ page, total, pageSize, base }: { page: number; total: number; pageSize: number; base: string }) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const link = (p: number) => {
    const u = new URL(base, "http://x");
    u.searchParams.set("page", String(p));
    return `${u.pathname}${u.search}`;
  };
  return (
    <div className="mt-3 flex items-center justify-between text-sm text-slate-600">
      <span>{total} record{total === 1 ? "" : "s"} · page {page} of {pages}</span>
      <span className="flex gap-2">
        {page > 1 ? <Link className="btn-secondary" href={link(page - 1)}>Previous</Link> : null}
        {page < pages ? <Link className="btn-secondary" href={link(page + 1)}>Next</Link> : null}
      </span>
    </div>
  );
}

export function Notice({ sp }: { sp: Record<string, string | string[] | undefined> }) {
  const err = typeof sp.error === "string" ? sp.error : null;
  const done = typeof sp.done === "string" ? sp.done : null;
  if (err) return <div role="alert" className="mb-4 rounded-md border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">Action refused: {err.replace(/_/g, " ")}</div>;
  if (done) return <div role="status" className="mb-4 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-800">Done: {done.replace(/_/g, " ")}</div>;
  return null;
}

/** An audited owner action: POSTs to /api/owner/actions (same-origin, CSRF-checked). */
export function ActionForm({
  action, returnTo, fields, children, label, danger, needsReason = true, note,
}: {
  action: string;
  returnTo: string;
  fields?: Record<string, string>;
  children?: React.ReactNode;
  label: string;
  danger?: boolean;
  needsReason?: boolean;
  note?: string;
}) {
  return (
    <form method="post" action="/api/owner/actions" className="space-y-2 rounded-md border border-slate-200 bg-white p-3">
      <input type="hidden" name="action" value={action} />
      <input type="hidden" name="returnTo" value={returnTo} />
      {Object.entries(fields ?? {}).map(([k, v]) => <input key={k} type="hidden" name={k} value={v} />)}
      {children}
      {needsReason ? (
        <div>
          <label className="label" htmlFor={`${action}-reason`}>Reason (recorded in the audit log)</label>
          <input id={`${action}-reason`} name={action === "acknowledge_review" ? "note" : "reason"} required minLength={5} maxLength={1000} className="input" />
        </div>
      ) : null}
      {note ? <p className="text-xs text-slate-500">{note}</p> : null}
      <button type="submit" className={danger ? "btn-danger" : "btn"}>{label}</button>
    </form>
  );
}
