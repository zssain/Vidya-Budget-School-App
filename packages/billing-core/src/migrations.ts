// Ordered, checksum-tracked application of infra/licensing/migrations/*.sql.
// Runs as the database owner/admin (never as an application role). Applied files
// are recorded in public.vidya_billing_migrations; an edited, already-applied file
// is a hard error (migrations are append-only — add 000N_*.sql instead).
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Queryable } from "./db";

export const DEFAULT_MIGRATIONS_DIR = resolve(
  fileURLToPath(new URL(".", import.meta.url)),
  "../../../infra/licensing/migrations",
);

export interface MigrationFile {
  name: string;
  sql: string;
  sha256: string;
}

export function readMigrations(dir = DEFAULT_MIGRATIONS_DIR): MigrationFile[] {
  return readdirSync(dir)
    .filter((f) => /^\d{4}_[a-z0-9_]+\.sql$/.test(f))
    .sort()
    .map((name) => {
      const sql = readFileSync(join(dir, name), "utf8");
      return { name, sql, sha256: createHash("sha256").update(sql).digest("hex") };
    });
}

export async function applyMigrations(q: Queryable, dir = DEFAULT_MIGRATIONS_DIR): Promise<string[]> {
  await q.query(`CREATE TABLE IF NOT EXISTS public.vidya_billing_migrations (
    filename text PRIMARY KEY, sha256 text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now())`);
  const done = new Map(
    (await q.query<{ filename: string; sha256: string }>(`SELECT filename, sha256 FROM public.vidya_billing_migrations`)).rows.map(
      (r) => [r.filename, r.sha256],
    ),
  );
  const applied: string[] = [];
  for (const m of readMigrations(dir)) {
    const prev = done.get(m.name);
    if (prev) {
      if (prev !== m.sha256) throw new Error(`migration ${m.name} was edited after being applied`);
      continue;
    }
    await q.query("BEGIN");
    try {
      await q.query(m.sql);
      await q.query(`INSERT INTO public.vidya_billing_migrations (filename, sha256) VALUES ($1, $2)`, [m.name, m.sha256]);
      await q.query("COMMIT");
    } catch (e) {
      await q.query("ROLLBACK");
      throw e;
    }
    await q.query("RESET search_path");
    applied.push(m.name);
  }
  return applied;
}
