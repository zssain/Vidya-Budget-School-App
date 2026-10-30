// CSV export (owner only). Cells are formula-injection-neutralised by billing-core.
import { toCsv } from "@vidya/billing-core";
import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { deny, json, NO_STORE } from "@/lib/http";
import { requireOwner } from "@/lib/owner-guard";
import {
  AUDIT_COLUMNS,
  DELIVERY_COLUMNS,
  LICENCE_COLUMNS,
  listAudit,
  listDeliveries,
  listLicences,
  listOrders,
  listPayments,
  listSchools,
  listTransfers,
  ORDER_COLUMNS,
  parseList,
  PAYMENT_COLUMNS,
  SCHOOL_COLUMNS,
  TRANSFER_COLUMNS,
} from "@/lib/queries";
import { surfaceAllows } from "@/lib/surface";

const MAX_ROWS = 10_000;

const TABLES = {
  orders: { cols: ORDER_COLUMNS, list: listOrders },
  payments: { cols: PAYMENT_COLUMNS, list: listPayments },
  licences: { cols: LICENCE_COLUMNS, list: listLicences },
  schools: { cols: SCHOOL_COLUMNS, list: listSchools },
  transfers: { cols: TRANSFER_COLUMNS, list: listTransfers },
  delivery: { cols: DELIVERY_COLUMNS, list: listDeliveries },
  audit: { cols: AUDIT_COLUMNS, list: listAudit },
} as const;

export async function GET(req: Request, ctx: { params: Promise<{ table: string }> }) {
  if (!surfaceAllows("owner")) return json({ error: "not_found" }, 404);
  const d = await requireOwner();
  if (!d.ok) return deny(d.reason);
  const { table } = await ctx.params;
  const spec = TABLES[table as keyof typeof TABLES];
  if (!spec) return json({ error: "unknown_table" }, 404);
  const url = new URL(req.url);
  const p = parseList(Object.fromEntries(url.searchParams.entries()));
  const rows = await spec.list(db(), { ...p, page: 1 }, MAX_ROWS);
  const body = toCsv(
    spec.cols,
    rows.map((r) => spec.cols.map((c) => {
      const v = (r as Record<string, unknown>)[c];
      return v !== null && typeof v === "object" && !(v instanceof Date) && !Array.isArray(v) ? JSON.stringify(v) : v;
    })),
  );
  const stamp = new Date().toISOString().slice(0, 10);
  return new NextResponse(body, {
    headers: {
      ...NO_STORE,
      "Content-Type": "text/csv; charset=utf-8",
      "Content-Disposition": `attachment; filename="vidya-${table}-${stamp}.csv"`,
    },
  });
}
