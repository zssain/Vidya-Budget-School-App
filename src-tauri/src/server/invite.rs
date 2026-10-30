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

/// Build the PWA web-invite URL `https://<pwa>/#d=<base64url JSON>` (join v2). The
/// PWA reads a **minimal** payload in the URL **fragment** (never sent to the host):
/// `{v:2, invite_id, secret, school_name, school_id}`. `secret` is the invite's
/// 256-bit web secret; the short code and LAN facts are NOT included — the browser
/// joins through Drive by proving it holds the secret (vidya_core::seal join v2).
pub fn build_web_join_link(payload: &JoinPayload, invite_id: &str, web_secret: &str) -> Result<String, String> {
    let json = serde_json::to_vec(&serde_json::json!({
        "v": 2,
        "invite_id": invite_id,
        "secret": web_secret,
        "school_name": payload.school_name,
        "school_id": payload.school_id,
    }))
    .map_err(|e| e.to_string())?;
    let origin = crate::config::pwa_origin().ok_or_else(|| "no PWA origin configured for this build".to_string())?;
    let link = format!("{origin}/#d={}", URL_SAFE_NO_PAD.encode(&json));
    if link.len() > MAX_LINK_BYTES {
        return Err(format!("web join link too large ({} > {MAX_LINK_BYTES} bytes)", link.len()));
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
    fn web_join_link_carries_invite_code_in_fragment() {
        let link = build_web_join_link(&payload(), "inv-1", "c2VjcmV0").unwrap();
        assert!(link.starts_with(&crate::config::pwa_origin().unwrap()));
        let d = link.split("/#d=").nth(1).expect("has #d= fragment");
        let json = URL_SAFE_NO_PAD.decode(d.as_bytes()).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        // Join v2: invite id + web secret, never the short code.
        assert_eq!(v["v"], 2);
        assert_eq!(v["invite_id"], "inv-1");
        assert_eq!(v["secret"], "c2VjcmV0");
        assert_eq!(v["school_name"], "Saraswati Public School");
        assert_eq!(v["school_id"], "sch-demo");
        assert!(v.get("invite_code").is_none() && v.get("code").is_none());
        // Minimal payload — no LAN facts leak into the browser link.
        assert!(v.get("lan_addrs").is_none());
        assert!(v.get("cert_sha256").is_none());
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
