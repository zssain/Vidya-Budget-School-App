// Worker configuration. Every value is server-only; nothing here is ever bundled
// for a browser. Test and live credentials are distinct and must agree with
// RAZORPAY_MODE (rzp_test_… vs rzp_live_…), otherwise the worker refuses to start.
import { hostname } from "node:os";
import { z } from "zod";

const schema = z
  .object({
    BILLING_WORKER_DATABASE_URL: z.string().min(1),
    RAZORPAY_MODE: z.enum(["test", "live"]),
    RAZORPAY_KEY_ID: z.string().regex(/^rzp_(test|live)_[A-Za-z0-9]+$/),
    RAZORPAY_KEY_SECRET: z.string().min(8),
    RAZORPAY_ACCOUNT_ID: z.string().optional(),
    RESEND_API_KEY: z.string().min(8),
    BILLING_FROM_EMAIL: z.string().min(3),
    WORKER_ID: z.string().optional(),
    WORKER_LEASE_SECONDS: z.coerce.number().int().min(15).max(3600).default(120),
    WORKER_POLL_MS: z.coerce.number().int().min(100).max(60000).default(2000),
    WORKER_SWEEP_SECONDS: z.coerce.number().int().min(5).max(3600).default(60),
    WEBHOOK_RETENTION_DAYS: z.coerce.number().int().min(1).max(3650).default(30),
    WORKER_HEALTH_PORT: z.coerce.number().int().min(1).max(65535).optional(),
  })
  .superRefine((v, ctx) => {
    if (!v.RAZORPAY_KEY_ID.startsWith(`rzp_${v.RAZORPAY_MODE}_`)) {
      ctx.addIssue({ code: "custom", message: "RAZORPAY_KEY_ID does not match RAZORPAY_MODE", path: ["RAZORPAY_KEY_ID"] });
    }
  });

export type WorkerConfig = z.infer<typeof schema> & { workerId: string };

export function loadConfig(env: NodeJS.ProcessEnv = process.env): WorkerConfig {
  const parsed = schema.safeParse(env);
  if (!parsed.success) {
    // Report only the variable NAMES that are wrong — never their values.
    const names = [...new Set(parsed.error.issues.map((i) => i.path.join(".")))].join(", ");
    throw new Error(`billing-worker: invalid configuration for: ${names}`);
  }
  return { ...parsed.data, workerId: parsed.data.WORKER_ID ?? `billing-worker@${hostname()}:${process.pid}` };
}
