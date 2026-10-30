// Redirect allowlist for owner flows: only same-origin absolute paths under known
// owner prefixes. Anything else (scheme, //host, backslashes, control chars,
// encoded tricks) → fallback.
const OWNER_PREFIXES = [
  "/dashboard",
  "/schools",
  "/orders",
  "/payments",
  "/licences",
  "/transfers",
  "/delivery",
  "/audit",
  "/settings",
  "/mfa",
];

export function safeOwnerRedirect(next: unknown, fallback = "/dashboard"): string {
  if (typeof next !== "string" || next.length === 0 || next.length > 512) return fallback;
  if (!next.startsWith("/") || next.startsWith("//") || next.includes("\\")) return fallback;
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(next)) return fallback;
  let decoded: string;
  try {
    decoded = decodeURIComponent(next);
  } catch {
    return fallback;
  }
  if (decoded.startsWith("//") || decoded.includes("\\")) return fallback;
  const path = next.split(/[?#]/, 1)[0]!;
  const ok = OWNER_PREFIXES.some((p) => path === p || path.startsWith(p + "/"));
  return ok ? next : fallback;
}
