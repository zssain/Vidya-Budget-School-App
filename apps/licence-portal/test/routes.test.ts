// Route-level authorization and payment-boundary tests: the REAL route handlers and
// page components, a mocked Supabase server client, and a real Postgres 16 database
// accessed through the least-privilege vidya_portal role.
import { randomBytes } from "node:crypto";
import pg from "pg";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  createCheckoutOrder,
  attachProviderOrder,
  signCheckout,
  signWebhookBody,
  type RazorpayApi,
} from "@vidya/billing-core";
import { createTestDatabase, seedTestProduct, type TestDb } from "@vidya/billing-core/test-support";
import { fakeSupabase, IDENTITIES, OWNER_ID, type Identity } from "./fake-supabase";

const h = vi.hoisted(() => ({ identity: null as unknown as Identity }));
vi.mock("@/lib/supabase/server", () => ({
  createSupabaseServerClient: async () => fakeSupabase(h.identity),
  OWNER_COOKIE_NAME: "vidya-owner",
  hardenCookie: (o: unknown) => o,
}));

// Route handlers are invoked directly (no Next request scope), so stub connection();
// request-time rendering in production is enforced by force-dynamic (see build output).
vi.mock("next/server", async (orig) => ({ ...(await orig<typeof import("next/server")>()), connection: async () => undefined }));

import { setPoolForTests } from "@/lib/db";
import { setRazorpayForTests } from "@/lib/razorpay";
import { GET as exportGET } from "@/app/api/owner/export/[table]/route";
import { GET as vlicGET } from "@/app/api/owner/licences/[id]/vlic/route";
import { POST as actionsPOST } from "@/app/api/owner/actions/route";
import { POST as webhookPOST } from "@/app/api/webhooks/razorpay/route";
import { POST as ordersPOST } from "@/app/api/checkout/orders/route";
import { POST as verifyPOST } from "@/app/api/checkout/verify/route";
import { GET as licenceGET } from "@/app/api/checkout/licence/[token]/route";
import { POST as mfaVerifyPOST } from "@/app/api/auth/mfa/verify/route";
import { POST as mfaEnrollPOST } from "@/app/api/auth/mfa/enroll/route";
import DashboardPage from "@/app/(owner)/dashboard/page";
import OrdersPage from "@/app/(owner)/orders/page";
import LicencePage from "@/app/(owner)/licences/[id]/page";

const OWNER_ORIGIN = "https://owner.portal.test";
const CHECKOUT_ORIGIN = "https://buy.portal.test";
const WEBHOOK_SECRET = "test-webhook-secret-not-real";
const KEY_SECRET = "test-key-secret-not-real";
const MC = "R69C-090M-XG7X-1";

let tdb: TestDb;
let productId: string;
let pricePaise: number;

class FakeRzp implements RazorpayApi {
  created: unknown[] = [];
  async fetchOrderPayments() { return []; }
  async createOrder(input: { amount: number }) {
    this.created.push(input);
    return { id: `order_T${randomBytes(6).toString("hex")}` };
  }
}
const rzp = new FakeRzp();

beforeAll(async () => {
  Object.assign(process.env, {
    PORTAL_SURFACE: "both",
    OWNER_ORIGIN,
    CHECKOUT_ORIGIN,
    SUPABASE_URL: "https://project-ref.supabase.test",
    SUPABASE_PUBLISHABLE_KEY: "sb_publishable_TESTFIXTURE",
    OWNER_AUTH_USER_ID: OWNER_ID,
    PORTAL_DATABASE_URL: "postgres://unused-in-tests",
    RAZORPAY_MODE: "test",
    RAZORPAY_KEY_ID: "rzp_test_FIXTURE",
    RAZORPAY_KEY_SECRET: KEY_SECRET,
    RAZORPAY_WEBHOOK_SECRET: WEBHOOK_SECRET,
  });
  tdb = await createTestDatabase(pg);
  ({ id: productId, pricePaise } = await seedTestProduct(tdb.admin));
  setPoolForTests(tdb.pool("vidya_portal"));
  setRazorpayForTests(rzp);
});
afterAll(async () => {
  setPoolForTests(null);
  await tdb?.drop();
});
beforeEach(() => {
  h.identity = IDENTITIES.unauthenticated!;
  process.env.PORTAL_SURFACE = "both";
});

