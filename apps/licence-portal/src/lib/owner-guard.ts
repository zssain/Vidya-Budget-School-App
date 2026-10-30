// requireOwner(): the ONLY gate in front of owner data. Every owner page, route
// handler and server-side mutation calls it before touching the database. Proxy
// redirects and hidden navigation are conveniences, not access control.
//
// Decision procedure (all must pass):
//  1. supabase.auth.getUser()   — validates the session with the Supabase Auth
//                                  server (catches revoked/expired sessions).
//  2. supabase.auth.getClaims() — verifies the access-token JWT (JWKS or server
//                                  round trip) and yields aal / amr.
//  3. claims.sub === user.id === OWNER_AUTH_USER_ID (immutable Auth UUID). Email,
//     user_metadata and app_metadata are NEVER consulted — they are not an
//     authorization input, so forged "role: owner" metadata does nothing.
//  4. claims.aal === "aal2" (TOTP verified in this session) unless the caller is the
//     narrow MFA enrolment/verification flow (allowAal1).
//  5. Optional freshness: for sensitive actions the newest MFA (totp) amr entry must
//     be within `freshSeconds` (step-up re-verification).
import "server-only";
import type { SupabaseClient } from "@supabase/supabase-js";
import { redirect } from "next/navigation";
import { connection } from "next/server";
import { ownerEnv } from "./env";
import { createSupabaseServerClient } from "./supabase/server";

export type DenyReason = "unauthenticated" | "not_owner" | "mfa_required" | "step_up_required" | "misconfigured";

export interface Owner {
  id: string;
  sessionId: string | null;
  aal: string;
  /** unix seconds of the newest password / totp verification in this session */
  lastPasswordAt: number | null;
  lastMfaAt: number | null;
  hasVerifiedTotp: boolean;
}

export type OwnerDecision = { ok: true; owner: Owner } | { ok: false; reason: DenyReason };

export interface GuardOptions {
  allowAal1?: boolean;
  freshSeconds?: number;
  now?: () => number;
}

type AmrEntry = { method?: string; timestamp?: number } | string;

function newest(amr: AmrEntry[] | undefined, methods: string[]): number | null {
  let best: number | null = null;
  for (const e of amr ?? []) {
    if (typeof e === "string") continue; // RFC-8176 string form has no timestamp
    if (e.method && methods.includes(e.method) && typeof e.timestamp === "number") {
      best = best === null ? e.timestamp : Math.max(best, e.timestamp);
    }
  }
  return best;
}

/** Pure decision over an (official) Supabase server client — unit-tested with a mock. */
export async function evaluateOwner(
  supabase: Pick<SupabaseClient, "auth">,
  allowedOwnerId: string | null | undefined,
  opts: GuardOptions = {},
): Promise<OwnerDecision> {
  if (!allowedOwnerId || !/^[0-9a-f-]{36}$/i.test(allowedOwnerId)) return { ok: false, reason: "misconfigured" };
  let userId: string;
  let factors: { factor_type?: string; status?: string }[] = [];
  try {
    const { data, error } = await supabase.auth.getUser();
    if (error || !data?.user?.id) return { ok: false, reason: "unauthenticated" };
    userId = data.user.id;
    factors = (data.user.factors ?? []) as typeof factors;
  } catch {
    return { ok: false, reason: "unauthenticated" };
  }
  let claims: { sub?: string; aal?: string; amr?: AmrEntry[]; session_id?: string } | undefined;
  try {
    const { data, error } = await supabase.auth.getClaims();
    if (error || !data?.claims) return { ok: false, reason: "unauthenticated" };
    claims = data.claims as typeof claims;
  } catch {
    return { ok: false, reason: "unauthenticated" };
  }
  if (!claims?.sub || claims.sub !== userId) return { ok: false, reason: "unauthenticated" };
  if (userId.toLowerCase() !== allowedOwnerId.toLowerCase()) return { ok: false, reason: "not_owner" };

  const owner: Owner = {
    id: userId,
    sessionId: claims.session_id ?? null,
    aal: claims.aal ?? "aal1",
    lastPasswordAt: newest(claims.amr, ["password"]),
    lastMfaAt: newest(claims.amr, ["totp", "mfa/totp"]),
    hasVerifiedTotp: factors.some((f) => f.factor_type === "totp" && f.status === "verified"),
  };
  if (!opts.allowAal1 && owner.aal !== "aal2") return { ok: false, reason: "mfa_required" };
  if (opts.freshSeconds !== undefined) {
    const now = Math.floor((opts.now ?? Date.now)() / 1000);
    if (owner.aal !== "aal2" || owner.lastMfaAt === null || now - owner.lastMfaAt > opts.freshSeconds) {
      return { ok: false, reason: "step_up_required" };
    }
  }
  return { ok: true, owner };
}

/** For route handlers: returns the decision; the caller maps it to 401/403. */
export async function requireOwner(opts: GuardOptions = {}): Promise<OwnerDecision> {
  await connection(); // never evaluate at build/prerender time
  let ownerId: string;
  try {
    ownerId = ownerEnv().OWNER_AUTH_USER_ID;
  } catch {
    return { ok: false, reason: "misconfigured" };
  }
  const supabase = await createSupabaseServerClient();
  return evaluateOwner(supabase, ownerId, opts);
}

/** For pages: redirect away instead of rendering owner data. */
export async function requireOwnerPage(returnTo: string, opts: GuardOptions = {}): Promise<Owner> {
  const d = await requireOwner(opts);
  if (d.ok) return d.owner;
  const next = encodeURIComponent(returnTo);
  switch (d.reason) {
    case "mfa_required":
      redirect(`/mfa?next=${next}`);
    case "step_up_required":
      redirect(`/mfa?step_up=1&next=${next}`);
    case "not_owner":
      redirect(`/login?error=not_owner`);
    case "misconfigured":
      redirect(`/login?error=misconfigured`);
    default:
      redirect(`/login?next=${next}`);
  }
}

/** Seconds within which a sensitive owner action needs a fresh TOTP verification. */
export const STEP_UP_SECONDS = 5 * 60;
/** MFA enrolment at AAL1 requires a password sign-in this recent. */
export const ENROL_PASSWORD_MAX_AGE_SECONDS = 10 * 60;

/** Whether the owner signed in with their password recently enough to enrol MFA. */
export function passwordIsFresh(owner: Owner, nowMs = Date.now()): boolean {
  return owner.lastPasswordAt !== null && Math.floor(nowMs / 1000) - owner.lastPasswordAt <= ENROL_PASSWORD_MAX_AGE_SECONDS;
}
