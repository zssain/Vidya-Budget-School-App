//! Email sender — school-PC-only Gmail send (P14 Step 3, §10.1).
//!
//! **Testing-mode build (OWNER-DECISIONS #13).** `gmail.send` is a Google
//! *Sensitive* scope: sending from a *published* app needs OAuth verification
//! (see `docs/phase-14-checks.md`). The owner runs the OAuth app in **Testing**
//! mode for now, so this module is built + unit-tested against a fake Gmail API
//! and wired behind [`GMAIL_SEND_SCOPE`], but the **live** send rides the P12-gated
//! real OAuth client (not built here) — so it is **not verified live** yet.
//!
//! What IS built + provable offline:
//! * [`build_mime`] — a hand-built RFC 2822 / MIME message (UTF-8 subject encoded
//!   per RFC 2047; `multipart/related` with an inline QR PNG via `cid:`;
//!   `multipart/mixed` for attachments) + [`to_raw`] (base64url for Gmail's `raw`).
//! * [`GmailSender`] — the minimal send surface, so the real client and
//!   [`FakeGmail`] are interchangeable (mirrors `sync::drive::DriveApi`).
//! * [`send_queued_emails`] — drains `channel='email' status='queued'` rows: builds
//!   MIME, sends, marks `sent` (+ provider id) or `failed` (+ exact error), skips
//!   guardians with no email / withdrawn consent, and honours the daily cap.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use rusqlite::{params, Connection, OptionalExtension};

/// The minimal OAuth scope to SEND email (verified, `docs/phase-14-checks.md`).
/// Added to the sync-account consent when the live client lands (re-consent).
pub const GMAIL_SEND_SCOPE: &str = "https://www.googleapis.com/auth/gmail.send";

/// Gmail API send endpoint (`userId=me`). Non-upload variant; the upload variant
/// (`/upload/gmail/v1/...`) is used for large MIME with attachments.
pub const GMAIL_SEND_ENDPOINT: &str = "https://gmail.googleapis.com/gmail/v1/users/me/messages/send";

/// Free consumer-Gmail cap is ~500 recipients/day; the default stays safely under.
pub const DEFAULT_DAILY_CAP: i64 = 400;

// ------------------------------------------------------------------ MIME ------

/// One file attachment (≤ 20 MB total across a message, enforced by the caller).
pub struct Attachment<'a> {
    pub filename: &'a str,
    pub mime: &'a str,
    pub bytes: &'a [u8],
}

/// Everything needed to build one MIME message. `date` (RFC 2822) and `boundary`
/// (a stable base, e.g. the message id) are passed in — no clock / no randomness,
/// so the output is deterministic and snapshot-testable.
pub struct EmailParts<'a> {
    pub from_name: &'a str,
    pub from_email: &'a str,
    pub to: &'a str,
    pub reply_to: Option<&'a str>,
    pub subject: &'a str,
    pub body_text: &'a str,
    /// Inline QR PNG bytes (referenced as `cid:qrcode` in the HTML alternative).
    pub qr_png: Option<&'a [u8]>,
    pub attachments: &'a [Attachment<'a>],
    pub date: &'a str,
    pub boundary: &'a str,
}

/// Base64 (standard, padded) wrapped to 76-char lines per RFC 2045.
fn b64_wrapped(bytes: &[u8]) -> String {
    let raw = STANDARD.encode(bytes);
    let mut out = String::with_capacity(raw.len() + raw.len() / 76 + 2);
    for (i, ch) in raw.chars().enumerate() {
        if i != 0 && i % 76 == 0 {
            out.push_str("\r\n");
        }
        out.push(ch);
    }
    out
}

/// Encode a header value per RFC 2047 (`=?UTF-8?B?…?=`) when it contains non-ASCII;
/// a pure-ASCII value is returned unchanged.
fn encode_header(s: &str) -> String {
    if s.is_ascii() {
        s.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(s.as_bytes()))
    }
}

