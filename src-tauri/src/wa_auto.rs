//! Automatic WhatsApp (optional module `wa_auto`, off by default) — P14 Step 7.
//!
//! The school (not Vidya) pays Meta and uses its OWN WhatsApp Business phone-number
//! id + access token + pre-approved **utility** templates. Verified against Meta's
//! official Cloud API docs — see `docs/phase-14-checks.md` §Check 3.
//!
//! **Not verified live** (needs the owner's Meta setup). What IS built + provable
//! offline: the template payload builder, the Meta error-code mapping, and a
//! rate-limited send loop behind a [`WaSender`] trait so the live `reqwest` client
//! and [`FakeWa`] are interchangeable (mirrors `email::GmailSender`). There is **no
//! webhook** (no public server, §STOP), so a send's honest final status is
//! **accepted by WhatsApp** (`status='sent'` + the returned `wamid`), never
//! "delivered".

use rusqlite::{params, Connection, OptionalExtension};

use crate::email::{mark_failed, mark_sent, SendReport};

/// Current documented Graph API version (Meta bumps this; kept in one place).
pub const GRAPH_API_VERSION: &str = "v23.0";

/// The Cloud API send endpoint for a phone-number id.
pub fn graph_send_url(phone_number_id: &str) -> String {
    format!("https://graph.facebook.com/{GRAPH_API_VERSION}/{phone_number_id}/messages")
}

/// The wa_auto config, read from `school.settings_json.wa_auto`. The access token
/// is protected by the SQLCipher-encrypted DB (like refresh tokens / PIN hashes)
/// and is NEVER logged.
#[derive(Debug, Clone, Default)]
pub struct WaAutoConfig {
    pub phone_number_id: String,
    pub token: String,
    /// Approved template name per `"<purpose>.<lang>"` (e.g. `fee_reminder.en`).
    pub templates: std::collections::BTreeMap<String, String>,
}

impl WaAutoConfig {
    pub fn read(conn: &Connection) -> rusqlite::Result<Option<WaAutoConfig>> {
        let raw: Option<String> = conn
            .query_row("SELECT settings_json FROM school LIMIT 1", [], |r| r.get(0))
            .optional()?;
        let v: serde_json::Value = raw.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
        let w = match v.get("wa_auto") {
            Some(w) => w,
            None => return Ok(None),
        };
        let phone_number_id = w.get("phone_number_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let token = w.get("token").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if phone_number_id.is_empty() || token.is_empty() {
            return Ok(None);
        }
        let mut templates = std::collections::BTreeMap::new();
        if let Some(map) = w.get("templates").and_then(|x| x.as_object()) {
            for (k, val) in map {
                if let Some(name) = val.as_str() {
                    templates.insert(k.clone(), name.to_string());
                }
            }
        }
        Ok(Some(WaAutoConfig { phone_number_id, token, templates }))
    }

    fn template_for(&self, purpose: &str, lang: &str) -> Option<&str> {
        self.templates
            .get(&format!("{purpose}.{lang}"))
            .or_else(|| self.templates.get(&format!("{purpose}.en")))
            .map(String::as_str)
    }
}

/// The template-message request body (a single body variable `{{1}}` = the rendered
/// text, so a school needs one simple utility template per purpose/language).
pub fn template_payload(to: &str, template: &str, lang: &str, body_param: &str) -> serde_json::Value {
    serde_json::json!({
        "messaging_product": "whatsapp",
        "to": to,
        "type": "template",
        "template": {
            "name": template,
            "language": { "code": lang },
            "components": [ { "type": "body", "parameters": [ { "type": "text", "text": body_param } ] } ]
        }
    })
}

/// A WhatsApp Cloud API failure, mapped from Meta's `error.code` (see checks §3c).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WaError {
    /// 190 / 0 / 200 — token expired/invalid/missing ("sign in again").
    #[error("wa token invalid")]
    TokenInvalid,
    /// 4 / 80007 / 130429 / 131056 — rate/throughput ("back off, keep queued").
    #[error("wa rate limited")]
    RateLimited,
    /// 131042 — the school's Meta billing / payment method.
    #[error("wa payment issue")]
    Payment,
    /// 132xxx — template variable mismatch / not approved in language / paused.
    #[error("wa template error {0}")]
    Template(i64),
    /// Per-recipient failure (131047 / 131026 / 131050 / other) — kept verbatim.
    #[error("wa undeliverable {0}")]
    Undeliverable(i64),
    /// Transport / other.
    #[error("wa io: {0}")]
    Io(String),
}

pub type WaResult<T> = Result<T, WaError>;

