import { describe, expect, it } from "vitest";
import {
  canonicalMachineCode,
  csvCell,
  deriveMachineCode,
  formatInr,
  parseMachineCodeCompact,
  safeOwnerRedirect,
  signCheckout,
  signWebhookBody,
  toCsv,
  verifyCheckoutSignature,
  verifyWebhookSignature,
  backoffSeconds,
  nextOrderStatus,
  normalisePaymentEntity,
  safeErrorText,
  hashStatusToken,
  newStatusToken,
  looksLikeStatusToken,
} from "../src/index";

// Vectors produced by running the REAL Rust implementation
// (vidya_core::licence::machine_code / parse_machine_code) on 2026-10-31 via a
// scratch binary depending on crates/vidya-core. "3N0S-10YX-JQCR-T" is also the
// cross-crate golden vector pinned in licence.rs.
const RUST_DERIVE: [string, string][] = [
  ["golden-machine", "3N0S-10YX-JQCR-T"],
  ["pc-1", "R69C-090M-XG7X-1"],
  ["pc-2", "RD9F-FC95-VGVQ-3"],
  ["machine-uuid-abc", "B0CF-D09Q-5KZT-4"],
  ["some-machine", "W535-M7VV-QCWS-J"],
  ["school-pc-0001", "3357-F8E9-PEQV-P"],
];
const RUST_PARSE: [string, string | null][] = [
  ["r69c 090m xg7x 1", "R69C090MXG7X1"],
  ["R69C-O9OM-XG7X-1", "R69C090MXG7X1"],
  ["R69C-090M-XG7X-2", null],
  ["R69C-090M-XG7X", null],
  ["r69c-090m-xg7x-1-0", null],
  ["R69C-090M-XG7U-1", null],
  ["ILO0-0000-0000-2", "1100000000002"],
  ["", null],
  ["R69C_090M_XG7X_1", null],
];

describe("machine code (port of vidya-core licence.rs)", () => {
  it.each(RUST_DERIVE)("derive %s → %s (matches Rust)", (id, code) => {
    expect(deriveMachineCode(id)).toBe(code);
    expect(parseMachineCodeCompact(code)).toBe(code.replace(/-/g, ""));
  });
  it.each(RUST_PARSE)("parse %j → %j (matches Rust)", (input, out) => {
    expect(parseMachineCodeCompact(input)).toBe(out);
  });
  it("canonical form is dashed upper-case", () => {
    expect(canonicalMachineCode(" r69c-o9om xg7x-1 ")).toBe("R69C-090M-XG7X-1");
    expect(canonicalMachineCode("nope")).toBeNull();
  });
  it("every single-character checksum flip is rejected", () => {
    const good = "R69C090MXG7X1";
    for (const ch of "0123456789ABCDEFGHJKMNPQRSTVWXYZ") {
      if (ch === "1") continue;
      expect(parseMachineCodeCompact(good.slice(0, 12) + ch)).toBeNull();
    }
  });
  it("rejects non-ASCII look-alikes and U", () => {
    expect(parseMachineCodeCompact("R69C-090M-XG7X-１")).toBeNull();
    expect(parseMachineCodeCompact("UUUU-UUUU-UUUU-U")).toBeNull();
  });
});

describe("money", () => {
  it("formats integer paise as INR without floats", () => {
    expect(formatInr(0)).toBe("₹0.00");
    expect(formatInr(5)).toBe("₹0.05");
    expect(formatInr(123456)).toBe("₹1,234.56");
    expect(formatInr(1234567890)).toBe("₹1,23,45,678.90");
    expect(() => formatInr(1.5)).toThrow();
    expect(() => formatInr(-1)).toThrow();
  });
});

describe("csv formula injection", () => {
  it("neutralises formula-leading cells and quotes everything", () => {
    expect(csvCell("=HYPERLINK(\"x\")")).toBe(`"'=HYPERLINK(""x"")"`);
    expect(csvCell("+91 98")).toBe(`"'+91 98"`);
    expect(csvCell("-2")).toBe(`"'-2"`);
    expect(csvCell("@SUM(A1)")).toBe(`"'@SUM(A1)"`);
    expect(csvCell("\t=1")).toBe(`"'\t=1"`);
    expect(csvCell("  =1")).toBe(`"'  =1"`);
    expect(csvCell("Saraswati School")).toBe(`"Saraswati School"`);
    expect(csvCell(null)).toBe(`""`);
    expect(toCsv(["a", "b"], [[1, "x,y"]])).toBe(`"a","b"\r\n"1","x,y"\r\n`);
  });
});

