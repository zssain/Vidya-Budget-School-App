// A mocked Supabase server client with the exact method shapes requireOwner uses.
// Identities are synthetic; no real Supabase project is contacted.
export const OWNER_ID = "11111111-1111-4111-8111-111111111111";
export const OTHER_ID = "22222222-2222-4222-8222-222222222222";

export interface Identity {
  user: { id: string; user_metadata?: Record<string, unknown>; app_metadata?: Record<string, unknown>; factors?: { factor_type: string; status: string; id?: string }[] } | null;
  claims?: { sub: string; aal: string; amr?: unknown[]; session_id?: string; user_metadata?: Record<string, unknown>; app_metadata?: Record<string, unknown> } | null;
}

const now = () => Math.floor(Date.now() / 1000);
const totp = [{ factor_type: "totp", status: "verified", id: "33333333-3333-4333-8333-333333333333" }];

export const IDENTITIES: Record<string, Identity> = {
  unauthenticated: { user: null, claims: null },
  nonOwnerAal2: {
    user: { id: OTHER_ID, factors: totp },
    claims: { sub: OTHER_ID, aal: "aal2", amr: [{ method: "totp", timestamp: now() }] },
  },
  ownerAal1: {
    user: { id: OWNER_ID, factors: totp },
    claims: { sub: OWNER_ID, aal: "aal1", amr: [{ method: "password", timestamp: now() }] },
  },
  // A different user who set user_metadata/app_metadata claiming to be the owner.
  forgedMetadata: {
    user: { id: OTHER_ID, user_metadata: { role: "owner", is_owner: true, owner_id: OWNER_ID }, app_metadata: { role: "owner" }, factors: totp },
    claims: { sub: OTHER_ID, aal: "aal2", amr: [{ method: "totp", timestamp: now() }], user_metadata: { role: "owner" }, app_metadata: { role: "owner" } },
  },
  // JWT claims say "owner" but the Auth server says the session belongs to someone else.
  forgedClaims: {
    user: { id: OTHER_ID, factors: totp },
    claims: { sub: OWNER_ID, aal: "aal2", amr: [{ method: "totp", timestamp: now() }] },
  },
  ownerAal2: {
    user: { id: OWNER_ID, factors: totp },
    claims: { sub: OWNER_ID, aal: "aal2", amr: [{ method: "password", timestamp: now() - 30 }, { method: "totp", timestamp: now() - 20 }] },
  },
  ownerAal2Stale: {
    user: { id: OWNER_ID, factors: totp },
    claims: { sub: OWNER_ID, aal: "aal2", amr: [{ method: "password", timestamp: now() - 7200 }, { method: "totp", timestamp: now() - 7200 }] },
  },
};

export function fakeSupabase(id: Identity) {
  return {
    auth: {
      getUser: async () =>
        id.user ? { data: { user: id.user }, error: null } : { data: { user: null }, error: { name: "AuthSessionMissingError", message: "no session" } },
      getClaims: async () =>
        id.claims ? { data: { claims: id.claims, header: {}, signature: new Uint8Array() }, error: null } : { data: null, error: { message: "no session" } },
      signInWithPassword: async () => ({ data: { user: id.user }, error: id.user ? null : { message: "invalid" } }),
      signOut: async () => ({ error: null }),
      mfa: {
        listFactors: async () => ({ data: { all: id.user?.factors ?? [], totp: id.user?.factors ?? [] }, error: null }),
        challengeAndVerify: async ({ code }: { code: string }) => (code === "123456" ? { data: {}, error: null } : { data: null, error: { message: "bad code" } }),
        enroll: async () => ({ data: { id: "44444444-4444-4444-8444-444444444444", totp: { qr_code: "data:image/svg+xml;utf8,<svg/>", secret: "TESTSECRET", uri: "otpauth://x" } }, error: null }),
        unenroll: async () => ({ data: {}, error: null }),
      },
    },
  };
}
