// One pg Pool per server process, connected as the least-privilege vidya_portal
// login role (see runbooks). Never imported by client components.
import "server-only";
import type { PoolLike } from "@vidya/billing-core";
import pg from "pg";
import { databaseUrl } from "./env";

const g = globalThis as unknown as { __vidyaPortalPool?: pg.Pool };

let override: PoolLike | null = null;

/** Tests inject a pool bound to a throwaway database. */
export function setPoolForTests(p: PoolLike | null) {
  override = p;
}

export function db(): PoolLike {
  if (override) return override;
  if (!g.__vidyaPortalPool) {
    g.__vidyaPortalPool = new pg.Pool({ connectionString: databaseUrl(), max: 5, idleTimeoutMillis: 30_000 });
  }
  return g.__vidyaPortalPool as unknown as PoolLike;
}
