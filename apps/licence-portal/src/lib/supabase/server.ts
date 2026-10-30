// Supabase Auth client for the OWNER surface, per the official @supabase/ssr
// Next.js flow (cookies getAll/setAll). Cookie hardening we apply on top of the SDK
// defaults, because no browser code ever reads the session:
//   * HttpOnly  — the SDK default is readable by JS (so its browser client can work);
//                 we never create a browser client, so we force HttpOnly.
//   * Secure    — always in production.
//   * SameSite=Lax, Path=/, no Domain → host-only: the checkout host never sees it.
//   * name "__Host-vidya-owner" in production (the __Host- prefix is enforced by
//     browsers: Secure + Path=/ + no Domain).
import "server-only";
import { createServerClient, type CookieOptions } from "@supabase/ssr";
import type { SupabaseClient } from "@supabase/supabase-js";
import { cookies } from "next/headers";
import { ownerEnv } from "../env";

const PROD = process.env.NODE_ENV === "production";
export const OWNER_COOKIE_NAME = PROD ? "__Host-vidya-owner" : "vidya-owner";

export function hardenCookie(options: CookieOptions = {}): CookieOptions {
  return { ...options, httpOnly: true, secure: PROD || options.secure === true, sameSite: "lax", path: "/", domain: undefined };
}

export async function createSupabaseServerClient(): Promise<SupabaseClient> {
  const env = ownerEnv();
  const store = await cookies();
  return createServerClient(env.SUPABASE_URL, env.SUPABASE_PUBLISHABLE_KEY, {
    cookieOptions: { name: OWNER_COOKIE_NAME, path: "/", sameSite: "lax", secure: PROD },
    cookies: {
      getAll: () => store.getAll(),
      setAll: (list) => {
        try {
          for (const { name, value, options } of list) store.set(name, value, hardenCookie(options));
        } catch {
          // Called from a Server Component render: cookies are read-only there. The
          // proxy refreshes the session on every owner request, so this is safe.
        }
      },
    },
    auth: { flowType: "pkce", detectSessionInUrl: false, persistSession: true, autoRefreshToken: false },
  });
}
