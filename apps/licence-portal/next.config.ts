import path from "node:path";
import type { NextConfig } from "next";

const repoRoot = path.join(import.meta.dirname, "../..");

const securityHeaders = [
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "Referrer-Policy", value: "no-referrer" },
  { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=(), payment=(self)" },
  { key: "Cross-Origin-Opener-Policy", value: "same-origin-allow-popups" },
];

const nextConfig: NextConfig = {
  // The shared billing domain lives in ../../packages/billing-core as TypeScript source.
  transpilePackages: ["@vidya/billing-core"],
  serverExternalPackages: ["pg", "razorpay"],
  turbopack: { root: repoRoot },
  outputFileTracingRoot: repoRoot,
  poweredByHeader: false,
  reactStrictMode: true,
  async headers() {
    return [{ source: "/:path*", headers: securityHeaders }];
  },
};

export default nextConfig;