/// Minimal HTML escape for the QR-carrying HTML alternative.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn text_part(body: &str) -> String {
    format!("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}", b64_wrapped(body.as_bytes()))
}

/// Build the raw RFC 2822 message. The structure adapts:
/// plain → `text/plain`; +QR → `multipart/related[ alternative[text,html], png ]`;
/// +attachments → wrap the above in `multipart/mixed`.
pub fn build_mime(p: &EmailParts) -> String {
    let content = if let Some(qr) = p.qr_png {
        let html = format!(
            "<html><body><p>{}</p><p><img src=\"cid:qrcode\" alt=\"UPI QR\"></p></body></html>",
            html_escape(p.body_text).replace('\n', "<br>")
        );
        let html_part = format!(
            "Content-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
            b64_wrapped(html.as_bytes())
        );
        let alt_b = format!("{}-alt", p.boundary);
        let alt = multipart("alternative", &alt_b, &[text_part(p.body_text), html_part]);
        let img = format!(
            "Content-Type: image/png\r\nContent-Transfer-Encoding: base64\r\nContent-ID: <qrcode>\r\nContent-Disposition: inline; filename=\"qr.png\"\r\n\r\n{}",
            b64_wrapped(qr)
        );
        let rel_b = format!("{}-rel", p.boundary);
        multipart("related", &rel_b, &[alt, img])
    } else {
        text_part(p.body_text)
    };

    let content = if p.attachments.is_empty() {
        content
    } else {
        let mut parts = vec![content];
        for a in p.attachments {
            parts.push(format!(
                "Content-Type: {}; name=\"{}\"\r\nContent-Transfer-Encoding: base64\r\nContent-Disposition: attachment; filename=\"{}\"\r\n\r\n{}",
                a.mime, a.filename, a.filename, b64_wrapped(a.bytes)
            ));
        }
        let mix_b = format!("{}-mix", p.boundary);
        multipart("mixed", &mix_b, &parts)
    };

    let mut headers = String::new();
    headers.push_str(&format!("From: {} <{}>\r\n", encode_header(p.from_name), p.from_email));
    headers.push_str(&format!("To: {}\r\n", p.to));
    if let Some(rt) = p.reply_to {
        headers.push_str(&format!("Reply-To: {rt}\r\n"));
    }
    headers.push_str(&format!("Subject: {}\r\n", encode_header(p.subject)));
    headers.push_str(&format!("Date: {}\r\n", p.date));
    headers.push_str("MIME-Version: 1.0\r\n");
    // `content` is a full MIME entity (its own Content-Type header + body).
    format!("{headers}{content}")
}

/// Assemble a `multipart/<subtype>` entity from full child entities.
fn multipart(subtype: &str, boundary: &str, parts: &[String]) -> String {
    let mut out = format!("Content-Type: multipart/{subtype}; boundary=\"{boundary}\"\r\n\r\n");
    for part in parts {
        out.push_str(&format!("--{boundary}\r\n{part}\r\n"));
    }
    out.push_str(&format!("--{boundary}--\r\n"));
    out
}

/// The Gmail `raw` field: base64url (no padding) of the whole RFC 2822 message.
pub fn to_raw(mime: &str) -> String {
    URL_SAFE_NO_PAD.encode(mime.as_bytes())
}

// ---------------------------------------------------------------- sender ------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GmailError {
    /// 429 / rateLimitExceeded — back off, leave the row queued, continue later.
    #[error("gmail rate limited")]
    RateLimited,
    /// The account's daily send quota is exhausted — stop for today.
    #[error("gmail daily limit reached")]
    DailyLimit,
    /// 401 — the OAuth token was revoked/expired ("sign in again").
    #[error("gmail token revoked")]
    TokenRevoked,
    /// Any other transport/API failure (kept verbatim as the row's error).
    #[error("gmail io: {0}")]
    Io(String),
}