const DENIED = ["unauthenticated", "nonOwnerAal2", "ownerAal1", "forgedMetadata", "forgedClaims"] as const;

function form(fields: Record<string, string>, origin = OWNER_ORIGIN) {
  const fd = new FormData();
  for (const [k, v] of Object.entries(fields)) fd.set(k, v);
  return new Request(`${OWNER_ORIGIN}/api/owner/actions`, { method: "POST", headers: { origin }, body: fd });
}

async function seededIssuedLicence() {
  const portal = tdb.pool("vidya_portal");
  const o = await createCheckoutOrder(portal, {
    productId, customerName: "Test Buyer", email: "buyer@example.test", phone: null, schoolName: "Fake Route School", machineCode: MC, environment: "test",
  });
  const providerOrderId = `order_T${randomBytes(6).toString("hex")}`;
  await attachProviderOrder(portal, o.orderId, providerOrderId);
  // provision via the admin connection (equivalent of a verified capture by the worker)
  const ent = await tdb.admin.query(
    `INSERT INTO billing.entitlements (order_id, modules, machine_code, school_name, source) VALUES ($1, '{core}', $2, 'Fake Route School', 'verified_payment') RETURNING id`,
    [o.orderId, MC],
  );
  const iss = await tdb.admin.query(
    `INSERT INTO billing.licence_issuances (entitlement_id, generation, licence_id, machine_code, issued_at, status, licence_key, vlic)
     VALUES ($1, 1, 'VID-' || gen_random_uuid(), $2, '2026-10-31T00:00:00Z', 'issued', 'TESTKEY', 'TEST-VLIC') RETURNING id`,
    [ent.rows[0].id, MC],
  );
  return { ...o, providerOrderId, issuanceId: iss.rows[0].id as string };
}

