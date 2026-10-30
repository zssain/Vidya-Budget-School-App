-- P21 (Drive join v2): a 256-bit secret per invite for the iPhone/web route. It rides
-- in the web invite's URL fragment and is never uploaded; the joining device proves
-- it holds it (HMAC) and the school PC seals the response under a key derived from
-- it. Kept only in this encrypted DB (needed to verify proofs); cleared on use.
-- Additive. Older invites (NULL) can't be used for a web join — issue a new invite.
ALTER TABLE invite ADD COLUMN web_secret TEXT;