/// Map a Meta `error.code` to a [`WaError`] (checks §3c).
pub fn map_error_code(code: i64) -> WaError {
    match code {
        190 | 0 | 200 => WaError::TokenInvalid,
        4 | 80007 | 130429 | 131056 => WaError::RateLimited,
        131042 => WaError::Payment,
        132000 | 132001 | 132015 => WaError::Template(code),
        other => WaError::Undeliverable(other),
    }
}

/// The minimal Cloud API surface (send one template), so the live `reqwest` client
/// and [`FakeWa`] are interchangeable. Returns the `wamid` on success.
pub trait WaSender {
    fn send_template(&self, to: &str, template: &str, lang: &str, body_param: &str) -> WaResult<String>;
}

/// Normalise a 10-digit Indian mobile to the msisdn WhatsApp expects (`91<mobile>`);
/// a number already carrying a country code is returned digits-only.
fn to_msisdn(mobile: &str) -> Option<String> {
    let digits: String = mobile.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.len() {
        10 => Some(format!("91{digits}")),
        11..=15 => Some(digits),
        _ => None,
    }
}

/// Drain the `wa_auto` queue: build each template message, send, record `sent`
/// (accepted by WhatsApp, + wamid) or `failed` (+ exact error). Skips rows with no
/// mobile / no configured template; stops on cap / rate-limit / token / payment,
/// leaving rows `queued`. The live caller runs this rate-limited on a timer.
pub fn send_queued_wa_auto(
    conn: &mut Connection,
    sender: &dyn WaSender,
    cfg: &WaAutoConfig,
    cap: i64,
    now: &str,
) -> rusqlite::Result<SendReport> {
    let today = &now[..10.min(now.len())];
    let mut sent_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM message WHERE channel='wa_auto' AND status='sent' AND substr(sent_at,1,10)=?1",
        params![today], |r| r.get(0),
    )?;

    #[allow(clippy::type_complexity)]
    let queued: Vec<(String, Option<String>, Option<String>, Option<String>, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, to_address, body, kind, language FROM message \
             WHERE channel='wa_auto' AND status='queued' ORDER BY created_at",
        )?;
        let out = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        out
    };

    let mut report = SendReport::default();
    for (id, to_address, body, kind, lang) in queued {
        if sent_today >= cap {
            report.remaining += 1;
            continue;
        }
        let msisdn = to_address.as_deref().and_then(to_msisdn);
        let msisdn = match msisdn {
            Some(m) => m,
            None => { mark_failed(conn, &id, "no_mobile", now)?; report.skipped_no_email += 1; continue; }
        };
        // DPDP consent (§9): don't send a student-linked message to a guardian whose
        // `messages` consent was withdrawn after the row was queued. Mirrors the
        // email drain's check (record_message also gates at queue time).
        let (rel_tbl, rel_id): (Option<String>, Option<String>) = conn
            .query_row("SELECT related_table, related_id FROM message WHERE id=?1", params![id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
            .unwrap_or((None, None));
        if !crate::email::row_consented(conn, rel_tbl.as_deref(), rel_id.as_deref()) {
            mark_failed(conn, &id, "consent_withdrawn", now)?;
            report.skipped_no_consent += 1;
            continue;
        }
        let purpose = kind.as_deref().unwrap_or("");
        let template = match cfg.template_for(purpose, &lang) {
            Some(t) => t.to_string(),
            None => { mark_failed(conn, &id, "no_template", now)?; report.failed += 1; continue; }
        };
        let body_param = body.as_deref().unwrap_or("");
        match sender.send_template(&msisdn, &template, &lang, body_param) {
            Ok(wamid) => { mark_sent(conn, &id, &wamid, now)?; report.sent += 1; sent_today += 1; }
            Err(WaError::RateLimited | WaError::TokenInvalid | WaError::Payment) => { report.remaining += 1; break; }
            Err(e) => { mark_failed(conn, &id, &e.to_string(), now)?; report.failed += 1; }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    struct FakeWa {
        fail_at: std::collections::HashMap<usize, WaError>,
        sent: std::cell::RefCell<Vec<(String, String)>>,
    }
    impl FakeWa {
        fn ok() -> Self { FakeWa { fail_at: Default::default(), sent: Default::default() } }
    }
    impl WaSender for FakeWa {
        fn send_template(&self, to: &str, template: &str, _lang: &str, _body: &str) -> WaResult<String> {
            let n = self.sent.borrow().len();
            if let Some(e) = self.fail_at.get(&n) { return Err(e.clone()); }
            self.sent.borrow_mut().push((to.into(), template.into()));
            Ok(format!("wamid.TEST{n}"))
        }
    }

    fn conn() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }
    fn cfg() -> WaAutoConfig {
        let mut templates = std::collections::BTreeMap::new();
        templates.insert("fee_reminder.en".to_string(), "vidya_fee_reminder".to_string());
        WaAutoConfig { phone_number_id: "123456".into(), token: "TOKEN".into(), templates }
    }
    fn queue(c: &Connection, id: &str, to: Option<&str>, kind: &str) {
        c.execute(
            "INSERT INTO message(id,channel,language,to_address,body,kind,status,created_at,updated_at,sync_state) \
             VALUES(?1,'wa_auto','en',?2,'Body',?3,'queued','2026-09-23T09:00:00Z','2026-09-23T09:00:00Z','confirmed')",
            params![id, to, kind],
        ).unwrap();
    }

    #[test]
    fn payload_shape_and_url() {
        assert_eq!(graph_send_url("123"), "https://graph.facebook.com/v23.0/123/messages");
        let p = template_payload("919876543210", "vidya_fee_reminder", "en", "Pay 2100");
        assert_eq!(p["messaging_product"], "whatsapp");
        assert_eq!(p["template"]["name"], "vidya_fee_reminder");
        assert_eq!(p["template"]["components"][0]["parameters"][0]["text"], "Pay 2100");
    }

    #[test]
    fn error_code_mapping() {
        assert_eq!(map_error_code(190), WaError::TokenInvalid);
        assert_eq!(map_error_code(130429), WaError::RateLimited);
        assert_eq!(map_error_code(131042), WaError::Payment);
        assert_eq!(map_error_code(132001), WaError::Template(132001));
        assert_eq!(map_error_code(131047), WaError::Undeliverable(131047));
    }

    #[test]
    fn send_loop_accepts_and_records_wamid() {
        let mut c = conn();
        queue(&c, "m1", Some("9876543210"), "fee_reminder");
        let rep = send_queued_wa_auto(&mut c, &FakeWa::ok(), &cfg(), 400, "2026-09-24T06:00:00Z").unwrap();
        assert_eq!(rep.sent, 1);
        let (status, pref): (String, Option<String>) = c.query_row("SELECT status, provider_ref FROM message WHERE id='m1'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(status, "sent"); // = accepted by WhatsApp
        assert_eq!(pref.as_deref(), Some("wamid.TEST0"));
    }

    #[test]
    fn skips_no_mobile_and_no_template() {
        let mut c = conn();
        queue(&c, "m1", None, "fee_reminder");            // no mobile
        queue(&c, "m2", Some("9876543210"), "circular");  // no template configured
        let rep = send_queued_wa_auto(&mut c, &FakeWa::ok(), &cfg(), 400, "2026-09-24T06:00:00Z").unwrap();
        assert_eq!(rep.sent, 0);
        assert_eq!(rep.skipped_no_email + rep.failed, 2);
        let n: i64 = c.query_row("SELECT COUNT(*) FROM message WHERE status='failed'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn rate_limit_stops_batch_leaving_queued() {
        let mut c = conn();
        queue(&c, "m1", Some("9876543210"), "fee_reminder");
        queue(&c, "m2", Some("9876543211"), "fee_reminder");
        let fake = FakeWa { fail_at: [(0usize, WaError::RateLimited)].into_iter().collect(), sent: Default::default() };
        let rep = send_queued_wa_auto(&mut c, &fake, &cfg(), 400, "2026-09-24T06:00:00Z").unwrap();
        assert_eq!(rep.sent, 0);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM message WHERE status='queued'", [], |r| r.get::<_, i64>(0)).unwrap(), 2);
    }

    #[test]
    fn config_read_from_settings() {
        let c = conn();
        c.execute("INSERT INTO school(id,name,backup_salt,settings_json,created_at,updated_at,sync_state) VALUES('s','S',x'00',?1,'t','t','confirmed')",
            params![r#"{"wa_auto":{"phone_number_id":"111","token":"T","templates":{"fee_reminder.en":"tpl_fr"}}}"#]).unwrap();
        let cfg = WaAutoConfig::read(&c).unwrap().unwrap();
        assert_eq!(cfg.phone_number_id, "111");
        assert_eq!(cfg.template_for("fee_reminder", "hi"), Some("tpl_fr")); // falls back to .en
        assert_eq!(cfg.template_for("absence_alert", "en"), None);
        // No wa_auto key → None (module unconfigured).
        let c2 = conn();
        c2.execute("INSERT INTO school(id,name,backup_salt,settings_json,created_at,updated_at,sync_state) VALUES('s','S',x'00','{}','t','t','confirmed')", []).unwrap();
        assert!(WaAutoConfig::read(&c2).unwrap().is_none());
    }
}