pub type GmailResult<T> = Result<T, GmailError>;

/// The minimal Gmail send surface, so the live `reqwest` client and [`FakeGmail`]
/// are interchangeable (like `sync::drive::DriveApi`). The implementation carries
/// the acting identity (the sync account's access token).
pub trait GmailSender {
    /// Send a base64url `raw` message; returns Gmail's message id on success.
    fn send(&self, raw: &str) -> GmailResult<String>;
}

/// Outcome of one drain of the queue.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SendReport {
    pub sent: i64,
    pub failed: i64,
    pub skipped_no_email: i64,
    pub skipped_no_consent: i64,
    /// Left queued (cap reached / rate-limited / token revoked) for next time.
    pub remaining: i64,
}

/// True if the student still has active `messages` consent (P13). A row whose
/// `related_table='student'` carries the student id; other rows are treated as
/// consented (they came from an authorised flow).
pub(crate) fn row_consented(conn: &Connection, related_table: Option<&str>, related_id: Option<&str>) -> bool {
    match (related_table, related_id) {
        (Some("student"), Some(sid)) => crate::commands::logic::messages_consent_logic(conn, sid).unwrap_or(false),
        _ => true,
    }
}

/// Drain the email outbox: build + send each `queued` email, honestly recording
/// the result. `sender` is the identity; `cap` is the day's remaining budget;
/// `now`/`date` are passed in (no clock). Status changes append a server op so
/// devices see the honest status. Stops early on cap / rate-limit / revoked token.
#[allow(clippy::too_many_arguments)]
pub fn send_queued_emails(
    conn: &mut Connection,
    sender: &dyn GmailSender,
    from_name: &str,
    from_email: &str,
    reply_to: Option<&str>,
    cap: i64,
    now: &str,
    date: &str,
) -> rusqlite::Result<SendReport> {
    let today = &now[..10.min(now.len())];
    let mut sent_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM message WHERE channel='email' AND status='sent' AND substr(sent_at,1,10)=?1",
        params![today], |r| r.get(0),
    )?;

    #[allow(clippy::type_complexity)]
    let queued: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT id, to_address, subject, body, related_table FROM message \
             WHERE channel='email' AND status='queued' ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let mut report = SendReport::default();
    for (id, to_address, subject, body, related_table) in queued {
        // Cap reached → leave the rest queued for the next day (visible note).
        if sent_today >= cap {
            report.remaining += 1;
            continue;
        }
        let related_id: Option<String> = conn
            .query_row("SELECT related_id FROM message WHERE id=?1", params![id], |r| r.get(0))
            .optional()?
            .flatten();
        let addr = to_address.as_deref().map(str::trim).filter(|s| !s.is_empty());
        if addr.is_none() {
            mark_failed(conn, &id, "no_email", now)?;
            report.skipped_no_email += 1;
            continue;
        }
        if !row_consented(conn, related_table.as_deref(), related_id.as_deref()) {
            mark_failed(conn, &id, "consent_withdrawn", now)?;
            report.skipped_no_consent += 1;
            continue;
        }
        let mime = build_mime(&EmailParts {
            from_name,
            from_email,
            to: addr.unwrap(),
            reply_to,
            subject: subject.as_deref().unwrap_or(""),
            body_text: body.as_deref().unwrap_or(""),
            qr_png: None,
            attachments: &[],
            date,
            boundary: &id,
        });
        match sender.send(&to_raw(&mime)) {
            Ok(provider_id) => {
                mark_sent(conn, &id, &provider_id, now)?;
                report.sent += 1;
                sent_today += 1;
            }
            Err(GmailError::RateLimited | GmailError::DailyLimit | GmailError::TokenRevoked) => {
                // Transient / auth: stop the batch, leave this + the rest queued.
                report.remaining += 1;
                break;
            }
            Err(GmailError::Io(e)) => {
                mark_failed(conn, &id, &e, now)?;
                report.failed += 1;
            }
        }
    }
    Ok(report)
}

