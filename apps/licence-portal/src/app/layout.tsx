import type { Metadata } from "next";
import "./globals.css";

// Every response depends on the request (auth cookie, capability token, live
// billing data) — nothing may be prerendered or served from a shared cache.
export const dynamic = "force-dynamic";
export const fetchCache = "force-no-store";

export const metadata: Metadata = {
  title: "Vidya licensing",
  robots: { index: false, follow: false },
  referrer: "no-referrer",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
