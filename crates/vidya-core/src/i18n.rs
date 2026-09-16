use std::{collections::BTreeMap, sync::OnceLock};

use crate::roles::Lang;

/// Every translation key emitted by this crate.
pub const REQUIRED_KEYS: &[&str] = &[
    "attendance.error.future",
    "attendance.error.past_read_only",
    "date.error.invalid",
    "fees.error.amount",
    "fees.error.cheque_reference",
    "fees.error.no_balance",
    "fees.error.over_balance",
    "fees.error.rte",
    "fees.error.student_not_active",
    "fees.error.upi_reference",
    "fees.error.upi_used",
    "hlc.error.invalid",
    "marks.error.grade_scale",
    "marks.error.invalid",
    "money.error.invalid",
    "password.contains_username",
    "password.same_as_current",
    "password.too_short",
    "permission.denied",
    "permission.needs_section",
    "permission.no_sections",
    "permission.unknown_action",
    "session.error.invalid",
    "validation.class_name",
    "validation.mobile",
    "validation.name",
    "validation.reason",
    "validation.receipt_prefix",
    "validation.school_code",
    "validation.udise",
    "validation.username",
];

fn english() -> &'static BTreeMap<String, String> {
    static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MESSAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../locales/en.json"))
            .expect("vidya-core English locale must be valid JSON")
    })
}

fn hindi() -> &'static BTreeMap<String, String> {
    static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MESSAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../locales/hi.json"))
            .expect("vidya-core Hindi locale must be valid JSON")
    })
}

/// Looks up and interpolates a message, falling back from Hindi to English to the key.
pub fn message(lang: Lang, key: &str, params: &BTreeMap<String, String>) -> String {
    render_message(lang, key, params, english(), hindi())
}

fn render_message(
    lang: Lang,
    key: &str,
    params: &BTreeMap<String, String>,
    en: &BTreeMap<String, String>,
    hi: &BTreeMap<String, String>,
) -> String {
    let template = match lang {
        Lang::En => en.get(key),
        Lang::Hi => hi.get(key).or_else(|| en.get(key)),
    };
    let mut rendered = template.cloned().unwrap_or_else(|| key.to_owned());
    for (name, value) in params {
        rendered = rendered.replace(&format!("{{{name}}}"), value);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_contain_every_required_key() {
        assert_eq!(
            english().keys().collect::<Vec<_>>(),
            hindi().keys().collect::<Vec<_>>()
        );
        for key in REQUIRED_KEYS {
            assert!(english().contains_key(*key), "missing English key {key}");
            assert!(hindi().contains_key(*key), "missing Hindi key {key}");
        }
    }

    #[test]
    fn replaces_placeholders_and_falls_back() {
        let params = BTreeMap::from([("balance".to_owned(), "₹9,100".to_owned())]);
        assert_eq!(
            message(Lang::En, "fees.error.over_balance", &params),
            "That is more than the balance of ₹9,100."
        );
        let en = BTreeMap::from([("english.only".to_owned(), "English fallback".to_owned())]);
        assert_eq!(
            render_message(Lang::Hi, "english.only", &BTreeMap::new(), &en, &BTreeMap::new()),
            "English fallback"
        );
        assert_eq!(message(Lang::En, "missing.key", &BTreeMap::new()), "missing.key");
    }
}