describe("razorpay signatures", () => {
  const secret = "whsec_test_fixture";
  const body = Buffer.from(`{"event":"payment.captured","payload":{}}`);
  it("accepts the exact raw body", () => {
    expect(verifyWebhookSignature(body, signWebhookBody(body, secret), secret)).toBe(true);
  });
  it("rejects forged, wrong-secret, reserialised, empty and malformed signatures", () => {
    const sig = signWebhookBody(body, secret);
    expect(verifyWebhookSignature(body, sig, "other")).toBe(false);
    const reserialised = Buffer.from(JSON.stringify(JSON.parse(body.toString()), null, 1));
    expect(verifyWebhookSignature(reserialised, sig, secret)).toBe(false);
    expect(verifyWebhookSignature(body, "", secret)).toBe(false);
    expect(verifyWebhookSignature(body, null, secret)).toBe(false);
    expect(verifyWebhookSignature(body, sig.slice(0, 63), secret)).toBe(false);
    expect(verifyWebhookSignature(body, "zz" + sig.slice(2), secret)).toBe(false);
    expect(verifyWebhookSignature(body, sig, "")).toBe(false);
  });
  it("checkout signature binds stored order id + payment id", () => {
    const s = signCheckout("order_A", "pay_1", "key_secret");
    expect(verifyCheckoutSignature("order_A", "pay_1", s, "key_secret")).toBe(true);
    expect(verifyCheckoutSignature("order_B", "pay_1", s, "key_secret")).toBe(false);
    expect(verifyCheckoutSignature("order_A", "pay_2", s, "key_secret")).toBe(false);
  });
});

describe("redirect allowlist", () => {
  it.each([
    ["/orders?q=1", "/orders?q=1"],
    ["/licences/abc", "/licences/abc"],
    ["//evil.example", "/dashboard"],
    ["https://evil.example/dashboard", "/dashboard"],
    ["/\\evil", "/dashboard"],
    ["/%2F%2Fevil", "/dashboard"],
    ["/api/owner/x", "/dashboard"],
    ["/dashboardevil", "/dashboard"],
    ["javascript:alert(1)", "/dashboard"],
    [undefined, "/dashboard"],
  ])("%j → %j", (input, out) => {
    expect(safeOwnerRedirect(input)).toBe(out);
  });
});

describe("state machine + misc", () => {
  it("order status never moves backwards and authorized never pays", () => {
    expect(nextOrderStatus("pending", "authorized", false, true)).toBe("pending");
    expect(nextOrderStatus("pending", "captured", true, true)).toBe("paid");
    expect(nextOrderStatus("pending", "captured", true, false)).toBe("paid_awaiting_machine_code");
    expect(nextOrderStatus("refunded", "captured", true, true)).toBe("refunded");
    expect(nextOrderStatus("paid", "refunded", true, true)).toBe("refunded");
    expect(nextOrderStatus("paid", "partially_refunded", true, true)).toBe("partially_refunded");
    expect(nextOrderStatus("refunded", "partially_refunded", true, true)).toBe("refunded");
    expect(nextOrderStatus("paid", "disputed", true, true)).toBe("disputed");
  });
  it("normalises payment entities defensively", () => {
    expect(normalisePaymentEntity({ id: "pay_1", amount: 100, currency: "INR", status: "captured", order_id: "order_1" }))
      .toMatchObject({ state: "captured", captureHappened: true });
    expect(normalisePaymentEntity({ id: "pay_1", amount: 100, currency: "INR", status: "authorized" }))
      .toMatchObject({ state: "authorized", captureHappened: false });
    expect(normalisePaymentEntity({ id: "pay_1", amount: 1.5, currency: "INR", status: "captured" })).toBeNull();
    expect(normalisePaymentEntity({ id: "x", amount: 1, currency: "INR", status: "captured" })).toBeNull();
    expect(normalisePaymentEntity({ id: "pay_1", amount: 100, currency: "INR", status: "refunded", refund_status: "partial", captured: true }))
      .toMatchObject({ state: "partially_refunded", captureHappened: true });
  });
  it("backoff is exponential and bounded", () => {
    expect([1, 2, 3, 4].map(backoffSeconds)).toEqual([5, 10, 20, 40]);
    expect(backoffSeconds(30)).toBe(3600);
  });
  it("error text redacts secrets", () => {
    expect(safeErrorText(new Error("bad rzp_test_ABCdef123 postgres://u:p@h/db"))).toBe("Error: bad rzp_test_*** postgres://***");
  });
  it("status tokens are 256-bit and only the hash is derived", () => {
    const { token, hash } = newStatusToken();
    expect(looksLikeStatusToken(token)).toBe(true);
    expect(hash).toBe(hashStatusToken(token));
    expect(hash).not.toContain(token);
    expect(looksLikeStatusToken("short")).toBe(false);
  });
});
