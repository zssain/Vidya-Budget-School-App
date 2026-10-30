-- 0002: portal/worker support tables. Additive only; 0001 is never edited.
--
-- Why:
--  * auth_rate_limits — login/MFA/checkout throttling must work across multiple
--    serverless instances, so the counter lives in Postgres (an in-memory map per
--    instance would be trivially bypassed). Buckets hold a SHA-256 of the client
--    key (IP / email), never the raw value.
--  * worker_heartbeats — the dashboard and the readiness check show whether a
--    billing worker is actually running (spec: "worker heartbeat").
--  * orders.status_token_hash unique index — capability lookups are by hash; the
--    index makes them O(log n) and guarantees one order per capability.
SET search_path = billing, public;

CREATE TABLE auth_rate_limits (
  bucket        text        NOT NULL,          -- e.g. 'login:ip:<sha256>'
  window_start  timestamptz NOT NULL,
  hits          int         NOT NULL DEFAULT 1 CHECK (hits >= 1),
  PRIMARY KEY (bucket, window_start)
);
ALTER TABLE auth_rate_limits ENABLE ROW LEVEL SECURITY;

CREATE TABLE worker_heartbeats (
  worker_id   text PRIMARY KEY,
  service     text NOT NULL CHECK (service IN ('billing-worker', 'licence-signer')),
  last_seen   timestamptz NOT NULL DEFAULT now(),
  started_at  timestamptz NOT NULL DEFAULT now(),
  info        jsonb NOT NULL DEFAULT '{}'::jsonb
);
ALTER TABLE worker_heartbeats ENABLE ROW LEVEL SECURITY;

CREATE UNIQUE INDEX orders_status_token_hash_key ON orders (status_token_hash)
  WHERE status_token_hash IS NOT NULL;
CREATE INDEX orders_status_created_idx ON orders (status, created_at);
CREATE INDEX licence_issuances_status_idx ON licence_issuances (status);
CREATE INDEX deliveries_issuance_idx ON deliveries (issuance_id);
CREATE INDEX owner_audit_target_idx ON owner_audit (target, action);

REVOKE ALL ON auth_rate_limits, worker_heartbeats FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON auth_rate_limits TO vidya_portal;
GRANT SELECT ON worker_heartbeats TO vidya_portal;
GRANT SELECT, INSERT, UPDATE ON worker_heartbeats TO vidya_worker, vidya_signer;

CREATE POLICY server_roles ON auth_rate_limits TO vidya_portal USING (true) WITH CHECK (true);
CREATE POLICY server_roles ON worker_heartbeats TO vidya_portal, vidya_worker, vidya_signer
  USING (true) WITH CHECK (true);