describe("owner API routes: only the configured AAL2 owner", () => {
  it.each(DENIED)("export CSV denied for %s", async (k) => {
    h.identity = IDENTITIES[k]!;
    const r = await exportGET(new Request(`${OWNER_ORIGIN}/api/owner/export/orders`), { params: Promise.resolve({ table: "orders" }) });
    expect([401, 403]).toContain(r.status);
    expect(await r.text()).not.toContain("school_name");
  });

  it("export CSV allowed for the AAL2 owner, no-store, formula-safe", async () => {
    await createCheckoutOrder(tdb.pool("vidya_portal"), {
      productId, customerName: "=HYPERLINK(\"http://evil\")", email: "evil@example.test", phone: "+911234567", schoolName: "@SUM(A1)", machineCode: MC, environment: "test",
    });
    h.identity = IDENTITIES.ownerAal2!;
    const r = await exportGET(new Request(`${OWNER_ORIGIN}/api/owner/export/orders`), { params: Promise.resolve({ table: "orders" }) });
    expect(r.status).toBe(200);
    expect(r.headers.get("cache-control")).toContain("no-store");
    const csv = await r.text();
    expect(csv).toContain(`"'@SUM(A1)"`);
    expect(csv).toContain(`"'=HYPERLINK(""http://evil"")"`);
    expect(csv).toContain(`"'+911234567"`);
  });

  it.each(DENIED)(".vlic download denied for %s", async (k) => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES[k]!;
    const r = await vlicGET(new Request(`${OWNER_ORIGIN}/x`), { params: Promise.resolve({ id: issuanceId }) });
    expect([401, 403]).toContain(r.status);
  });

  it(".vlic download returns the exact stored artefact to the AAL2 owner", async () => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES.ownerAal2!;
    const r = await vlicGET(new Request(`${OWNER_ORIGIN}/x`), { params: Promise.resolve({ id: issuanceId }) });
    expect(r.status).toBe(200);
    expect(await r.text()).toBe("TEST-VLIC");
  });

  it.each(DENIED)("owner mutation denied for %s (nothing written)", async (k) => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES[k]!;
    const r = await actionsPOST(form({ action: "resend", issuanceId, reason: "should not happen", returnTo: "/licences" }));
    expect([401, 403]).toContain(r.status);
    const j = await tdb.admin.query(`SELECT count(*)::int AS n FROM billing.jobs WHERE payload->>'issuance_id' = $1 AND kind = 'deliver_licence'`, [issuanceId]);
    expect(j.rows[0].n).toBe(0);
  });

  it("owner mutation from a foreign Origin is refused even for the AAL2 owner (CSRF)", async () => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES.ownerAal2!;
    const r = await actionsPOST(form({ action: "resend", issuanceId, reason: "csrf attempt", returnTo: "/licences" }, "https://evil.example"));
    expect(r.status).toBe(403);
    const r2 = await actionsPOST(new Request(`${OWNER_ORIGIN}/api/owner/actions`, { method: "POST", headers: { "sec-fetch-site": "cross-site" }, body: new FormData() }));
    expect(r2.status).toBe(403);
  });

  it("AAL2 owner resend works and redirects only to allowlisted paths", async () => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES.ownerAal2!;
    const r = await actionsPOST(form({ action: "resend", issuanceId, reason: "customer lost the email", returnTo: "https://evil.example/" }));
    expect(r.status).toBe(303);
    expect(r.headers.get("location")).toBe(`${OWNER_ORIGIN}/dashboard?done=resend`);
    const j = await tdb.admin.query(`SELECT count(*)::int AS n FROM billing.jobs WHERE payload->>'issuance_id' = $1 AND kind = 'deliver_licence'`, [issuanceId]);
    expect(j.rows[0].n).toBe(1);
  });

  it("sensitive action (transfer) with stale MFA → step-up redirect, nothing written", async () => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES.ownerAal2Stale!;
    const r = await actionsPOST(form({ action: "transfer", issuanceId, newMachineCode: "RD9F-FC95-VGVQ-3", reason: "new PC", returnTo: `/licences/${issuanceId}` }));
    expect(r.status).toBe(303);
    expect(r.headers.get("location")).toContain("/mfa?step_up=1");
    const t = await tdb.admin.query(`SELECT count(*)::int AS n FROM billing.licence_transfers WHERE source_issuance_id = $1`, [issuanceId]);
    expect(t.rows[0].n).toBe(0);
    h.identity = IDENTITIES.ownerAal2!;
    const ok = await actionsPOST(form({ action: "transfer", issuanceId, newMachineCode: "RD9F-FC95-VGVQ-3", reason: "new PC", returnTo: `/licences/${issuanceId}` }));
    expect(ok.status).toBe(303);
    expect(ok.headers.get("location")).toMatch(/\/licences\/[0-9a-f-]{36}\?done=transfer$/);
  });
});

describe("owner pages: server components refuse to render owner data", () => {
  async function digest(p: Promise<unknown>) {
    try {
      await p;
      return "rendered";
    } catch (e) {
      return String((e as { digest?: string }).digest ?? e);
    }
  }
  it.each([
    ["unauthenticated", "/login?next="],
    ["nonOwnerAal2", "/login?error=not_owner"],
    ["ownerAal1", "/mfa?next="],
    ["forgedMetadata", "/login?error=not_owner"],
    ["forgedClaims", "/login?next="],
  ] as const)("dashboard for %s → redirect %s", async (k, to) => {
    h.identity = IDENTITIES[k]!;
    const d = await digest(DashboardPage({ searchParams: Promise.resolve({}) }));
    expect(d).toContain("NEXT_REDIRECT");
    expect(d).toContain(to);
  });

  it("orders + licence pages also redirect when denied, render for the AAL2 owner", async () => {
    const { issuanceId } = await seededIssuedLicence();
    h.identity = IDENTITIES.ownerAal1!;
    expect(await digest(OrdersPage({ searchParams: Promise.resolve({}) }))).toContain("NEXT_REDIRECT");
    expect(await digest(LicencePage({ params: Promise.resolve({ id: issuanceId }), searchParams: Promise.resolve({}) }))).toContain("NEXT_REDIRECT");
    h.identity = IDENTITIES.ownerAal2!;
    expect(await digest(DashboardPage({ searchParams: Promise.resolve({}) }))).toBe("rendered");
    expect(await digest(OrdersPage({ searchParams: Promise.resolve({}) }))).toBe("rendered");
    expect(await digest(LicencePage({ params: Promise.resolve({ id: issuanceId }), searchParams: Promise.resolve({}) }))).toBe("rendered");
  });
});

