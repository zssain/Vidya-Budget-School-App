-- 0013_v2_messaging.sql — Phase 13 (v2): messaging engine (foundation §8.6).
--
-- Additive only. `message` is the outbox (channel/status honest — no fake sends);
-- `message_template` holds the subject/body per (key, language) with named
-- {placeholders} validated in vidya-core::messages. NO sending happens in P13 —
-- Phase 14 adds the channels. Hindi and Telugu template bodies are DRAFTS pending
-- native-speaker review (flagged in the P13 handoff).

CREATE TABLE IF NOT EXISTS message (
  id               TEXT PRIMARY KEY,
  kind             TEXT,                       -- purpose (usually the template key)
  channel          TEXT NOT NULL CHECK (channel IN ('email','wa_tap','wa_auto','app')),
  template_key     TEXT,
  language         TEXT NOT NULL DEFAULT 'en' CHECK (language IN ('en','hi','te')),
  to_guardian_id   TEXT REFERENCES guardian(id),
  to_staff_id      TEXT REFERENCES staff(id),
  to_address       TEXT,
  subject          TEXT,
  body             TEXT,
  attachments_json TEXT,
  status           TEXT NOT NULL DEFAULT 'draft'
    CHECK (status IN ('draft','queued','sent','tapped','failed','read')),
  error            TEXT,
  related_table    TEXT,
  related_id       TEXT,
  created_by       TEXT,
  sent_at          TEXT,
  provider_ref     TEXT,
  school_id        TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'on_device'
);

CREATE INDEX IF NOT EXISTS idx_message_status  ON message(status);
CREATE INDEX IF NOT EXISTS idx_message_related ON message(related_table, related_id);

-- Templates: id-keyed (for the sync engine) with a UNIQUE (key, language).
CREATE TABLE IF NOT EXISTS message_template (
  id         TEXT PRIMARY KEY,
  key        TEXT NOT NULL,
  language   TEXT NOT NULL CHECK (language IN ('en','hi','te')),
  subject    TEXT,
  body       TEXT NOT NULL,
  updated_by TEXT,
  school_id  TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (key, language)
);

-- Seed the five templates in en (authoritative) / hi / te (drafts). Placeholders
-- match vidya-core::messages::allowed_placeholders.
INSERT OR IGNORE INTO message_template(id, key, language, subject, body) VALUES
  ('tpl-absence_alert-en','absence_alert','en','Absent today',
   'Dear parent, {student_name} was marked absent on {date}. — {school_name}'),
  ('tpl-absence_alert-hi','absence_alert','hi','आज अनुपस्थित',
   'प्रिय अभिभावक, {student_name} को {date} को अनुपस्थित दर्ज किया गया। — {school_name}'),
  ('tpl-absence_alert-te','absence_alert','te','ఈరోజు గైర్హాజరు',
   'ప్రియమైన తల్లిదండ్రులారా, {student_name} {date} నాడు గైర్హాజరుగా నమోదయ్యారు. — {school_name}'),

  ('tpl-fee_reminder-en','fee_reminder','en','Fee reminder',
   'Dear parent, {amount} is due for {student_name} by {due_date}. Pay: {upi_link} — {school_name}'),
  ('tpl-fee_reminder-hi','fee_reminder','hi','फीस अनुस्मारक',
   'प्रिय अभिभावक, {student_name} की {amount} फीस {due_date} तक बकाया है। भुगतान करें: {upi_link} — {school_name}'),
  ('tpl-fee_reminder-te','fee_reminder','te','ఫీజు గుర్తుచేయుట',
   'ప్రియమైన తల్లిదండ్రులారా, {student_name} కోసం {amount} ఫీజు {due_date} లోపు చెల్లించవలెను. చెల్లించండి: {upi_link} — {school_name}'),

  ('tpl-receipt_share-en','receipt_share','en','Fee receipt',
   'Receipt {receipt_no}: {amount} received for {student_name}. Thank you. — {school_name}'),
  ('tpl-receipt_share-hi','receipt_share','hi','फीस रसीद',
   'रसीद {receipt_no}: {student_name} के लिए {amount} प्राप्त हुए। धन्यवाद। — {school_name}'),
  ('tpl-receipt_share-te','receipt_share','te','ఫీజు రసీదు',
   'రసీదు {receipt_no}: {student_name} కోసం {amount} స్వీకరించబడింది. ధన్యవాదాలు. — {school_name}'),

  ('tpl-circular-en','circular','en','{title}',
   '{body} — {school_name}'),
  ('tpl-circular-hi','circular','hi','{title}',
   '{body} — {school_name}'),
  ('tpl-circular-te','circular','te','{title}',
   '{body} — {school_name}'),

  ('tpl-homework-en','homework','en','Homework · {class} {subject}',
   '{class} {subject} homework ({date}): {homework} — {school_name}'),
  ('tpl-homework-hi','homework','hi','गृहकार्य · {class} {subject}',
   '{class} {subject} गृहकार्य ({date}): {homework} — {school_name}'),
  ('tpl-homework-te','homework','te','ఇంటి పని · {class} {subject}',
   '{class} {subject} ఇంటి పని ({date}): {homework} — {school_name}');
