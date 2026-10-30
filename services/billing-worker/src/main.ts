// Entry point: a continuously running process (container / VM / systemd), NOT a
// serverless function. SIGTERM finishes the current job, then exits.
import { createServer } from "node:http";
import pg from "pg";
import { loadConfig } from "./config";
import { razorpayApi, resendMailer } from "./providers";
import { startLoop, type WorkerDeps } from "./worker";

function log(level: "info" | "warn" | "error", msg: string, fields: Record<string, unknown> = {}) {
  // Structured, minimal: job ids/kinds and redacted errors only — no payloads, emails or keys.
  process.stdout.write(JSON.stringify({ t: new Date().toISOString(), level, msg, ...fields }) + "\n");
}

const cfg = loadConfig();
const pool = new pg.Pool({ connectionString: cfg.BILLING_WORKER_DATABASE_URL, max: 5 });
pool.on("error", (e) => log("error", "pg pool error", { error: e.message }));

const deps: WorkerDeps = {
  pool,
  razorpay: razorpayApi(cfg.RAZORPAY_KEY_ID, cfg.RAZORPAY_KEY_SECRET),
  mailer: resendMailer(cfg.RESEND_API_KEY),
  workerId: cfg.workerId,
  environment: cfg.RAZORPAY_MODE,
  fromEmail: cfg.BILLING_FROM_EMAIL,
  leaseSeconds: cfg.WORKER_LEASE_SECONDS,
  expectedAccountId: cfg.RAZORPAY_ACCOUNT_ID ?? null,
  webhookRetentionDays: cfg.WEBHOOK_RETENTION_DAYS,
  log,
};

const loop = startLoop(deps, { pollMs: cfg.WORKER_POLL_MS, sweepSeconds: cfg.WORKER_SWEEP_SECONDS });
log("info", "billing worker started", { worker: cfg.workerId, mode: cfg.RAZORPAY_MODE });

// Optional loopback-only health endpoint for the process supervisor. It exposes
// liveness and queue age only — never owner data — and binds to 127.0.0.1.
let server: ReturnType<typeof createServer> | null = null;
if (cfg.WORKER_HEALTH_PORT) {
  server = createServer(async (_req, res) => {
    try {
      const q = await pool.query<{ ready: string; dead: string; oldest_s: string | null }>(
        `SELECT count(*) FILTER (WHERE state = 'ready')::text AS ready,
                count(*) FILTER (WHERE state = 'dead')::text AS dead,
                extract(epoch FROM now() - min(available_at) FILTER (WHERE state = 'ready'))::int::text AS oldest_s
           FROM billing.jobs`,
      );
      const last = loop.lastTick();
      const healthy = last !== null && Date.now() - last.getTime() < 5 * 60_000;
      res.writeHead(healthy ? 200 : 503, { "content-type": "application/json", "cache-control": "no-store" });
      res.end(JSON.stringify({ healthy, last_tick: last, queue: q.rows[0] }));
    } catch {
      res.writeHead(503).end();
    }
  }).listen(cfg.WORKER_HEALTH_PORT, "127.0.0.1");
}

async function shutdown(sig: string) {
  log("info", "shutting down", { sig });
  await loop.stop();
  server?.close();
  await pool.end();
  process.exit(0);
}
process.on("SIGTERM", () => void shutdown("SIGTERM"));
process.on("SIGINT", () => void shutdown("SIGINT"));
