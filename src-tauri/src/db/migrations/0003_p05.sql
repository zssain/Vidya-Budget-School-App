-- Phase 5 additions (prompts/P05). Internet access through the relay + end-to-end
-- sealing + replay defence. Only new columns on the existing `device` table; no
-- earlier migration is edited (Standing Rule 8).

-- The per-device session key agreed at join (docs §9 / P05 Step 2). Phase 4 already
-- returned it to the device in JoinResp; the SERVER must now KEEP it so it can seal
-- and unseal relay traffic to/from that device. Base64 of 32 random bytes; NULL for
-- devices that joined before this migration (they re-seal after their next join).
ALTER TABLE device ADD COLUMN session_key TEXT;

-- Replay defence (P05 Step 2): a strictly-monotonic request counter carried INSIDE
-- each sealed request. The server rejects a counter <= the last one it accepted from
-- that device. This is the high-water mark; starts at 0 (the first real counter is 1).
ALTER TABLE device ADD COLUMN last_counter INTEGER NOT NULL DEFAULT 0;