describe("MFA flow at AAL1 is owner-only", () => {
  const json = (body: unknown, origin = OWNER_ORIGIN) =>
    new Request(`${OWNER_ORIGIN}/api/auth/mfa/verify`, { method: "POST", headers: { origin, "content-type": "application/json" }, body: JSON.stringify(body) });
  it("non-owner / forged identities cannot verify or enrol", async () => {
    for (const k of ["unauthenticated", "nonOwnerAal2", "forgedMetadata", "forgedClaims"] as const) {
      h.identity = IDENTITIES[k]!;
      expect([401, 403]).toContain((await mfaVerifyPOST(json({ code: "123456" }))).status);
      expect([401, 403]).toContain((await mfaEnrollPOST(json({}))).status);
    }
  });
  it("AAL1 owner can verify a TOTP; a wrong code is refused; next is allowlisted", async () => {
    h.identity = IDENTITIES.ownerAal1!;
    expect((await mfaVerifyPOST(json({ code: "000000" }))).status).toBe(400);
    const ok = await mfaVerifyPOST(json({ code: "123456", next: "//evil.example" }));
    expect(ok.status).toBe(200);
    expect(await ok.json()).toEqual({ ok: true, next: "/dashboard" });
  });
  it("enrolment is refused once a verified factor exists", async () => {
    h.identity = IDENTITIES.ownerAal1!;
    expect((await mfaEnrollPOST(json({}))).status).toBe(409);
  });
});

describe("webhook route (raw body)", () => {
  async function post(body: string, sig?: string, eventId = `evt_${randomBytes(6).toString("hex")}`) {
    return webhookPOST(new Request(`${CHECKOUT_ORIGIN}/api/webhooks/razorpay`, {
      method: "POST",
      headers: { "x-razorpay-signature": sig ?? signWebhookBody(body, WEBHOOK_SECRET), "x-razorpay-event-id": eventId, "content-type": "application/json" },
      body,
    }));
  }
  const body = JSON.stringify({ entity: "event", event: "payment.captured", payload: { payment: { entity: { id: "pay_TR1", amount: 1, currency: "INR", status: "captured", order_id: "order_x" } } } });
  it("valid → 200 and stored; duplicate → 200 duplicate", async () => {
    const r1 = await post(body, undefined, "evt_route_dup");
    expect(r1.status).toBe(200);
    expect(await r1.json()).toEqual({ ok: true, duplicate: false });
    const r2 = await post(body, undefined, "evt_route_dup");
    expect(await r2.json()).toEqual({ ok: true, duplicate: true });
  });
  it("forged → 401; re-serialised → 401; nothing stored", async () => {
    expect((await post(body, signWebhookBody(body, "wrong"), "evt_route_forged")).status).toBe(401);
    expect((await post(JSON.stringify(JSON.parse(body)) + " ", signWebhookBody(body, WEBHOOK_SECRET), "evt_route_reser")).status).toBe(401);
    const n = await tdb.admin.query(`SELECT count(*)::int AS n FROM billing.webhook_receipts WHERE event_id IN ('evt_route_forged','evt_route_reser')`);
    expect(n.rows[0].n).toBe(0);
  });
});

