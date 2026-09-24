//! Server-rendered HTML shared by the website and the admin panel. Written by
//! hand, styled with the app's design tokens (docs/01-MOCK-SPEC §2) and the
//! Welcome-screen look — navy panel, serif headline, teal buttons, brand logo.
//!
//! Fonts: the website uses a serif/sans SYSTEM stack (no web-font fetch, matching
//! the app's "never load fonts from the web" ethos). Self-hosting the exact brand
//! faces (Newsreader / Geist) is a documented follow-up.

use crate::config::Config;

/// Minimal, safe HTML-escaping for text interpolated into pages.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Attribute-value escaping (same rules; kept separate for readability at call sites).
pub fn attr(s: &str) -> String {
    escape(s)
}

/// The single stylesheet, built from the mock tokens.
pub const CSS: &str = r#"
:root{
  --bg:#f5f7f6; --bg-outer:#e4eceb; --surface:#fdfdfb; --white:#ffffff;
  --panel:#e2eaeb; --navy:#0c1b38; --navy-deep:#08152b; --navy-raised:#1c3358;
  --ink:#13233f; --muted:#56657a; --on-navy:#c9d2de; --on-navy-muted:#9facbf;
  --on-navy-strong:#e8edf4; --line:#d5dde0; --line-strong:#c9d3d2;
  --accent:#2f7479; --accent-hover:#245c60; --gold:#c5ab7a; --gold-text:#8c6a2f;
  --danger:#c0392b; --online:#5fd0a0;
  --pill-partpaid-bg:#f4ecdc; --pill-partpaid-fg:#6b5220;
  --pill-unpaid-bg:#f6e4e2; --pill-unpaid-fg:#8e2f2a;
  --pill-marks-bg:#e7e3f1; --pill-marks-fg:#4a3b78;
  --serif:'Newsreader',Georgia,'Times New Roman',serif;
  --sans:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;
  --radius:16px; --radius-sm:10px;
}
*{box-sizing:border-box}
html,body{margin:0}
body{background:var(--bg-outer);color:var(--ink);font-family:var(--sans);
  font-size:16px;line-height:1.55;-webkit-font-smoothing:antialiased}
