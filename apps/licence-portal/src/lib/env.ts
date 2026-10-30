// Server-only configuration, parsed lazily at REQUEST time (so `next build` needs
// no secrets). The portal deliberately has NO NEXT_PUBLIC_ variables: nothing in
// the browser needs configuration, and the Supabase publishable key is only used
// server-side because every auth call goes through our route handlers.
import "server-only";
import { z } from "zod";

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const origin = z
  .string()
  .url()
  .refine((u) => new URL(u).origin === u, "must be an exact origin like https://owner.example.com (no path / trailing slash)");

export type Surface = "owner" | "checkout" | "both";

export function surface(): Surface {
  const s = process.env.PORTAL_SURFACE;
  if (s === "owner" || s === "checkout") return s;
  // 'both' is for local development only; production must pick one per deployment.
  if (s === "both" && process.env.NODE_ENV !== "production") return "both";
  return process.env.NODE_ENV === "production" ? "owner" : "both";
}

export class ConfigError extends Error {
  override name = "ConfigError";
}

function parse<T extends z.ZodRawShape>(shape: T, label: string): z.infer<z.ZodObject<T>> {
  const r = z.object(shape).safeParse(process.env);
  if (!r.success) {
    const names = [...new Set(r.error.issues.map((i) => i.path.join(".")))].join(", ");
    throw new ConfigError(`${label}: missing/invalid server configuration: ${names}`);
  }
  return r.data;
}

export function ownerEnv() {
  return parse(
    {
      OWNER_ORIGIN: origin,
      SUPABASE_URL: z.string().url(),
      SUPABASE_PUBLISHABLE_KEY: z.string().min(10),
      OWNER_AUTH_USER_ID: z.string().regex(uuid, "must be the owner's Supabase Auth user UUID"),
      PORTAL_DATABASE_URL: z.string().min(1),
    },
    "owner portal",
  );
}

export function checkoutEnv() {
  const e = parse(
    {
      CHECKOUT_ORIGIN: origin,
      PORTAL_DATABASE_URL: z.string().min(1),
      RAZORPAY_MODE: z.enum(["test", "live"]),
      RAZORPAY_KEY_ID: z.string().regex(/^rzp_(test|live)_[A-Za-z0-9]+$/),
      RAZORPAY_KEY_SECRET: z.string().min(8),
      RAZORPAY_WEBHOOK_SECRET: z.string().min(8),
      CHECKOUT_BRAND_NAME: z.string().min(1).default("Vidya"),
    },
    "checkout",
  );
  if (!e.RAZORPAY_KEY_ID.startsWith(`rzp_${e.RAZORPAY_MODE}_`)) {
    throw new ConfigError("checkout: RAZORPAY_KEY_ID does not match RAZORPAY_MODE");
  }
  return e;
}

export function databaseUrl(): string {
  const u = process.env.PORTAL_DATABASE_URL;
  if (!u) throw new ConfigError("PORTAL_DATABASE_URL is not set");
  return u;
}
