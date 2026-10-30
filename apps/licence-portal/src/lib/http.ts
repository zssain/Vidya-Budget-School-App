// Request/response helpers shared by route handlers.
import "server-only";
import { randomUUID } from "node:crypto";
import { NextResponse } from "next/server";
import type { DenyReason } from "./owner-guard";

export const NO_STORE = {
  "Cache-Control": "no-store, max-age=0, private",
  Pragma: "no-cache",
  "CDN-Cache-Control": "no-store",
  "Vercel-CDN-Cache-Control": "no-store",
} as const;

export function requestId(req: Request): string {
  const h = req.headers.get("x-request-id");
  return h && /^[A-Za-z0-9_-]{8,64}$/.test(h) ? h : randomUUID();
}

/**
 * CSRF defence for browser mutations: the request must come from our exact origin.
 * Browsers always send Origin on POST (forms and fetch); if it is absent we fall
 * back to Sec-Fetch-Site. Anything cross-site is refused.
 */
export function sameOrigin(req: Request, allowedOrigin: string): boolean {
  const origin = req.headers.get("origin");
  if (origin !== null) return origin === allowedOrigin;
  return req.headers.get("sec-fetch-site") === "same-origin";
}

export function json(body: unknown, status = 200, extra: Record<string, string> = {}) {
  return NextResponse.json(body, { status, headers: { ...NO_STORE, ...extra } });
}

export function deny(reason: DenyReason | "csrf" | "rate_limited"): NextResponse {
  const status =
    reason === "unauthenticated" ? 401 : reason === "rate_limited" ? 429 : reason === "misconfigured" ? 503 : 403;
  return json({ error: reason }, status);
}

/** 303 redirect to a same-origin path (callers pass only allowlisted paths). */
export function seeOther(origin: string, path: string): NextResponse {
  return NextResponse.redirect(new URL(path, origin), { status: 303, headers: NO_STORE });
}

/** Client key for rate limiting (first X-Forwarded-For hop set by the platform). */
export function clientKey(req: Request): string {
  const xff = req.headers.get("x-forwarded-for");
  const ip = xff?.split(",")[0]?.trim() || req.headers.get("x-real-ip") || "unknown";
  return ip.slice(0, 64);
}
