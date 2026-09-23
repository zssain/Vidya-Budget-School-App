//! Invitation deep-link + QR (prompts/P04 Step 5). The Principal's invite screen
//! renders a link `vidya://join?d=<base64url JSON JoinPayload>` (≤ 2 KB) and a QR
//! SVG; the join flow parses either. The 8-char code + hash live in `service`.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use crate::sync::protocol::JoinPayload;

const SCHEME_PREFIX: &str = "vidya://join?d=";
/// Deep-link payload hard cap (§5).
const MAX_LINK_BYTES: usize = 2048;

/// Build the `vidya://join?d=…` deep link for an invite payload.
pub fn build_join_link(payload: &JoinPayload) -> Result<String, String> {
    let json = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let link = format!("{SCHEME_PREFIX}{}", URL_SAFE_NO_PAD.encode(&json));
    if link.len() > MAX_LINK_BYTES {
        return Err(format!("join link too large ({} > {MAX_LINK_BYTES} bytes)", link.len()));
    }
    Ok(link)
}

/// Parse (and schema-validate) a `vidya://join?d=…` deep link.
pub fn parse_join_link(url: &str) -> Result<JoinPayload, String> {
    if url.len() > MAX_LINK_BYTES {
        return Err("join link too large".into());
    }
    let d = url.strip_prefix(SCHEME_PREFIX).ok_or("not a vidya join link")?;
    let json = URL_SAFE_NO_PAD.decode(d.as_bytes()).map_err(|_| "bad base64url payload".to_string())?;
    serde_json::from_slice::<JoinPayload>(&json).map_err(|e| format!("bad join payload: {e}"))
}

/// Render an invite link as an SVG QR code (navy on white), for the invite screen.
pub fn qr_svg(link: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(link.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(220, 220)
        .dark_color(qrcode::render::svg::Color("#0C1B38"))
        .light_color(qrcode::render::svg::Color("#FFFFFF"))
        .build())
}

/// The first 8 fingerprint chars shown on the invite screen (code-only joins) and
/// confirmed by the joiner before `/join` (§5).
pub fn short_fingerprint(fingerprint: &str) -> String {
    fingerprint.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> JoinPayload {
        JoinPayload {
            school_id: "sch-demo".into(),
            school_name: "Saraswati Public School".into(),
            lan_addrs: vec!["192.168.1.5".into(), "192.168.1.6".into()],
            port: 47650,
            cert_sha256: "ab12cd34ef56".repeat(4),
            relay_url: None,
            code: "VIDYA123".into(),
        }
    }

    #[test]
    fn join_link_round_trips_and_is_small() {
        let link = build_join_link(&payload()).unwrap();
        assert!(link.starts_with("vidya://join?d="));
        assert!(link.len() <= MAX_LINK_BYTES);
        let back = parse_join_link(&link).unwrap();
        assert_eq!(back, payload());
    }

    #[test]
    fn parse_rejects_non_vidya_links() {
        assert!(parse_join_link("https://example.com").is_err());
        assert!(parse_join_link("vidya://join?d=!!!notbase64!!!").is_err());
    }

    #[test]
    fn qr_svg_renders() {
        let link = build_join_link(&payload()).unwrap();
        let svg = qr_svg(&link).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("#0C1B38"));
    }

    #[test]
    fn short_fingerprint_is_eight_chars() {
        assert_eq!(short_fingerprint("abcdef0123456789"), "abcdef01");
    }
}