/// Gather the send config from the DB: `(from_name, from_email, reply_to, cap)`.
/// `from_name` = school name; `from_email` = the connected sync account's Gmail;
/// `reply_to` = the school email if set; `cap` = `settings_json.email_daily_cap`
/// clamped to `[1, 500]` (default 400). `None` when there is no connected sync
/// account to send from.
#[allow(clippy::type_complexity)]
pub fn email_config(conn: &Connection) -> rusqlite::Result<Option<(String, String, Option<String>, i64)>> {
    let school: Option<(String, Option<String>)> = conn
        .query_row("SELECT name, settings_json FROM school LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let (name, settings) = match school {
        Some(s) => s,
        None => return Ok(None),
    };
    let from_email: Option<String> = conn
        .query_row("SELECT email FROM drive_account WHERE kind='sync' AND status='connected'", [], |r| r.get(0))
        .optional()?
        .flatten();
    let from_email = match from_email.filter(|e| !e.trim().is_empty()) {
        Some(e) => e,
        None => return Ok(None),
    };
    let v: serde_json::Value = settings.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
    let cap = v.get("email_daily_cap").and_then(|x| x.as_i64()).unwrap_or(DEFAULT_DAILY_CAP).clamp(1, 500);
    let reply_to = v.get("email").and_then(|x| x.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    Ok(Some((name, from_email, reply_to, cap)))
}

/// School-PC background-task entry point: read config + drain the email queue.
/// `None` (no-op) when there is no connected sync account. The live caller runs
/// this on a timer with a real `reqwest` Gmail client + a 1 msg / 2 s pace; that
/// client rides the P12-gated OAuth flow and is **not wired/verified here**.
pub fn drain_queue(conn: &mut Connection, sender: &dyn GmailSender, now: &str, date: &str) -> rusqlite::Result<Option<SendReport>> {
    let (name, from_email, reply_to, cap) = match email_config(conn)? {
        Some(c) => c,
        None => return Ok(None),
    };
    Ok(Some(send_queued_emails(conn, sender, &name, &from_email, reply_to.as_deref(), cap, now, date)?))
}

/// Append a server op so devices sync a message-status change. Shared by the email
/// and wa_auto senders.
pub(crate) fn op_for_message_update(conn: &Connection, id: &str, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO op_log(op_id, hlc, device_id, staff_id, \"table\", record_id, kind, payload, applied_at) \
         VALUES (?1,?2,'','','message',?3,'update','{}',?2)",
        params![crate::commands::logic::new_id("op"), now, id],
    )?;
    Ok(())
}

/// Mark a message `sent` with the provider id (Gmail id / WhatsApp wamid). For
/// wa_auto "sent" means **accepted by WhatsApp** (no webhook ⇒ delivery unknown).
pub(crate) fn mark_sent(conn: &Connection, id: &str, provider_id: &str, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE message SET status='sent', provider_ref=?2, sent_at=?3, error=NULL, updated_at=?3 WHERE id=?1",
        params![id, provider_id, now],
    )?;
    op_for_message_update(conn, id, now)
}

