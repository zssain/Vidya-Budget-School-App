//! UPI payment settings + QR rendering (P14, §10.1).
//!
//! The UPI id / display name and the three "show the QR on…" toggles live in
//! `school.settings_json` (alongside `phone`). The deep-link string itself is
//! built by the pure `vidya_core::upi::upi_uri`; here we read the school's
//! settings and render a link as an SVG QR with the existing `qrcode` crate
//! (navy on white, like the invite QR). No PNG is produced in Rust — email turns
//! the SVG into a PNG in the webview canvas (§12, "no new crate").

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

/// Payments configuration, parsed from `school.settings_json`. Absent keys default
/// to "off"/none so a school that never set a UPI id simply shows no QR anywhere.
/// A sample balance (₹2,100) used only to render the Settings → Payments preview
/// QR, matching the prototype's "Scan to pay ₹2,100" preview.
const PREVIEW_PAISE: i64 = 210000;

#[derive(Debug, Clone, Default, Serialize)]
pub struct PaymentSettings {
    pub upi_id: Option<String>,
    pub upi_name: Option<String>,
    pub on_receipts: bool,
    pub on_reminders: bool,
    pub on_dues_list: bool,
    /// A sample `upi://pay` link (₹2,100) for the Settings preview QR — computed
    /// by [`PaymentSettings::read`], `None` when no UPI id is set. Not persisted.
    #[serde(default)]
    pub preview_link: Option<String>,
}

impl PaymentSettings {
    /// Read the payments settings from the single `school` row (or defaults), with
    /// the preview link computed for the Settings screen.
    pub fn read(conn: &Connection) -> rusqlite::Result<PaymentSettings> {
        let raw: Option<String> = conn
            .query_row("SELECT settings_json FROM school LIMIT 1", [], |r| r.get(0))
            .optional()?;
        let mut s = raw.map(|s| Self::from_json(&s)).unwrap_or_default();
        s.preview_link = s.link(PREVIEW_PAISE, "Fees · Term balance");
        Ok(s)
    }

    /// Parse the settings blob; unknown / malformed JSON yields defaults.
    pub fn from_json(s: &str) -> PaymentSettings {
        let v: serde_json::Value = serde_json::from_str(s).unwrap_or(serde_json::Value::Null);
        let str_field = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
        };
        let bool_field = |k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
        PaymentSettings {
            upi_id: str_field("upi_id"),
            upi_name: str_field("upi_name"),
            on_receipts: bool_field("upi_on_receipts"),
            on_reminders: bool_field("upi_on_reminders"),
            on_dues_list: bool_field("upi_on_dues_list"),
            preview_link: None,
        }
    }

    /// Build the `upi://pay` link for one payment of `amount_paise` with `note`,
    /// or `None` if no UPI id is configured. `amount_paise <= 0` yields an
    /// amount-editable link (per NPCI).
    pub fn link(&self, amount_paise: i64, note: &str) -> Option<String> {
        let vpa = self.upi_id.as_deref()?;
        // The stored VPA was validated on save; re-validate defensively so a
        // hand-edited DB can never emit a malformed link.
        let vpa = vidya_core::upi::validate_vpa(vpa).ok()?;
        let name = self.upi_name.as_deref().unwrap_or("");
        Some(vidya_core::upi::upi_uri(&vpa, name, amount_paise, note))
    }
}

/// Render `data` as an SVG QR code, navy (`#0C1B38`) on white — the app's brand
/// QR, matching the invite QR. Fails only if `data` is too long to encode.
pub fn qr_svg(data: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(data.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .dark_color(qrcode::render::svg::Color("#0C1B38"))
        .light_color(qrcode::render::svg::Color("#FFFFFF"))
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_settings_and_defaults() {
        let s = PaymentSettings::from_json(
            r#"{"phone":"9876543210","upi_id":"school@okhdfcbank","upi_name":"Green Valley","upi_on_receipts":true}"#,
        );
        assert_eq!(s.upi_id.as_deref(), Some("school@okhdfcbank"));
        assert_eq!(s.upi_name.as_deref(), Some("Green Valley"));
        assert!(s.on_receipts);
        assert!(!s.on_reminders);
        assert!(!s.on_dues_list);
        // Empty / missing → None / false.
        let d = PaymentSettings::from_json(r#"{"upi_id":"  "}"#);
        assert!(d.upi_id.is_none());
        assert!(!d.on_receipts);
        assert!(PaymentSettings::from_json("not json").upi_id.is_none());
    }

    #[test]
    fn link_needs_a_valid_vpa() {
        let none = PaymentSettings::default();
        assert!(none.link(210000, "Fees").is_none());
        let ok = PaymentSettings {
            upi_id: Some("school@okhdfcbank".into()),
            upi_name: Some("Green Valley".into()),
            on_receipts: true,
            ..Default::default()
        };
        let link = ok.link(210000, "Fees Kavya VI-B").unwrap();
        assert!(link.starts_with("upi://pay?pa=school@okhdfcbank&pn=Green%20Valley&am=2100.00&cu=INR&tn="));
        // A garbage stored VPA never emits a link.
        let bad = PaymentSettings { upi_id: Some("nothandle".into()), ..Default::default() };
        assert!(bad.link(1, "x").is_none());
    }

    #[test]
    fn qr_renders_svg() {
        let svg = qr_svg("upi://pay?pa=school@okhdfcbank&pn=X&am=1.00&cu=INR").unwrap();
        assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
        assert!(svg.contains("#0C1B38"));
    }
}