a{color:var(--accent);text-decoration:none}
a:hover{text-decoration:underline}
.wrap{max-width:1040px;margin:0 auto;padding:0 24px}
header.site{background:var(--surface);border-bottom:1px solid var(--line)}
header.site .wrap{display:flex;align-items:center;justify-content:space-between;height:68px}
header.site img.logo{height:30px;display:block}
nav.site a{color:var(--muted);font-size:14px;margin-left:22px;font-weight:500}
nav.site a.active,nav.site a:hover{color:var(--ink);text-decoration:none}
main{padding:40px 0 64px}
h1{font-family:var(--serif);font-weight:600;font-size:44px;line-height:1.08;margin:0 0 10px;letter-spacing:-0.01em}
h2{font-family:var(--serif);font-weight:600;font-size:26px;margin:0 0 12px}
h3{font-size:16px;margin:0 0 8px}
.eyebrow{text-transform:uppercase;letter-spacing:0.14em;font-size:11px;font-weight:600;color:var(--gold-text);margin-bottom:12px}
.sub{color:var(--muted);font-size:17px;max-width:60ch}
.panel-navy{background:var(--navy);color:var(--on-navy-strong);border-radius:22px;padding:44px 40px;position:relative;overflow:hidden}
.panel-navy h1{color:#fff}
.panel-navy .sub{color:var(--on-navy)}
.panel-navy .eyebrow{color:var(--gold)}
.card{background:var(--surface);border:1px solid var(--line);border-radius:var(--radius);padding:24px}
.grid{display:grid;gap:18px}
.grid.cols-3{grid-template-columns:repeat(3,1fr)}
.grid.cols-2{grid-template-columns:repeat(2,1fr)}
@media(max-width:820px){.grid.cols-3,.grid.cols-2{grid-template-columns:1fr}h1{font-size:34px}}
.btn{display:inline-block;border:none;border-radius:var(--radius-sm);padding:12px 22px;
  font-size:15px;font-weight:600;cursor:pointer;font-family:var(--sans)}
.btn-primary{background:var(--accent);color:#fff}
.btn-primary:hover{background:var(--accent-hover);text-decoration:none}
.btn-ghost{background:transparent;color:var(--on-navy-strong);border:1px solid rgba(255,255,255,.35)}
.btn-line{background:var(--white);color:var(--ink);border:1px solid var(--line-strong)}
.mt{margin-top:18px}.mt2{margin-top:32px}.mb{margin-bottom:18px}
.row{display:flex;gap:12px;align-items:center;flex-wrap:wrap}
.muted{color:var(--muted)}.small{font-size:13px}
label{display:block;font-size:13px;font-weight:600;color:var(--ink);margin:14px 0 6px}
input[type=text],input[type=email],input[type=tel],input[type=password],select,textarea{
  width:100%;padding:11px 13px;border:1px solid var(--line-strong);border-radius:var(--radius-sm);
  font-size:15px;font-family:var(--sans);background:var(--white);color:var(--ink)}
input:focus,select:focus,textarea:focus{outline:2px solid var(--accent);outline-offset:0;border-color:var(--accent)}
table{width:100%;border-collapse:collapse}
th{text-transform:uppercase;font-size:10px;letter-spacing:0.08em;color:var(--muted);
  text-align:left;padding:10px 12px;border-bottom:1px solid var(--line)}
td{padding:14px 12px;border-top:1px solid var(--line);font-size:14px;vertical-align:top}
.pill{display:inline-block;padding:3px 10px;border-radius:999px;font-size:12px;font-weight:600}
.pill-ok{background:#e2efe8;color:#1f5b42}
.pill-wait{background:var(--pill-partpaid-bg);color:var(--pill-partpaid-fg)}
.pill-bad{background:var(--pill-unpaid-bg);color:var(--pill-unpaid-fg)}
.pill-info{background:var(--pill-marks-bg);color:var(--pill-marks-fg)}
.code-box{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:22px;font-weight:600;
  letter-spacing:0.06em;background:var(--panel);border:1px dashed var(--line-strong);
  border-radius:var(--radius-sm);padding:16px 18px;display:inline-block}
.notice{border-radius:var(--radius-sm);padding:14px 16px;font-size:14px}
.notice-info{background:#eef4f4;border:1px solid #cfe0e0;color:#274a4c}
.notice-warn{background:var(--pill-partpaid-bg);border:1px solid var(--gold);color:var(--pill-partpaid-fg)}
.notice-bad{background:var(--pill-unpaid-bg);border:1px solid var(--danger);color:var(--pill-unpaid-fg)}
footer.site{border-top:1px solid var(--line);color:var(--muted);font-size:13px;padding:28px 0;background:var(--surface)}
footer.site .wrap{display:flex;justify-content:space-between;gap:16px;flex-wrap:wrap}
.qr{width:220px;height:220px;border:1px solid var(--line);border-radius:var(--radius-sm);background:var(--white);
  display:flex;align-items:center;justify-content:center;color:var(--muted);text-align:center;padding:12px}
.qr img{max-width:100%;max-height:100%}
.dl-meta{font-size:13px;color:var(--muted);margin:2px 0}
.dl-meta b{color:var(--ink);font-weight:600}
.kv{display:grid;grid-template-columns:190px 1fr;gap:8px 16px;font-size:14px}
.kv dt{color:var(--muted)}.kv dd{margin:0}
.sha{font-family:ui-monospace,Menlo,monospace;font-size:11px;word-break:break-all;color:var(--muted)}
"#;

/// A public-site page (top nav = Home / Pricing / Downloads / Support / Account).
pub fn page(cfg: &Config, title: &str, active: &str, body: &str) -> String {
    let nav = |key: &str, href: &str, label: &str| {
        let cls = if key == active { " class=\"active\"" } else { "" };
        format!("<a href=\"{href}\"{cls}>{label}</a>")
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>{title} · Vidya Budget School</title><style>{CSS}</style></head><body>\
<header class=\"site\"><div class=\"wrap\">\
<a href=\"/\"><img class=\"logo\" src=\"/assets/logo-on-light.svg\" alt=\"Vidya Budget School\"></a>\
<nav class=\"site\">{home}{pricing}{downloads}{support}{account}</nav></div></header>\
<main><div class=\"wrap\">{body}</div></main>\
{footer}</body></html>",
        title = escape(title),
        home = nav("home", "/", "Home"),
        pricing = nav("pricing", "/pricing", "Pricing"),
        downloads = nav("downloads", "/downloads", "Downloads"),
        support = nav("support", "/support", "Support"),
        account = nav("account", "/account", "Account"),
        footer = footer(cfg),
    )
}

fn footer(cfg: &Config) -> String {
    format!(
        "<footer class=\"site\"><div class=\"wrap\">\
<div>© {company} · Vidya Budget School</div>\
<div>Support: {email} · {phone} · <a href=\"{terms}\">Terms</a></div>\
</div></footer>",
        company = escape(&cfg.company_name),
        email = escape(&cfg.support_email),
        phone = escape(&cfg.support_phone),
        terms = attr(&cfg.terms_url),
    )
}
