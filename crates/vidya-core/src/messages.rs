//! messages — messaging-engine rules (P13, foundation §8.6). Pure: no IO, no clock.
//!
//! An outbox `message` carries a channel, a template, a language, a recipient and
//! an honest status; a `message_template` holds the subject/body per (key,
//! language) with named `{placeholders}`. This module validates a template's
//! placeholders against a per-template allow-list, renders a body by substituting
//! variables, and answers `can_message` (consent). **No sending happens here** —
//! Phase 14 adds the channels; P13 only builds and validates.

use std::collections::BTreeMap;

use crate::errors::{CoreError, CoreResult};

/// Delivery channel (`message.channel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Email,
    WaTap,
    WaAuto,
    App,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Email => "email",
            Channel::WaTap => "wa_tap",
            Channel::WaAuto => "wa_auto",
            Channel::App => "app",
        }
    }
    pub fn parse(s: &str) -> Option<Channel> {
        Some(match s {
            "email" => Channel::Email,
            "wa_tap" => Channel::WaTap,
            "wa_auto" => Channel::WaAuto,
            "app" => Channel::App,
            _ => return None,
        })
    }
}

/// Honest message status (`message.status`). No status ever claims a send that did
/// not happen (§3 rule 13): `tapped` means the share sheet / wa.me link was opened,
/// not that the parent received it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Draft,
    Queued,
    Sent,
    Tapped,
    Failed,
    Read,
}

impl MessageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageStatus::Draft => "draft",
            MessageStatus::Queued => "queued",
            MessageStatus::Sent => "sent",
            MessageStatus::Tapped => "tapped",
            MessageStatus::Failed => "failed",
            MessageStatus::Read => "read",
        }
    }
    pub fn parse(s: &str) -> Option<MessageStatus> {
        Some(match s {
            "draft" => MessageStatus::Draft,
            "queued" => MessageStatus::Queued,
            "sent" => MessageStatus::Sent,
            "tapped" => MessageStatus::Tapped,
            "failed" => MessageStatus::Failed,
            "read" => MessageStatus::Read,
            _ => return None,
        })
    }
}

/// The seed templates (§8.6). Each key has a fixed allow-list of placeholders.
pub const TEMPLATE_KEYS: &[&str] =
    &["absence_alert", "fee_reminder", "receipt_share", "circular", "homework"];

/// The placeholders a template body may use. A body using anything outside this
/// list fails [`validate_template`] (so a typo can't silently render blank).
pub fn allowed_placeholders(template_key: &str) -> &'static [&'static str] {
    match template_key {
        "absence_alert" => &["student_name", "date", "school_name"],
        "fee_reminder" => &["student_name", "amount", "instalment", "due_date", "school_name", "upi_link"],
        "receipt_share" => &["student_name", "receipt_no", "amount", "school_name"],
        "circular" => &["title", "body", "school_name"],
        "homework" => &["class", "subject", "homework", "date", "school_name"],
        _ => &[],
    }
}

/// The `{placeholder}` names used in `body`, in order of first appearance.
pub fn placeholders_in(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = body[i + 1..].find('}') {
                let name = &body[i + 1..i + 1 + end];
                // A placeholder name is a non-empty run of [a-z0-9_].
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
                    if !out.iter().any(|n| n == name) {
                        out.push(name.to_string());
                    }
                    i += end + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Every `{placeholder}` in `body` must be in the template's allow-list, and the
/// template key must be known.
pub fn validate_template(template_key: &str, body: &str) -> CoreResult<()> {
    if !TEMPLATE_KEYS.contains(&template_key) {
        return Err(CoreError::validation("template_key", "unknown"));
    }
    let allowed = allowed_placeholders(template_key);
    for p in placeholders_in(body) {
        if !allowed.contains(&p.as_str()) {
            return Err(CoreError::validation("placeholder", "not_allowed"));
        }
    }
    Ok(())
}

/// Render `body` by substituting `{name}` with `vars[name]`. Placeholders with no
/// matching variable are left intact (so a missing value is visible, never a
/// silent blank). Variables are already formatted by the caller (Indian ₹/date
/// formats, §3 rule 10).
pub fn render(body: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = body.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// May a guardian be messaged at all? Requires `messages` consent (§9, DPDP). The
/// consent flag is read from the `consent` table (Step 12) by the caller.
pub fn can_message(has_messages_consent: bool) -> bool {
    has_messages_consent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn channel_and_status_round_trip() {
        for c in [Channel::Email, Channel::WaTap, Channel::WaAuto, Channel::App] {
            assert_eq!(Channel::parse(c.as_str()), Some(c));
        }
        for s in [MessageStatus::Draft, MessageStatus::Queued, MessageStatus::Sent, MessageStatus::Tapped, MessageStatus::Failed, MessageStatus::Read] {
            assert_eq!(MessageStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(Channel::parse("sms"), None);
    }

    #[test]
    fn extracts_placeholders_in_order_without_dupes() {
        assert_eq!(
            placeholders_in("Hi {student_name}, {student_name} was absent on {date}."),
            vec!["student_name".to_string(), "date".to_string()]
        );
        // Braces that aren't valid placeholders are ignored.
        assert_eq!(placeholders_in("nothing here {Bad Name} {} { } end"), Vec::<String>::new());
    }

    #[test]
    fn validates_against_the_allow_list() {
        assert!(validate_template("absence_alert", "{student_name} absent on {date}").is_ok());
        // A placeholder not on the list is rejected.
        assert_eq!(
            validate_template("absence_alert", "{student_name} owes {amount}").unwrap_err().code(),
            "VALIDATION"
        );
        // Unknown template key.
        assert_eq!(validate_template("nope", "hi").unwrap_err().code(), "VALIDATION");
    }

    #[test]
    fn renders_known_placeholders_and_keeps_unknown_visible() {
        let body = "Dear parent, {student_name} owes {amount} due {due_date}.";
        let out = render(body, &vars(&[("student_name", "Riya"), ("amount", "₹3,100")]));
        assert_eq!(out, "Dear parent, Riya owes ₹3,100 due {due_date}.");
    }

    #[test]
    fn can_message_requires_consent() {
        assert!(can_message(true));
        assert!(!can_message(false));
    }

    #[test]
    fn every_template_key_has_placeholders() {
        for k in TEMPLATE_KEYS {
            assert!(!allowed_placeholders(k).is_empty(), "{k} needs placeholders");
        }
    }
}
