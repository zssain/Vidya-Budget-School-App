-- Vidya licence service — schema (prompts/P10 Step 2).
-- Plain SQLite (rusqlite `bundled`); operational data, not school data. Backups
-- are encrypted at the ops layer (Step 7). Times are ISO-8601 UTC text; money is
-- integer paise; ids are UUIDv7 text unless noted.
--
-- Adaptations from the Step-2 list, all documented in docs/phase-notes/phase-10.md:
--  * `order` is named `purchase_order` (ORDER is a SQL keyword).
--  * The purchase model is manual UPI (owner decision): the buyer scans a QR and
--    pays, then a company admin verifies the payment and issues the licence. So
--    an order gains the `awaiting_verification` status, and `payment_event` records
--    buyer claims and admin verifications instead of provider webhook events.

PRAGMA foreign_keys = ON;

-- A purchaser (one per checkout; email is not unique — a school may buy twice).
CREATE TABLE customer (
  id          TEXT PRIMARY KEY,
  email       TEXT NOT NULL,
  name        TEXT NOT NULL,
  phone       TEXT NOT NULL,
  school_name TEXT NOT NULL,
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_customer_email ON customer (email);

-- One purchase. `provider_order_id` is the public order reference shown to the
-- buyer (VIDYA-ORD-XXXX...), UNIQUE for idempotency. amount/currency/plan are
-- placeholders until the owner sets prices ([Price]); the manual flow does not
-- require them to be non-zero.
CREATE TABLE purchase_order (
  id                TEXT PRIMARY KEY,
  customer_id       TEXT NOT NULL REFERENCES customer (id),
  provider          TEXT NOT NULL DEFAULT 'upi_manual',
  provider_order_id TEXT NOT NULL UNIQUE,
  amount_paise      INTEGER NOT NULL DEFAULT 0,
  currency          TEXT NOT NULL DEFAULT 'INR',
  plan              TEXT NOT NULL DEFAULT 'perpetual',
  -- created → awaiting_verification (buyer says paid) → paid (admin verified)
  --         → failed (admin rejected/expired) → refunded
  status            TEXT NOT NULL DEFAULT 'created'
                     CHECK (status IN ('created','awaiting_verification','paid','failed','refunded')),
  upi_ref           TEXT,               -- buyer-submitted UPI reference / UTR (a claim, never proof)
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL
);
CREATE INDEX idx_order_status ON purchase_order (status);
CREATE INDEX idx_order_customer ON purchase_order (customer_id);

-- Every payment-relevant event. `provider_event_id` UNIQUE makes ingestion
-- idempotent (a repeated buyer claim or a double admin-verify collapses to one
-- row). `verified` = the admin confirmed money actually arrived.
CREATE TABLE payment_event (
  id                TEXT PRIMARY KEY,
  provider_event_id TEXT NOT NULL UNIQUE,
  order_id          TEXT NOT NULL REFERENCES purchase_order (id),
  kind              TEXT NOT NULL CHECK (kind IN ('buyer_claim','admin_verify','admin_reject','refund')),
  amount_paise      INTEGER,
  upi_ref           TEXT,
  actor             TEXT NOT NULL,      -- 'buyer' or an admin email
  verified          INTEGER NOT NULL DEFAULT 0,
  raw_json          TEXT NOT NULL,      -- the exact submitted/recorded payload
  received_at       TEXT NOT NULL
);
CREATE INDEX idx_event_order ON payment_event (order_id);

-- The signed licence register. The signed JSON returned to the app is exactly the
-- §10 field set (licence_id, school_id, plan, max_students|null, max_devices|null,
-- issued_at, server_machine_id); the extra columns here are company-side records.
CREATE TABLE licence (
  licence_id            TEXT PRIMARY KEY,
  school_id             TEXT NOT NULL UNIQUE,
  order_id              TEXT UNIQUE REFERENCES purchase_order (id),
  plan                  TEXT NOT NULL DEFAULT 'perpetual',
  max_students          INTEGER,         -- NULL = unlimited
  max_devices           INTEGER,         -- NULL = unlimited
  status                TEXT NOT NULL DEFAULT 'active'
                          CHECK (status IN ('active','revoked','moved')),
  issued_at             TEXT NOT NULL,
  server_machine_id     TEXT,            -- NULL until first activation; rebinds on transfer
  server_epoch          INTEGER NOT NULL DEFAULT 1,   -- +1 on every transfer (§8.9)
  -- Self-service transfer: the app registers a recovery-derived verifier key at
  -- activation; stored ENCRYPTED at rest (chacha20poly1305, server key) so a
  -- DB-only leak cannot forge a transfer proof. NULL = self-service unavailable
  -- (older app / not registered) → admin-approved transfer only.
  recovery_verifier_enc TEXT,
  app_version_last_seen  TEXT,
  last_check_at         TEXT,
  revoked_reason        TEXT,
  revoked_at            TEXT,
  created_at            TEXT NOT NULL
);
CREATE INDEX idx_licence_order ON licence (order_id);

-- An activation code. `code_hash` (SHA-256) is the lookup key at /v1/activate;
-- `code_enc` is the code encrypted at rest so the Account page can re-display it
-- (owner decision, P10). Reissue revokes the old row and inserts a new one.
CREATE TABLE activation_code (
  id                  TEXT PRIMARY KEY,
  code_hash           TEXT NOT NULL UNIQUE,
  code_enc            TEXT NOT NULL,      -- base64(nonce ‖ ChaCha20-Poly1305(server_key, code))
  licence_id          TEXT NOT NULL REFERENCES licence (licence_id),
  order_id            TEXT NOT NULL REFERENCES purchase_order (id),
  redeemed_at         TEXT,
  redeemed_machine_id TEXT,
  revoked_at          TEXT,
  created_at          TEXT NOT NULL
);
CREATE INDEX idx_code_licence ON activation_code (licence_id);
CREATE INDEX idx_code_order ON activation_code (order_id);

-- Every machine rebind (restore to a new PC / support transfer).
CREATE TABLE transfer (
  id             TEXT PRIMARY KEY,
  licence_id     TEXT NOT NULL REFERENCES licence (licence_id),
  old_machine_id TEXT,
  new_machine_id TEXT NOT NULL,
  at             TEXT NOT NULL,
  method         TEXT NOT NULL CHECK (method IN ('self_service','admin')),
  admin_email    TEXT,
  reason         TEXT
);
CREATE INDEX idx_transfer_licence ON transfer (licence_id);

-- Company staff who can sign into /admin. Bootstrapped from env on first start.
CREATE TABLE admin_user (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,           -- Argon2id PHC string
  role          TEXT NOT NULL DEFAULT 'staff' CHECK (role IN ('owner','staff')),
  disabled      INTEGER NOT NULL DEFAULT 0,
  failed_count  INTEGER NOT NULL DEFAULT 0,
  locked_until  TEXT,
  created_at    TEXT NOT NULL
);

-- Admin sessions. The cookie holds a random token; we store only its SHA-256.
-- CSRF token is per-session and required on every mutating form.
CREATE TABLE admin_session (
  id          TEXT PRIMARY KEY,          -- SHA-256 of the session cookie value
  admin_id    TEXT NOT NULL REFERENCES admin_user (id),
  csrf_token  TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  expires_at  TEXT NOT NULL
);
CREATE INDEX idx_session_admin ON admin_session (admin_id);

-- Append-only, hash-chained admin audit (same recipe as the app, §9):
-- hash = SHA-256(prev_hash ‖ canonical-JSON of the entry). Triggers below make
-- UPDATE/DELETE abort, so the chain is tamper-evident.
CREATE TABLE admin_audit (
  seq         INTEGER PRIMARY KEY AUTOINCREMENT,
  at          TEXT NOT NULL,
  admin_email TEXT NOT NULL,
  action      TEXT NOT NULL,
  target_type TEXT,
  target_id   TEXT,
  reason      TEXT,
  details_json TEXT NOT NULL DEFAULT '{}',
  prev_hash   TEXT NOT NULL,
  hash        TEXT NOT NULL
);
CREATE TRIGGER admin_audit_no_update BEFORE UPDATE ON admin_audit
  BEGIN SELECT RAISE(ABORT, 'admin_audit is append-only'); END;
CREATE TRIGGER admin_audit_no_delete BEFORE DELETE ON admin_audit
  BEGIN SELECT RAISE(ABORT, 'admin_audit is append-only'); END;

-- Relay-secret record per school (Step 2/5). NOTE (P05 stateless relay): the
-- effective secret is HMAC-SHA256(RELAY_SHARED_KEY, school_id), which cloud/relay
-- recomputes without state, so what is returned by the API cannot be rotated
-- per-school without a P05 change. This table records issuance + rotation intent
-- (hash + rotated_at) and the admin "rotate" action is audited; true rotation is
-- an open owner decision (documented in the handoff).
CREATE TABLE relay_secret (
  id          TEXT PRIMARY KEY,
  licence_id  TEXT NOT NULL REFERENCES licence (licence_id),
  secret_hash TEXT NOT NULL,             -- SHA-256 of the issued relay_secret
  generation  INTEGER NOT NULL DEFAULT 1,
  active      INTEGER NOT NULL DEFAULT 1,
  rotated_at  TEXT,
  created_at  TEXT NOT NULL
);
CREATE INDEX idx_relay_secret_licence ON relay_secret (licence_id);
