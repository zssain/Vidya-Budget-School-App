# Operations: worker, signer, migrations, monitoring

## Processes

| Process | Start | Scale | Health |
|---|---|---|---|
| owner portal | `npm start` in `apps/licence-portal` (`PORTAL_SURFACE=owner`) | stateless, any count | `GET /api/health` |
| checkout | same build, `PORTAL_SURFACE=checkout` | stateless | `GET /api/health` |
| billing worker | `npm start` in `services/billing-worker` | 1+ (SKIP LOCKED claims) | `WORKER_HEALTH_PORT` on 127.0.0.1 → `{healthy,last_tick,queue}`; row in `billing.worker_heartbeats` |
| licence signer | see licence-signer.md | 1+ | its job queue (`issue_licence`) |

SIGTERM: the worker finishes the current job and exits. If it is killed instead, the job's
lease (default 120 s) expires and another worker reclaims it; every handler is idempotent.

## Alerts to configure

- `billing.worker_heartbeats.last_seen` older than 5 min.
- `SELECT count(*) FROM billing.jobs WHERE state = 'dead'` > 0.
- Oldest ready job: `SELECT now() - min(available_at) FROM billing.jobs WHERE state='ready'` > 15 min.
- Paid but not issued for > 30 min (signer down): dashboard card "Paid, awaiting issuance".
- `deliveries.state = 'failed'` growing.

Logs: the worker writes one JSON line per job (id, kind, attempt, outcome, redacted error).
It never logs payloads, emails, keys or connection strings.

## Dead jobs

Inspect: `SELECT id, kind, attempts, last_error FROM billing.jobs WHERE state='dead';`
After fixing the cause, requeue (admin connection):
```sql
UPDATE billing.jobs SET state='ready', attempts=0, available_at=now(), last_error=NULL
 WHERE id = '<job id>' AND state = 'dead';
```
Requeueing is safe for every kind (idempotent). For a dead `issue_licence` job the signer
also marked the issuance `failed`; use the owner action **Re-issue (new generation)** on the
licence page instead of requeueing.

## Migrations

Add `infra/licensing/migrations/000N_<name>.sql` (never edit an applied file — the runner
refuses checksum changes), test with the suites (they rebuild the template), then
`BILLING_MIGRATION_DATABASE_URL=… npm run migrate` before deploying code that needs it.

## Data retention

`purge_webhooks` runs daily and replaces raw webhook payloads older than
`WEBHOOK_RETENTION_DAYS` (default 30) with `{"purged":true}`, keeping event id + SHA-256 for
dedupe and audit. Rate-limit windows older than a day can be deleted
(`purgeRateLimits`, or `DELETE FROM billing.auth_rate_limits WHERE window_start < now() - interval '1 day'`).
Financial rows and `owner_audit` are never deleted by the application.

## Backup / restore

Use the provider's PITR. Restore drill: restore to a new database, run `npm run migrate`
(should report "up to date"), point a staging portal at it, check counts of orders,
entitlements and issuances. The signing key is backed up separately (licence-signer.md).
