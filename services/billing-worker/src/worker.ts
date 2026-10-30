// The durable job loop. All state lives in Postgres (billing.jobs); nothing is kept
// in memory between ticks, so killing the process at any instant is safe: an
// uncommitted transaction rolls back, and a claimed-but-unfinished job's lease
// expires and is reclaimed by any worker.
import {
  claimJob,
  completeJob,
  deadLetterExhaustedLeases,
  deliverLicence,
  enqueueJob,
  failJob,
  processWebhookReceipt,
  purgeWebhookPayloads,
  reconcileOrder,
  safeErrorText,
  scheduleInitialDeliveries,
  scheduleReconciliation,
  WORKER_KINDS,
  PermanentJobError,
  type Environment,
  type Job,
  type Mailer,
  type PoolLike,
  type RazorpayApi,
} from "@vidya/billing-core";

export interface WorkerDeps {
  pool: PoolLike;
  razorpay: RazorpayApi;
  mailer: Mailer;
  workerId: string;
  environment: Environment;
  fromEmail: string;
  leaseSeconds: number;
  expectedAccountId?: string | null;
  webhookRetentionDays: number;
  log?: (level: "info" | "warn" | "error", msg: string, fields?: Record<string, unknown>) => void;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function uuidField(job: Job, name: string): string {
  const v = job.payload?.[name];
  if (typeof v !== "string" || !UUID.test(v)) throw new PermanentJobError(`job payload ${name} is missing or invalid`);
  return v;
}

export async function runJob(deps: WorkerDeps, job: Job): Promise<unknown> {
  switch (job.kind) {
    case "process_webhook":
      return processWebhookReceipt(deps.pool, uuidField(job, "receipt_id"), {
        expectedAccountId: deps.expectedAccountId ?? null,
      });
    case "reconcile_order":
      return reconcileOrder(deps.pool, deps.razorpay, uuidField(job, "order_id"), deps.environment);
    case "deliver_licence": {
      const issuanceId = uuidField(job, "issuance_id");
      const key = typeof job.payload.idempotency_key === "string" ? job.payload.idempotency_key : job.dedupe_key;
      return deliverLicence(deps.pool, deps.mailer, { issuanceId, idempotencyKey: key, from: deps.fromEmail });
    }
    case "purge_webhooks":
      return { purged: await purgeWebhookPayloads(deps.pool, deps.webhookRetentionDays) };
    default:
      throw new PermanentJobError(`worker does not handle ${job.kind}`);
  }
}

/** Claim and run at most one job. Returns false when the queue had nothing ready. */
export async function runOnce(deps: WorkerDeps): Promise<boolean> {
  const job = await claimJob(deps.pool, { workerId: deps.workerId, kinds: WORKER_KINDS, leaseSeconds: deps.leaseSeconds });
  if (!job) return false;
  try {
    const result = await runJob(deps, job);
    await completeJob(deps.pool, job, deps.workerId);
    deps.log?.("info", "job done", { id: job.id, kind: job.kind, attempt: job.attempts, result: summarise(result) });
  } catch (err) {
    const outcome = await failJob(deps.pool, job, deps.workerId, err);
    deps.log?.(outcome === "dead" ? "error" : "warn", "job failed", {
      id: job.id,
      kind: job.kind,
      attempt: job.attempts,
      outcome,
      error: safeErrorText(err),
    });
  }
  return true;
}

function summarise(result: unknown): unknown {
  if (result && typeof result === "object" && "state" in result) return (result as { state: unknown }).state;
  return undefined;
}

/** Periodic sweeps: lease dead-lettering, initial deliveries, reconciliation, purge. */
export async function sweep(deps: WorkerDeps, now = new Date()): Promise<Record<string, number>> {
  const dead = await deadLetterExhaustedLeases(deps.pool, WORKER_KINDS);
  const deliveries = await scheduleInitialDeliveries(deps.pool);
  const reconciles = await scheduleReconciliation(deps.pool, { environment: deps.environment });
  const purge = await enqueueJob(deps.pool, {
    kind: "purge_webhooks",
    dedupeKey: `purge:${now.toISOString().slice(0, 10)}`,
  });
  return { dead, deliveries, reconciles, purge: purge.created ? 1 : 0 };
}

export async function heartbeat(deps: WorkerDeps, info: Record<string, unknown>): Promise<void> {
  await deps.pool.query(
    `INSERT INTO billing.worker_heartbeats (worker_id, service, last_seen, info)
     VALUES ($1, 'billing-worker', now(), $2::jsonb)
     ON CONFLICT (worker_id) DO UPDATE SET last_seen = now(), info = EXCLUDED.info`,
    [deps.workerId, JSON.stringify(info)],
  );
}

export interface LoopHandle {
  stop(): Promise<void>;
  lastTick(): Date | null;
}

export function startLoop(deps: WorkerDeps, opts: { pollMs: number; sweepSeconds: number }): LoopHandle {
  let stopping = false;
  let lastTick: Date | null = null;
  let lastSweep = 0;
  const done = (async () => {
    while (!stopping) {
      try {
        if (Date.now() - lastSweep > opts.sweepSeconds * 1000) {
          const s = await sweep(deps);
          await heartbeat(deps, { sweep: s, pid: process.pid });
          lastSweep = Date.now();
        }
        let ran = 0;
        while (!stopping && ran < 25 && (await runOnce(deps))) ran++;
        lastTick = new Date();
      } catch (err) {
        deps.log?.("error", "tick failed", { error: safeErrorText(err) });
      }
      if (!stopping) await new Promise((r) => setTimeout(r, opts.pollMs));
    }
  })();
  return {
    async stop() {
      stopping = true;
      await done;
    },
    lastTick: () => lastTick,
  };
}
