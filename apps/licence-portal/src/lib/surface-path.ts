// Pure path → surface classification (shared by proxy.ts and route handlers).
export type RouteSurface = "owner" | "checkout" | "shared";

const CHECKOUT_PREFIXES = ["/buy", "/api/checkout", "/api/webhooks"];
const SHARED_PREFIXES = ["/_next", "/api/health", "/favicon.ico", "/robots.txt"];

export function surfaceOfPath(pathname: string): RouteSurface {
  const hit = (p: string) => pathname === p || pathname.startsWith(p + "/");
  if (pathname === "/") return "shared"; // redirects to /dashboard or /buy per deployment
  if (SHARED_PREFIXES.some(hit)) return "shared";
  if (CHECKOUT_PREFIXES.some(hit)) return "checkout";
  return "owner";
}
