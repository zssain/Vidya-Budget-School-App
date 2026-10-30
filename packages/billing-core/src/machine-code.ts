// Faithful TypeScript port of the machine-code CHECKSUM rules in
// crates/vidya-core/src/licence.rs (`parse_machine_code`, `crockford_value`,
// `machine_code`). The portal never computes a customer's code from their
// browser — `deriveMachineCode` exists only so tests can cross-check vectors
// against the Rust implementation (see test/machine-code.test.ts).
import { createHash } from "node:crypto";

/** Crockford base32 alphabet (no I, L, O, U) — identical to `CROCKFORD` in licence.rs. */
export const CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/** Rust `char::is_whitespace` (Unicode White_Space) — `\s` in JS also covers BOM;
 *  we list the White_Space set explicitly so the two agree byte-for-byte. */
const RUST_WHITESPACE = new Set([
  0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003,
  0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f,
  0x3000,
]);

/** `crockford_value`: ASCII-uppercase, map I/L→1 and O→0, then index in CROCKFORD. */
function crockfordValue(ch: string): number | null {
  // Rust's to_ascii_uppercase only touches a–z.
  const up = ch.length === 1 && ch >= "a" && ch <= "z" ? ch.toUpperCase() : ch;
  const mapped = up === "I" || up === "L" ? "1" : up === "O" ? "0" : up;
  if (mapped.length !== 1) return null;
  const i = CROCKFORD.indexOf(mapped);
  return i >= 0 ? i : null;
}

/**
 * Validate a typed machine code with the exact rules of `parse_machine_code`.
 * Returns the canonical 13-character (no dashes) upper-case form, or null when the
 * length, alphabet or checksum (`Σ data indices mod 32`) is wrong.
 */
export function parseMachineCodeCompact(text: string): string | null {
  const vals: number[] = [];
  for (const ch of text) {
    // iterate by code point, like Rust's `chars()`
    const cp = ch.codePointAt(0)!;
    if (ch === "-" || RUST_WHITESPACE.has(cp)) continue;
    const v = crockfordValue(ch);
    if (v === null) return null;
    vals.push(v);
  }
  if (vals.length !== 13) return null;
  let sum = 0;
  for (let i = 0; i < 12; i++) sum += vals[i]!;
  if (sum % 32 !== vals[12]) return null;
  return vals.map((v) => CROCKFORD[v]).join("");
}

/** Dashed display form `XXXX-XXXX-XXXX-C` of a compact canonical code. */
export function formatMachineCode(compact: string): string {
  return `${compact.slice(0, 4)}-${compact.slice(4, 8)}-${compact.slice(8, 12)}-${compact.slice(12)}`;
}

/**
 * Canonical form stored in `billing.orders.machine_code` / entitlements and put in
 * the signed payload: upper-case dashed `XXXX-XXXX-XXXX-C` (the same shape the app
 * displays and `machine_code()` emits). null = invalid.
 */
export function canonicalMachineCode(text: string): string | null {
  const compact = parseMachineCodeCompact(text);
  return compact === null ? null : formatMachineCode(compact);
}

/** Port of `machine_code(machine_id)` — TEST USE ONLY (vector cross-checking). */
export function deriveMachineCode(machineId: string): string {
  const digest = createHash("sha256").update(machineId, "utf8").digest();
  const bits60 = digest.readBigUInt64BE(0) >> 4n;
  const idx: number[] = [];
  let sum = 0;
  for (let i = 0; i < 12; i++) {
    const v = Number((bits60 >> BigInt(5 * (11 - i))) & 0x1fn);
    idx.push(v);
    sum += v;
  }
  const compact = idx.map((v) => CROCKFORD[v]).join("") + CROCKFORD[sum % 32];
  return formatMachineCode(compact);
}
