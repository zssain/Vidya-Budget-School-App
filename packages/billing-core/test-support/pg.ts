// Test-only helpers: real Postgres 16, one fresh database per test file cloned from
// a template that has every migration applied. Connections take on the real
// least-privilege application roles (vidya_worker / vidya_portal) via the `role`
// startup option, so grants and RLS are exercised exactly as in production.
// NEVER imported by production code.
import { createHash, randomBytes } from "node:crypto";
import { applyMigrations, readMigrations } from "../src/migrations";
import type { PoolLike, Queryable } from "../src/db";

// Structural view of the `pg` module so the caller's own pg install is used
// (avoids cross-install class identity issues).
interface ClientLike extends Queryable {
  connect(): Promise<unknown>;
  end(): Promise<void>;
}
export interface Pg {
  Client: new (cfg: { connectionString: string }) => ClientLike;
  Pool: new (cfg: { connectionString: string; max?: number }) => unknown;
}
export type EndablePool = PoolLike & { end(): Promise<void> };

export const ADMIN_URL = process.env.BILLING_TEST_ADMIN_URL ?? "postgres://postgres@localhost:55432/postgres";

function withDb(url: string, db: string, role?: string): string {
  const u = new URL(url);
  u.pathname = `/${db}`;
  if (role) u.searchParams.set("options", `-c role=${role}`);
  return u.toString();
}

export interface TestDb {
  name: string;
  /** Pool acting as an application role (default vidya_worker). */
  pool(role?: "vidya_worker" | "vidya_portal" | "vidya_signer"): EndablePool;
  /** Superuser pool for fixture setup / inspection only. */
  admin: EndablePool;
  drop(): Promise<void>;
}

export async function createTestDatabase(pg: Pg): Promise<TestDb> {
  const migs = readMigrations();
  const tplHash = createHash("sha256").update(migs.map((m) => m.sha256).join(",")).digest("hex").slice(0, 12);
  const tpl = `vidya_billing_tpl_${tplHash}`;
  const name = `vidya_billing_t_${randomBytes(5).toString("hex")}`;

  const admin = new pg.Client({ connectionString: ADMIN_URL });
  await admin.connect();
  try {
    await admin.query("SELECT pg_advisory_lock(7317001)");
    const exists = await admin.query("SELECT 1 FROM pg_database WHERE datname = $1", [tpl]);
    if (exists.rowCount === 0) {
      await admin.query(`CREATE DATABASE "${tpl}"`);
      const c = new pg.Client({ connectionString: withDb(ADMIN_URL, tpl) });
      await c.connect();
      try {
        await applyMigrations(c);
      } finally {
        await c.end();
      }
    }
    await admin.query(`CREATE DATABASE "${name}" TEMPLATE "${tpl}"`);
  } finally {
    await admin.query("SELECT pg_advisory_unlock(7317001)").catch(() => undefined);
    await admin.end();
  }

  const pools: EndablePool[] = [];
  const mk = (role?: string) => {
    const p = new pg.Pool({ connectionString: withDb(ADMIN_URL, name, role), max: 10 }) as EndablePool;
    pools.push(p);
    return p;
  };
  const adminPool = mk();
  const cache = new Map<string, EndablePool>();
  return {
    name,
    admin: adminPool,
    pool(role = "vidya_worker") {
      let p = cache.get(role);
      if (!p) {
        p = mk(role);
        cache.set(role, p);
      }
      return p;
    },
    async drop() {
      await Promise.all(pools.map((p) => p.end()));
      const a = new pg.Client({ connectionString: ADMIN_URL });
      await a.connect();
      try {
        await a.query(`DROP DATABASE IF EXISTS "${name}" WITH (FORCE)`);
      } finally {
        await a.end();
      }
    },
  };
}

/** Insert a fake test-only product. Price is a TEST FIXTURE, not a selling price. */
export async function seedTestProduct(
  admin: PoolLike,
  opts: { id?: string; pricePaise?: number; modules?: string[] } = {},
): Promise<{ id: string; pricePaise: number }> {
  const id = opts.id ?? "test-fixture-sku-v1";
  const pricePaise = opts.pricePaise ?? 123456;
  await admin.query(
    `INSERT INTO billing.products (id, name, currency, price_paise, modules)
     VALUES ($1, 'TEST FIXTURE product (not a real price)', 'INR', $2, $3::text[])
     ON CONFLICT (id) DO NOTHING`,
    [id, pricePaise, opts.modules ?? ["core"]],
  );
  return { id, pricePaise };
}