describe("public checkout", () => {
  const req = (url: string, body: unknown, origin = CHECKOUT_ORIGIN) =>
    new Request(`${CHECKOUT_ORIGIN}${url}`, { method: "POST", headers: { origin, "content-type": "application/json", "x-forwarded-for": `10.0.0.${Math.floor(Math.random() * 200)}` }, body: JSON.stringify(body) });
  const base = { productId: "", customerName: "Test Buyer", email: "buyer@example.test", schoolName: "Fake Checkout School", machineCode: MC, confirm: true };

  it("amount comes from the server catalogue, never the browser", async () => {
    const r = await ordersPOST(req("/api/checkout/orders", { ...base, productId, amount: 1, amount_paise: 1, currency: "USD", modules: ["everything"] }));
    expect(r.status).toBe(200);
    const b = await r.json();
    expect(b.checkout).toMatchObject({ amount: pricePaise, currency: "INR", key: "rzp_test_FIXTURE" });
    expect(JSON.stringify(b)).not.toContain(KEY_SECRET);
    expect(rzp.created.at(-1)).toMatchObject({ amount: pricePaise, currency: "INR" });
  });
  it("invalid machine-code checksum → 400 before any provider order", async () => {
    const before = rzp.created.length;
    const r = await ordersPOST(req("/api/checkout/orders", { ...base, productId, machineCode: "R69C-090M-XG7X-2" }));
    expect(r.status).toBe(400);
    expect((await r.json()).error).toBe("invalid_machine_code");
    expect(rzp.created.length).toBe(before);
  });
  it("cross-origin order creation → 403", async () => {
    expect((await ordersPOST(req("/api/checkout/orders", { ...base, productId }, "https://evil.example"))).status).toBe(403);
  });
  it("browser 'success' with a forged signature is refused; a valid one only enqueues reconciliation", async () => {
    const o = await seededIssuedLicence();
    const bad = await verifyPOST(req("/api/checkout/verify", { statusToken: o.statusToken, razorpay_payment_id: "pay_TB1", razorpay_signature: "ab".repeat(32) }));
    expect(bad.status).toBe(400);
    const good = await verifyPOST(req("/api/checkout/verify", { statusToken: o.statusToken, razorpay_payment_id: "pay_TB1", razorpay_signature: signCheckout(o.providerOrderId, "pay_TB1", KEY_SECRET) }));
    expect(await good.json()).toEqual({ status: "verifying" });
    const ord = await tdb.admin.query(`SELECT status FROM billing.orders WHERE id = $1`, [o.orderId]);
    expect(ord.rows[0].status).toBe("pending");
  });
  it("customer .vlic download: own token works; another customer's / guessed token → 404", async () => {
    const a = await seededIssuedLicence();
    const b = await createCheckoutOrder(tdb.pool("vidya_portal"), { productId, customerName: "Other", email: "other@example.test", phone: null, schoolName: "Other Fake School", machineCode: MC, environment: "test" });
    const get = (t: string) => licenceGET(new Request(`${CHECKOUT_ORIGIN}/api/checkout/licence/${t}`), { params: Promise.resolve({ token: t }) });
    const ra = await get(a.statusToken);
    expect(ra.status).toBe(200);
    expect(await ra.text()).toBe("TEST-VLIC");
    expect((await get(b.statusToken)).status).toBe(404); // B has no licence; never A's
    expect((await get(randomBytes(32).toString("base64url"))).status).toBe(404);
    expect((await get(a.orderId)).status).toBe(404);
  });
});

describe("deployment surface separation", () => {
  it("checkout deployment 404s owner APIs; owner deployment 404s webhooks/checkout", async () => {
    h.identity = IDENTITIES.ownerAal2!;
    process.env.PORTAL_SURFACE = "checkout";
    const r = await exportGET(new Request(`${OWNER_ORIGIN}/api/owner/export/orders`), { params: Promise.resolve({ table: "orders" }) });
    expect(r.status).toBe(404);
    process.env.PORTAL_SURFACE = "owner";
    const w = await webhookPOST(new Request(`${CHECKOUT_ORIGIN}/api/webhooks/razorpay`, { method: "POST", body: "{}" }));
    expect(w.status).toBe(404);
  });
});
