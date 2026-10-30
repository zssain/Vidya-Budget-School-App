-- Vidya owner licensing portal — billing schema (spec Phase 11). Postgres 15+.
-- Commercial metadata only: NO student, attendance, fee-receipt or school-Drive data.
-- Money is integer paise + explicit currency. Payment, entitlement (fulfilment) and
-- delivery states are separate. Every state change is transactional; uniqueness
-- constraints make webhook/worker/signer retries converge on ONE outcome.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ---------------------------------------------------------------- roles ------
-- Least-privilege roles (created idempotently; passwords set by the operator, never here).
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vidya_portal') THEN CREATE ROLE vidya_portal NOLOGIN; END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vidya_worker') THEN CREATE ROLE vidya_worker NOLOGIN; END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vidya_signer') THEN CREATE ROLE vidya_signer NOLOGIN; END IF;
END $$;

CREATE SCHEMA IF NOT EXISTS billing;
SET search_path = billing, public;

-- ------------------------------------------------------------- catalogue ------
CREATE TABLE products (
  id           text PRIMARY KEY,                       -- immutable SKU+version, e.g. 'vidya-perpetual-v1'
  name         text NOT NULL,
  currency     text NOT NULL DEFAULT 'INR' CHECK (currency = 'INR'),
  price_paise  bigint NOT NULL CHECK (price_paise > 0),
  modules      text[] NOT NULL CHECK (cardinality(modules) >= 1 AND 'core' = ANY (modules)),
  active       boolean NOT NULL DEFAULT true,
  created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing_customers (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name        text NOT NULL,
  email       text NOT NULL,
  phone       text,
  created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing_schools (
  id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  customer_id  uuid NOT NULL REFERENCES billing_customers(id),
  school_name  text NOT NULL,
  created_at   timestamptz NOT NULL DEFAULT now()
);

-- ----------------------------------------------------------------- orders ------
CREATE TABLE orders (
  id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  school_id          uuid NOT NULL REFERENCES billing_schools(id),
  product_id         text NOT NULL REFERENCES products(id),
  -- snapshot at order time (a later price/module change never alters this order)
  amount_paise       bigint NOT NULL CHECK (amount_paise > 0),
  currency           text NOT NULL CHECK (currency = 'INR'),
  modules            text[] NOT NULL,
  machine_code       text,                               -- canonical XXXX-XXXX-XXXX-C; NULL = paid_awaiting_machine_code
  environment        text NOT NULL CHECK (environment IN ('test','live')),
  provider           text NOT NULL DEFAULT 'razorpay' CHECK (provider IN ('razorpay','manual')),
  provider_order_id  text UNIQUE,
  status             text NOT NULL DEFAULT 'created'
                     CHECK (status IN ('created','pending','paid','paid_awaiting_machine_code','refunded','partially_refunded','disputed','cancelled')),
  status_token_hash  text,                               -- sha256 of the customer's status-page capability
  status_token_expires_at timestamptz,
  created_at         timestamptz NOT NULL DEFAULT now(),
  updated_at         timestamptz NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------- payments ------
CREATE TABLE payments (
  id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  order_id             uuid NOT NULL REFERENCES orders(id),
  provider_payment_id  text NOT NULL UNIQUE,
  amount_paise         bigint NOT NULL CHECK (amount_paise >= 0),
  currency             text NOT NULL,
  state                text NOT NULL CHECK (state IN ('authorized','captured','failed','refunded','partially_refunded','disputed')),
  refunded_paise       bigint NOT NULL DEFAULT 0 CHECK (refunded_paise >= 0),
  captured_at          timestamptz,
  created_at           timestamptz NOT NULL DEFAULT now(),
  updated_at           timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE webhook_receipts (
  id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  provider         text NOT NULL,
  environment      text NOT NULL CHECK (environment IN ('test','live')),
  event_id         text NOT NULL,
  event_type       text NOT NULL,
  payload_sha256   text NOT NULL,
  payload          jsonb NOT NULL,                       -- retention bounded by the purge job
  verified_at      timestamptz NOT NULL,
  processing_state text NOT NULL DEFAULT 'pending' CHECK (processing_state IN ('pending','done','failed','ignored')),
  last_error       text,
  UNIQUE (provider, environment, event_id)
);

-- ------------------------------------------------------------ entitlements ------
-- Exactly ONE entitlement per order, created only from a verified capture (or an
-- audited owner manual approval).
CREATE TABLE entitlements (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  order_id      uuid NOT NULL UNIQUE REFERENCES orders(id),
  modules       text[] NOT NULL,
  machine_code  text NOT NULL,
  school_name   text NOT NULL,
  source        text NOT NULL CHECK (source IN ('verified_payment','owner_manual')),
  commercial_status text NOT NULL DEFAULT 'active' CHECK (commercial_status IN ('active','refunded','disputed','superseded')),
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- One row per (entitlement, generation). Generation 1 = first issue; a transfer
-- creates generation n+1 with a new machine code. The EXACT payload bytes and
-- signature are persisted so a retry/resend never produces a different artefact.
CREATE TABLE licence_issuances (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  entitlement_id uuid NOT NULL REFERENCES entitlements(id),
  generation     int  NOT NULL CHECK (generation >= 1),
  licence_id     text NOT NULL UNIQUE,
  machine_code   text NOT NULL,
  issued_at      text NOT NULL,                          -- exact string inside the signed payload
  signer_key_id  text,
  payload        bytea,                                  -- exact signed bytes
  signature      bytea,
  licence_key    text,                                   -- grouped base64url(payload||sig)
  vlic           text,                                   -- .vlic file contents
  status         text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','issued','failed','superseded')),
  last_error     text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  issued_ts      timestamptz,
  UNIQUE (entitlement_id, generation)
);

CREATE TABLE licence_transfers (
  id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  source_issuance_id   uuid NOT NULL REFERENCES licence_issuances(id),
  target_machine_code  text NOT NULL,
  reason               text NOT NULL CHECK (length(reason) > 0),
  approved_at          timestamptz NOT NULL DEFAULT now(),
  replacement_issuance_id uuid REFERENCES licence_issuances(id)
);

-- --------------------------------------------------------------------- jobs ------
CREATE TABLE jobs (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  kind          text NOT NULL CHECK (kind IN ('process_webhook','reconcile_order','issue_licence','deliver_licence','purge_webhooks')),
  dedupe_key    text NOT NULL UNIQUE,
  payload       jsonb NOT NULL DEFAULT '{}'::jsonb,
  state         text NOT NULL DEFAULT 'ready' CHECK (state IN ('ready','running','done','dead')),
  attempts      int  NOT NULL DEFAULT 0,
  max_attempts  int  NOT NULL DEFAULT 8,
  available_at  timestamptz NOT NULL DEFAULT now(),
  locked_by     text,
  lease_until   timestamptz,
  last_error    text,
  created_at    timestamptz NOT NULL DEFAULT now(),
  completed_at  timestamptz
);
CREATE INDEX jobs_claim_idx ON jobs (kind, state, available_at);

CREATE TABLE deliveries (
  id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  issuance_id         uuid NOT NULL REFERENCES licence_issuances(id),
  channel             text NOT NULL CHECK (channel IN ('email')),
  idempotency_key     text NOT NULL UNIQUE,
  provider_message_id text,
  state               text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','sent','failed')),
  attempts            int NOT NULL DEFAULT 0,
  last_error          text,
  created_at          timestamptz NOT NULL DEFAULT now(),
  updated_at          timestamptz NOT NULL DEFAULT now()
);

-- -------------------------------------------------------------- owner audit ------
CREATE TABLE owner_audit (
  id          bigserial PRIMARY KEY,
  at          timestamptz NOT NULL DEFAULT now(),
  actor       text NOT NULL,                 -- owner auth uuid, or 'worker' / 'signer'
  action      text NOT NULL,
  target      text,
  before_json jsonb,
  after_json  jsonb,
  reason      text,
  request_id  text
);
-- Append-only for every application role.
CREATE OR REPLACE FUNCTION billing.forbid_audit_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'owner_audit is append-only'; END $$;
CREATE TRIGGER owner_audit_no_update BEFORE UPDATE OR DELETE ON owner_audit
  FOR EACH ROW EXECUTE FUNCTION billing.forbid_audit_change();

-- ------------------------------------------------------------- privileges ------
-- RLS on, no policies → browser/anon roles (if this DB is exposed through an API
-- layer) can read nothing; the server roles below are granted explicitly.
ALTER TABLE products ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing_customers ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing_schools ENABLE ROW LEVEL SECURITY;
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE payments ENABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE entitlements ENABLE ROW LEVEL SECURITY;
ALTER TABLE licence_issuances ENABLE ROW LEVEL SECURITY;
ALTER TABLE licence_transfers ENABLE ROW LEVEL SECURITY;
ALTER TABLE jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE owner_audit ENABLE ROW LEVEL SECURITY;

REVOKE ALL ON ALL TABLES IN SCHEMA billing FROM PUBLIC;
GRANT USAGE ON SCHEMA billing TO vidya_portal, vidya_worker, vidya_signer;

-- portal (owner dashboard + checkout API): read everything; create orders/customers;
-- audited owner actions; enqueue jobs.
GRANT SELECT ON ALL TABLES IN SCHEMA billing TO vidya_portal;
GRANT INSERT, UPDATE ON billing_customers, billing_schools, orders, licence_transfers, jobs, webhook_receipts TO vidya_portal;
GRANT INSERT ON owner_audit, entitlements, licence_issuances TO vidya_portal;
GRANT USAGE ON SEQUENCE owner_audit_id_seq TO vidya_portal;
-- worker: webhooks, payments, entitlements, jobs, deliveries.
GRANT SELECT ON ALL TABLES IN SCHEMA billing TO vidya_worker;
GRANT INSERT, UPDATE ON payments, webhook_receipts, entitlements, jobs, deliveries, orders TO vidya_worker;
GRANT INSERT ON licence_issuances, owner_audit TO vidya_worker;
GRANT USAGE ON SEQUENCE owner_audit_id_seq TO vidya_worker;
-- signer: can ONLY read entitlements/orders and fill in its issuance rows + jobs;
-- it cannot create payments or entitlements.
GRANT SELECT ON entitlements, orders, products, licence_issuances, jobs TO vidya_signer;
GRANT UPDATE (payload, signature, licence_key, vlic, status, last_error, issued_ts, signer_key_id) ON licence_issuances TO vidya_signer;
GRANT UPDATE (state, attempts, available_at, locked_by, lease_until, last_error, completed_at) ON jobs TO vidya_signer;
GRANT INSERT ON jobs, owner_audit TO vidya_signer;
GRANT USAGE ON SEQUENCE owner_audit_id_seq TO vidya_signer;
-- RLS policies: the three server roles see all rows (RLS still blocks any other role).
DO $$ DECLARE t text; BEGIN
  FOREACH t IN ARRAY ARRAY['products','billing_customers','billing_schools','orders','payments','webhook_receipts',
                           'entitlements','licence_issuances','licence_transfers','jobs','deliveries','owner_audit'] LOOP
    EXECUTE format('CREATE POLICY server_roles ON billing.%I TO vidya_portal, vidya_worker, vidya_signer USING (true) WITH CHECK (true)', t);
  END LOOP;
END $$;
