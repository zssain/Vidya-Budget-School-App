// requireOwner decision matrix over a mocked official Supabase server client.
import { describe, expect, it } from "vitest";
import { evaluateOwner } from "@/lib/owner-guard";
import { fakeSupabase, IDENTITIES, OWNER_ID } from "./fake-supabase";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const run = (k: keyof typeof IDENTITIES, opts = {}, owner: string | null = OWNER_ID) => evaluateOwner(fakeSupabase(IDENTITIES[k]!) as any, owner, opts);

describe("requireOwner matrix", () => {
  it.each([
    ["unauthenticated", "unauthenticated"],
    ["nonOwnerAal2", "not_owner"],
    ["ownerAal1", "mfa_required"],
    ["forgedMetadata", "not_owner"],
    ["forgedClaims", "unauthenticated"],
  ] as const)("%s → denied (%s)", async (k, reason) => {
    expect(await run(k)).toEqual({ ok: false, reason });
  });

  it("AAL2 owner → allowed", async () => {
    const d = await run("ownerAal2");
    expect(d.ok).toBe(true);
    if (d.ok) expect(d.owner).toMatchObject({ id: OWNER_ID, aal: "aal2", hasVerifiedTotp: true });
  });

  it("AAL1 owner may reach ONLY the MFA flow (allowAal1)", async () => {
    expect((await run("ownerAal1", { allowAal1: true })).ok).toBe(true);
    // allowAal1 never admits a different user
    expect(await run("nonOwnerAal2", { allowAal1: true })).toEqual({ ok: false, reason: "not_owner" });
    expect(await run("forgedMetadata", { allowAal1: true })).toEqual({ ok: false, reason: "not_owner" });
  });

  it("step-up: fresh TOTP passes, stale TOTP / AAL1 → step_up_required", async () => {
    expect((await run("ownerAal2", { freshSeconds: 300 })).ok).toBe(true);
    expect(await run("ownerAal2Stale", { freshSeconds: 300 })).toEqual({ ok: false, reason: "step_up_required" });
    expect(await run("ownerAal1", { allowAal1: true, freshSeconds: 300 })).toEqual({ ok: false, reason: "step_up_required" });
  });

  it("missing / malformed OWNER_AUTH_USER_ID denies everyone (fail closed)", async () => {
    expect(await run("ownerAal2", {}, null)).toEqual({ ok: false, reason: "misconfigured" });
    expect(await run("ownerAal2", {}, "owner@example.test")).toEqual({ ok: false, reason: "misconfigured" });
  });

  it("an SDK exception denies (fail closed)", async () => {
    const boom = { auth: { getUser: async () => { throw new Error("network"); }, getClaims: async () => ({}) } };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect(await evaluateOwner(boom as any, OWNER_ID)).toEqual({ ok: false, reason: "unauthenticated" });
  });
});
