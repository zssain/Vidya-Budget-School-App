-- Phase 12 (v2): two Google accounts + the server's epoch-signing key.
-- Additive only; no committed migration is edited; no financial/audit rows touched.

-- Step 1: the school's two Drive connections.
--   sync   = the shared school sync account (exchange bundles, acks, epoch.json,
--            class notes) — every staff device signs into it.
--   backup = the Principal's PRIVATE account (encrypted daily backups) — school PC only.
-- v1 installs had ONE Principal Drive connection used for both exchange and backups;
-- it is migrated to kind='backup' below so backups keep working, and Home then shows
-- "Connect the school sync account" until a sync account is connected (Step 1.2).
-- token_enc holds the OAuth refresh token (the DB is SQLCipher-encrypted at rest).
CREATE TABLE IF NOT EXISTS drive_account (
  kind               TEXT PRIMARY KEY CHECK (kind IN ('sync','backup')),
  email              TEXT,
  token_enc          BLOB,
  root_folder_id     TEXT,
  exchange_folder_id TEXT,
  notes_folder_id    TEXT,
  backups_folder_id  TEXT,
  connected_at       TEXT,
  status             TEXT NOT NULL DEFAULT 'connected'
);

-- Migrate a v1 Principal Drive connection (if one was ever stored as KV) to
-- kind='backup'. v1 never persisted Drive OAuth tokens in a table (the real client
-- was gated on the Phase 6 spike), so in practice there is nothing to copy here;
-- the app's accounts-model code performs the one-time KV→row migration if a legacy
-- connection value exists. This statement is a no-op placeholder kept for clarity.

-- Step 5: the school server's ed25519 key pair for signing exchange/epoch.json.
-- The private seed is stored here (the DB is SQLCipher-encrypted at rest, so it is
-- "encrypted in the DB"); the public key is shipped to devices at join so they can
-- verify the epoch marker. Generated at setup on the server PC.
ALTER TABLE school ADD COLUMN server_key_pub BLOB;
ALTER TABLE school ADD COLUMN server_key_priv_enc BLOB;