pub(crate) fn mark_failed(conn: &Connection, id: &str, error: &str, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE message SET status='failed', error=?2, updated_at=?3 WHERE id=?1",
        params![id, error, now],
    )?;
    op_for_message_update(conn, id, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeGmail {
        /// Fail the send whose 0-based index is in here, with the given error.
        fail_at: std::collections::HashMap<usize, GmailError>,
        sent: std::cell::RefCell<Vec<String>>,
    }
    impl FakeGmail {
        fn ok() -> Self {
            FakeGmail { fail_at: Default::default(), sent: Default::default() }
        }
    }
    impl GmailSender for FakeGmail {
        fn send(&self, raw: &str) -> GmailResult<String> {
            let n = self.sent.borrow().len();
            if let Some(e) = self.fail_at.get(&n) {
                return Err(e.clone());
            }
            self.sent.borrow_mut().push(raw.to_string());
            Ok(format!("gmail-msg-{n}"))
        }
    }

    #[test]
    fn mime_plain_ascii() {
        let mime = build_mime(&EmailParts {
            from_name: "Green Valley School", from_email: "vidya.gv@gmail.com", to: "p@example.com",
            reply_to: None, subject: "Fee reminder", body_text: "Dear parent, please pay.",
            qr_png: None, attachments: &[], date: "Wed, 23 Sep 2026 11:42:00 +0000", boundary: "msg-1",
        });
        assert!(mime.contains("From: Green Valley School <vidya.gv@gmail.com>\r\n"));
        assert!(mime.contains("Subject: Fee reminder\r\n"));
        assert!(mime.contains("Content-Type: text/plain; charset=UTF-8\r\n"));
        assert_eq!(STANDARD.decode("RGVhciBwYXJlbnQsIHBsZWFzZSBwYXku").unwrap(), b"Dear parent, please pay.");
    }

    #[test]
    fn subject_rfc2047_for_hindi_and_telugu() {
        // Hindi subject → =?UTF-8?B?...?= ; decodes back to the original.
        let hi = "फ़ीस याद‑दिलावा";
        let m = build_mime(&EmailParts {
            from_name: "स्कूल", from_email: "s@gmail.com", to: "p@e.com", reply_to: None,
            subject: hi, body_text: "x", qr_png: None, attachments: &[], date: "d", boundary: "b",
        });
        let line = m.lines().find(|l| l.starts_with("Subject: ")).unwrap();
        let enc = line.trim_start_matches("Subject: ").trim_start_matches("=?UTF-8?B?").trim_end_matches("?=");
        assert_eq!(String::from_utf8(STANDARD.decode(enc).unwrap()).unwrap(), hi);
        // From-name (also non-ASCII) is encoded too.
        assert!(m.contains("From: =?UTF-8?B?"));
    }

    #[test]
    fn mime_related_with_inline_qr_and_mixed_attachment() {
        let png = b"\x89PNG\r\n\x1a\nfake";
        let pdf = b"%PDF-1.4 fake";
        let m = build_mime(&EmailParts {
            from_name: "S", from_email: "s@g.com", to: "p@e.com", reply_to: Some("office@school.in"),
            subject: "Reminder", body_text: "Scan the QR.", qr_png: Some(png),
            attachments: &[Attachment { filename: "dues.pdf", mime: "application/pdf", bytes: pdf }],
            date: "d", boundary: "msg-9",
        });
        assert!(m.contains("Reply-To: office@school.in\r\n"));
        assert!(m.contains("multipart/mixed; boundary=\"msg-9-mix\""));
        assert!(m.contains("multipart/related; boundary=\"msg-9-rel\""));
        assert!(m.contains("multipart/alternative; boundary=\"msg-9-alt\""));
        assert!(m.contains("Content-ID: <qrcode>"));
        assert!(m.contains("Content-Disposition: attachment; filename=\"dues.pdf\""));
    }

    #[test]
    fn to_raw_is_url_safe_base64() {
        let raw = to_raw("From: a@b\r\n\r\nhi");
        assert!(!raw.contains('+') && !raw.contains('/') && !raw.contains('='));
        assert_eq!(URL_SAFE_NO_PAD.decode(raw).unwrap(), b"From: a@b\r\n\r\nhi");
    }

    // ---- send loop (needs a DB) ------------------------------------------
    use crate::db;
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    fn conn() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }
    fn queue_email(c: &Connection, id: &str, to: Option<&str>) {
        c.execute(
            "INSERT INTO message(id,channel,language,to_address,subject,body,status,created_at,updated_at,sync_state) \
             VALUES(?1,'email','en',?2,'Sub','Body','queued','2026-09-23T09:00:00Z','2026-09-23T09:00:00Z','confirmed')",
            params![id, to],
        ).unwrap();
    }

    #[test]
    fn send_loop_marks_sent_and_records_provider_id() {
        let mut c = conn();
        queue_email(&c, "m1", Some("a@e.com"));
        queue_email(&c, "m2", Some("b@e.com"));
        let rep = send_queued_emails(&mut c, &FakeGmail::ok(), "School", "s@g.com", None, 400, "2026-09-24T06:00:00Z", "d").unwrap();
        assert_eq!(rep.sent, 2);
        let (status, pref): (String, Option<String>) = c.query_row("SELECT status, provider_ref FROM message WHERE id='m1'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(status, "sent");
        assert_eq!(pref.as_deref(), Some("gmail-msg-0"));
        // A status op was logged so devices sync the honest 'sent'.
        let ops: i64 = c.query_row("SELECT COUNT(*) FROM op_log WHERE \"table\"='message'", [], |r| r.get(0)).unwrap();
        assert_eq!(ops, 2);
    }

    #[test]
    fn send_loop_skips_no_email_and_respects_cap() {
        let mut c = conn();
        queue_email(&c, "m1", None); // no email → skipped
        queue_email(&c, "m2", Some("b@e.com"));
        queue_email(&c, "m3", Some("d@e.com"));
        // Cap of 1 → one sent, one left queued.
        let rep = send_queued_emails(&mut c, &FakeGmail::ok(), "S", "s@g.com", None, 1, "2026-09-24T06:00:00Z", "d").unwrap();
        assert_eq!(rep.skipped_no_email, 1);
        assert_eq!(rep.sent, 1);
        assert_eq!(rep.remaining, 1);
        let queued: i64 = c.query_row("SELECT COUNT(*) FROM message WHERE status='queued'", [], |r| r.get(0)).unwrap();
        assert_eq!(queued, 1);
    }

    #[test]
    fn drain_queue_noop_without_sync_account_then_sends() {
        let mut c = conn();
        c.execute("INSERT INTO school(id,name,backup_salt,settings_json,created_at,updated_at,sync_state) VALUES('sch','Green Valley',x'00',?1,'t','t','confirmed')",
            params![r#"{"email_daily_cap":2}"#]).unwrap();
        queue_email(&c, "m1", Some("a@e.com"));
        // No connected sync account → no-op (nothing to send from).
        assert!(drain_queue(&mut c, &FakeGmail::ok(), "2026-09-24T06:00:00Z", "d").unwrap().is_none());
        assert_eq!(c.query_row("SELECT status FROM message WHERE id='m1'", [], |r| r.get::<_, String>(0)).unwrap(), "queued");
        // Connect the sync account → the queue drains.
        c.execute("INSERT INTO drive_account(kind,email,status) VALUES('sync','vidya.gv@gmail.com','connected')", []).unwrap();
        let rep = drain_queue(&mut c, &FakeGmail::ok(), "2026-09-24T06:00:00Z", "d").unwrap().unwrap();
        assert_eq!(rep.sent, 1);
    }

    #[test]
    fn send_loop_stops_on_rate_limit_leaving_rows_queued() {
        let mut c = conn();
        queue_email(&c, "m1", Some("a@e.com"));
        queue_email(&c, "m2", Some("b@e.com"));
        let fake = FakeGmail { fail_at: [(0usize, GmailError::RateLimited)].into_iter().collect(), sent: Default::default() };
        let rep = send_queued_emails(&mut c, &fake, "S", "s@g.com", None, 400, "2026-09-24T06:00:00Z", "d").unwrap();
        assert_eq!(rep.sent, 0);
        assert_eq!(rep.remaining, 1); // stopped at the first; the rest stay queued
        let queued: i64 = c.query_row("SELECT COUNT(*) FROM message WHERE status='queued'", [], |r| r.get(0)).unwrap();
        assert_eq!(queued, 2);
    }
}
