-- P21 (backups): a durable data-revision counter. Every write to a data table bumps
-- it (AFTER INSERT/UPDATE/DELETE triggers, installed by db::install_revision_triggers
-- after migrations so tables added later are covered too). The backup scheduler
-- compares it with the revision recorded by the last successful backup — the audit
-- head alone missed every write that isn't audited. Additive.
CREATE TABLE IF NOT EXISTS data_revision (
  id  INTEGER PRIMARY KEY CHECK (id = 1),
  rev INTEGER NOT NULL
);
INSERT OR IGNORE INTO data_revision(id, rev) VALUES (1, 0);
ALTER TABLE backup_run ADD COLUMN data_revision INTEGER;
