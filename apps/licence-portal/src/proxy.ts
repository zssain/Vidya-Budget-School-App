// Next.js 16 Proxy (formerly middleware; Node.js runtime). Responsibilities:
//  1. Surface separation: owner routes 404 on the checkout deployment and vice versa.
//  2. Owner surface: refresh the Supabase session cookie (official @supabase/ssr
//     pattern) and an OPTIMISTIC redirect to /login when no session cookie exists.
//     This is NOT authorization — every page/route still calls requireOwner().
//  3. Headers: Cache-Control no-store on every owner response, CSP per surface.
import { createServerClient } from "@supabase/ssr";
import { NextResponse, type NextRequest } from "next/server";
import { surfaceOfPath } from "./lib/surface-path";

const PROD = process.env.NODE_ENV === "production";
const OWNER_COOKIE = PROD ? "__Host-vidya-owner" : "vidya-owner";

function currentSurface(): "owner" | "checkout" | "both" {
  const s = process.env.PORTAL_SURFACE;
  if (s === "owner" || s === "checkout") return s;
  if (s === "both" && !PROD) return "both";
  return PROD ? "owner" : "both";
}

const OWNER_CSP = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline'" + (PROD ? "" : " 'unsafe-eval'"),
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "connect-src 'self'",
  "frame-ancestors 'none'",
  "form-action 'self'",
  "base-uri 'none'",
  "object-src 'none'",
].join("; ");

// Razorpay Standard Checkout loads checkout.js and opens its iframe/popup.
const CHECKOUT_CSP = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline' https://checkout.razorpay.com" + (PROD ? "" : " 'unsafe-eval'"),
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: https:",
  "connect-src 'self' https://*.razorpay.com",
  "frame-src https://api.razorpay.com https://checkout.razorpay.com",
  "frame-ancestors 'none'",
  "form-action 'self'",
  "base-uri 'none'",
  "object-src 'none'",
].join("; ");

const PUBLIC_OWNER_PATHS = ["/login", "/mfa", "/api/auth/login", "/api/auth/logout", "/api/auth/mfa"];

export async function proxy(req: NextRequest) {
  const path = req.nextUrl.pathname;
  const kind = surfaceOfPath(path);
  const cur = currentSurface();
  if (kind !== "shared" && cur !== "both" && kind !== cur) {
    return new NextResponse("Not found", { status: 404, headers: { "Cache-Control": "no-store" } });
  }

  if (kind === "checkout") {
    const res = NextResponse.next();
    res.headers.set("Content-Security-Policy", CHECKOUT_CSP);
    if (path.startsWith("/buy/status") || path.startsWith("/api/")) res.headers.set("Cache-Control", "no-store");
    return res;
  }
  if (kind === "shared") return NextResponse.next();

  // ---- owner surface ----
  let res = NextResponse.next({ request: req });
  const url = process.env.SUPABASE_URL;
  const key = process.env.SUPABASE_PUBLISHABLE_KEY;
  if (url && key) {
    const supabase = createServerClient(url, key, {
      cookieOptions: { name: OWNER_COOKIE, path: "/", sameSite: "lax", secure: PROD },
      cookies: {
        getAll: () => req.cookies.getAll(),
        setAll: (list) => {
          for (const { name, value } of list) req.cookies.set(name, value);
          res = NextResponse.next({ request: req });
          for (const { name, value, options } of list) {
            res.cookies.set(name, value, { ...options, httpOnly: true, secure: PROD || options?.secure === true, sameSite: "lax", path: "/" });
          }
        },
      },
      auth: { flowType: "pkce", detectSessionInUrl: false, autoRefreshToken: false },
    });
    // Refreshes an expiring session and writes the new cookies onto `res`.
    await supabase.auth.getUser().catch(() => undefined);
  }
  const hasSession = req.cookies.getAll().some((c) => c.name.startsWith(OWNER_COOKIE));
  const isPublic = PUBLIC_OWNER_PATHS.some((p) => path === p || path.startsWith(p + "/"));
  if (!hasSession && !isPublic && !path.startsWith("/api/")) {
    const to = req.nextUrl.clone();
    to.pathname = "/login";
    to.search = path === "/" ? "" : `?next=${encodeURIComponent(path)}`;
    const r = NextResponse.redirect(to, 303);
    r.headers.set("Cache-Control", "no-store");
    return r;
  }
  res.headers.set("Cache-Control", "no-store, max-age=0, private");
  res.headers.set("Content-Security-Policy", OWNER_CSP);
  return res;
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
