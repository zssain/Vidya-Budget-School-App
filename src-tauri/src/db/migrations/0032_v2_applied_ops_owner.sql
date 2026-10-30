-- P21 (server authorization): bind each remembered op to the device that sent it and
-- the exact content it carried. Dedup was by op_id alone, so a device that could read a
-- peer's bundle could pre-empt that peer's op_id with a different payload and have the
-- real op "already applied" (rejected) forever. Now a repeat of an op_id from another
-- device, or with different content, is refused without touching the stored result.
-- Additive; existing rows keep NULLs (treated as "owner unknown" → only an identical
-- retry is served from cache).
ALTER TABLE applied_ops ADD COLUMN device_id TEXT;
ALTER TABLE applied_ops ADD COLUMN payload_hash TEXT;
