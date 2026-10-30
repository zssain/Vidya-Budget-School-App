// Fixed-window rate limiting persisted in Postgres (billing.auth_rate_limits,
// migration 0002) so it holds across serverless instances. Keys are hashed.
import type { Queryable } from "./db";
import { sha256Hex } from "./tokens";

export interface RateLimitRule {
  name: string; // e.g. 'login'
  limit: number;
  windowSeconds: number;
}

export const RULES = {
  loginIp: { name: "login:ip", limit: 10, windowSeconds: 15 * 60 },
  loginEmail: { name: "login:email", limit: 5, windowSeconds: 15 * 60 },
  mfaUser: { name: "mfa:user", limit: 6, windowSeconds: 5 * 60 },
  checkoutIp: { name: "checkout:ip", limit: 20, windowSeconds: 60 * 60 },
  statusIp: { name: "status:ip", limit: 120, windowSeconds: 60 * 60 },
} satisfies Record<string, RateLimitRule>;

/** Count one hit; returns whether it is allowed. */
export async function hitRateLimit(q: Queryable, rule: RateLimitRule, key: string): Promise<boolean> {
  const bucket = `${rule.name}:${sha256Hex(key.toLowerCase())}`;
  const r = await q.query<{ hits: number }>(
    `INSERT INTO billing.auth_rate_limits AS rl (bucket, window_start, hits)
     VALUES ($1, to_timestamp(floor(extract(epoch FROM now()) / $2::int) * $2::int), 1)
     ON CONFLICT (bucket, window_start) DO UPDATE SET hits = rl.hits + 1
     RETURNING hits`,
    [bucket, rule.windowSeconds],
  );
  return r.rows[0]!.hits <= rule.limit;
}

export async function purgeRateLimits(q: Queryable): Promise<void> {
  await q.query(`DELETE FROM billing.auth_rate_limits WHERE window_start < now() - interval '1 day'`);
}
