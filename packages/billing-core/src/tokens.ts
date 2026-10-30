// Customer status-page capability: 256 random bits, base64url. Only the SHA-256
// hex digest is stored (orders.status_token_hash); the raw token exists only in the
// customer's URL. Lookups hash the presented token and match the digest, so the raw
// value never touches the database.
import { createHash, randomBytes } from "node:crypto";

export const STATUS_TOKEN_TTL_DAYS = 30;

export function newStatusToken(): { token: string; hash: string } {
  const token = randomBytes(32).toString("base64url");
  return { token, hash: hashStatusToken(token) };
}

export function hashStatusToken(token: string): string {
  return createHash("sha256").update(token, "utf8").digest("hex");
}

/** Shape check before hashing (43 base64url chars) — rejects junk cheaply. */
export function looksLikeStatusToken(token: string): boolean {
  return /^[A-Za-z0-9_-]{43}$/.test(token);
}

export function sha256Hex(data: Uint8Array | string): string {
  return createHash("sha256").update(data).digest("hex");
}
