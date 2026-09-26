-- 0026_v2_report_remarks.sql — Phase 16 (v2): report-card remarks (§10.4, Step 4).
--
-- Additive only. Belongs to the `classroom` module (default ON, §14). The class
-- teacher enters one remark per student per exam; it is locked once the Principal
-- makes the exam's report cards final (report_lock row exists → remarks read-only,
-- the same lock pattern as marks). report_template holds neutral suggestions in
-- en/hi/te (Settings → Report cards); the 10 seeded × 3 languages are DRAFTS marked
-- for native-speaker review (OWNER-DECISIONS #12).

CREATE TABLE IF NOT EXISTS report_remark (
  id           TEXT PRIMARY KEY,
  exam_id      TEXT NOT NULL REFERENCES exam(id),
  student_id   TEXT NOT NULL REFERENCES student(id),
  text         TEXT NOT NULL DEFAULT '',
  template_key TEXT,
  author       TEXT REFERENCES staff(id),
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed',
  UNIQUE (exam_id, student_id)
);
CREATE INDEX IF NOT EXISTS idx_report_remark_exam ON report_remark(exam_id);

-- One row per exam once the Principal makes its report cards final (locks remarks).
CREATE TABLE IF NOT EXISTS report_lock (
  exam_id      TEXT PRIMARY KEY REFERENCES exam(id),
  finalised_by TEXT REFERENCES staff(id),
  finalised_at TEXT NOT NULL,
  school_id    TEXT REFERENCES school(id),
  version INTEGER NOT NULL DEFAULT 1, hlc TEXT, created_at TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT '', updated_by_staff TEXT, updated_by_device TEXT,
  sync_state TEXT NOT NULL DEFAULT 'confirmed'
);

-- Neutral remark suggestions (10 keys × en/hi/te). DRAFT — native review pending.
CREATE TABLE IF NOT EXISTS report_template (
  id       TEXT PRIMARY KEY,
  key      TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('en','hi','te')),
  text     TEXT NOT NULL,
  UNIQUE (key, language)
);

INSERT INTO report_template(id, key, language, text) VALUES
  ('rt-attentive-en','attentive','en','A keen and attentive learner who participates well in class.'),
  ('rt-attentive-hi','attentive','hi','कक्षा में अच्छी तरह भाग लेने वाला एक उत्सुक और चौकस विद्यार्थी।'),
  ('rt-attentive-te','attentive','te','తరగతిలో చక్కగా పాల్గొనే ఆసక్తిగల, శ్రద్ధగల విద్యార్థి.'),
  ('rt-improving-en','improving','en','Has shown steady improvement this term. Keep it up.'),
  ('rt-improving-hi','improving','hi','इस सत्र में निरंतर सुधार दिखाया है। इसे बनाए रखें।'),
  ('rt-improving-te','improving','te','ఈ టర్మ్‌లో స్థిరమైన మెరుగుదల చూపింది. ఇలాగే కొనసాగించండి.'),
  ('rt-homework-en','homework','en','Completes homework regularly and neatly.'),
  ('rt-homework-hi','homework','hi','गृहकार्य नियमित और साफ-सुथरा पूरा करता है।'),
  ('rt-homework-te','homework','te','గృహపాఠాన్ని క్రమం తప్పకుండా, శుభ్రంగా పూర్తి చేస్తారు.'),
  ('rt-practice-en','practice','en','More practice at home will help build confidence.'),
  ('rt-practice-hi','practice','hi','घर पर अधिक अभ्यास से आत्मविश्वास बढ़ेगा।'),
  ('rt-practice-te','practice','te','ఇంట్లో ఎక్కువ అభ్యాసం ఆత్మవిశ్వాసాన్ని పెంచుతుంది.'),
  ('rt-attendance-en','attendance','en','Regular attendance will help further progress.'),
  ('rt-attendance-hi','attendance','hi','नियमित उपस्थिति आगे की प्रगति में मदद करेगी।'),
  ('rt-attendance-te','attendance','te','క్రమమైన హాజరు మరింత పురోగతికి సహాయపడుతుంది.'),
  ('rt-reading-en','reading','en','Encourage daily reading to strengthen language skills.'),
  ('rt-reading-hi','reading','hi','भाषा कौशल मजबूत करने के लिए रोज़ पढ़ने को प्रोत्साहित करें।'),
  ('rt-reading-te','reading','te','భాషా నైపుణ్యాలను బలోపేతం చేయడానికి రోజువారీ చదవడాన్ని ప్రోత్సహించండి.'),
  ('rt-helpful-en','helpful','en','Polite, helpful and respectful towards teachers and friends.'),
  ('rt-helpful-hi','helpful','hi','शिक्षकों और मित्रों के प्रति विनम्र, सहायक और आदरपूर्ण।'),
  ('rt-helpful-te','helpful','te','ఉపాధ్యాయులు, స్నేహితుల పట్ల మర్యాదగా, సహాయకరంగా, గౌరవంగా ఉంటారు.'),
  ('rt-focus-en','focus','en','Can improve by focusing more during lessons.'),
  ('rt-focus-hi','focus','hi','पाठ के दौरान अधिक ध्यान देकर सुधार कर सकता है।'),
  ('rt-focus-te','focus','te','పాఠాల సమయంలో ఎక్కువ దృష్టి పెట్టడం ద్వారా మెరుగుపడవచ్చు.'),
  ('rt-creative-en','creative','en','Shows creativity and asks thoughtful questions.'),
  ('rt-creative-hi','creative','hi','रचनात्मकता दिखाता है और विचारशील प्रश्न पूछता है।'),
  ('rt-creative-te','creative','te','సృజనాత్మకత చూపి, ఆలోచనాత్మక ప్రశ్నలు అడుగుతారు.'),
  ('rt-potential-en','potential','en','Has good potential; consistent effort will bring better results.'),
  ('rt-potential-hi','potential','hi','अच्छी क्षमता है; निरंतर प्रयास से बेहतर परिणाम मिलेंगे।'),
  ('rt-potential-te','potential','te','మంచి సామర్థ్యం ఉంది; స్థిరమైన కృషి మెరుగైన ఫలితాలను తెస్తుంది.');
