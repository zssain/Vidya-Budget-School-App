//! Downloads page data (prompts/P10 Step 6). The service reads a `releases.json`
//! produced by the release workflow (Phase 9). Draft releases are never listed.
//! Schema is documented in `releases.example.json`; sizes are decimal bytes
//! (§2: 40 MB = 40,000,000).

use serde::Deserialize;

use crate::config::Config;
use crate::ui::escape;

#[derive(Debug, Deserialize)]
pub struct Releases {
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub releases: Vec<Release>,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub released_on: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub notes_url: Option<String>,
    #[serde(default)]
    pub platforms: Vec<Platform>,
}

#[derive(Debug, Deserialize)]
pub struct Platform {
    pub os: String, // windows | macos | android
    pub label: String,
    pub file: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub download_bytes: u64,
    #[serde(default)]
    pub installed_bytes: u64,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub requirements: String,
    /// `null`/absent = unsigned; else e.g. "Acme Pvt Ltd".
    #[serde(default)]
    pub signed_by: Option<String>,
}

/// Load + parse `releases.json`. Returns `None` if unset, missing or invalid.
pub fn load(cfg: &Config) -> Option<Releases> {
    let path = cfg.releases_json.as_ref()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn fmt_mb(bytes: u64) -> String {
    if bytes == 0 {
        return "—".into();
    }
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// Render the downloads page body (cards for Windows / Android / macOS).
pub fn render(cfg: &Config, releases: Option<&Releases>) -> String {
    let latest = releases
        .and_then(|r| r.releases.iter().find(|rel| !rel.draft));

    let mut body = String::new();
    body.push_str("<div class=\"eyebrow\">Downloads</div><h1>Get Vidya</h1>");
    body.push_str(
        "<p class=\"sub\">Each download is under 40 MB. The school's data is stored separately \
on the school's own PC — it is never inside the download.</p>",
    );

    let rel = match latest {
        Some(r) => r,
        None => {
            body.push_str(
                "<div class=\"card mt2\"><p class=\"muted\">No public releases yet. \
Once a build is published it will appear here with its version, size and SHA-256 checksum.</p></div>",
            );
            return body;
        }
    };

    body.push_str(&format!(
        "<p class=\"muted mt small\">Latest version <b>{}</b>{}{}</p>",
        escape(&rel.version),
        if rel.released_on.is_empty() { String::new() } else { format!(" · released {}", escape(&rel.released_on)) },
        match &rel.notes_url {
            Some(u) if !u.is_empty() => format!(" · <a href=\"{}\">Release notes</a>", escape(u)),
            _ => String::new(),
        },
    ));

    body.push_str("<div class=\"grid cols-3 mt2\">");
    for os in ["windows", "android", "macos"] {
        for p in rel.platforms.iter().filter(|p| p.os == os) {
            body.push_str(&card(cfg, p));
        }
    }
    body.push_str("</div>");

    // Honest, per §2 disclosure + per-platform install notes.
    body.push_str(
        "<div class=\"notice notice-info mt2\">Not counted in the download size but installed by the \
operating system when needed: the web engine (on Windows 10, Microsoft WebView2 may be downloaded \
once during install), Android System WebView, plus your school's data, backups and logs.</div>",
    );
    body.push_str(
        "<div class=\"card mt\"><h3>Installing on Android</h3><p class=\"small muted\">Android may warn \
that the app is from an unknown source. Open the downloaded APK, then when prompted tap \
<b>Settings → Allow from this source</b> (or <b>Install unknown apps</b>), turn it on for your \
browser/Files app, go back and tap <b>Install</b>. Choose the file that matches your phone \
(most phones use <b>arm64-v8a</b>).</p></div>",
    );
    body
}

fn card(cfg: &Config, p: &Platform) -> String {
    let signing = match &p.signed_by {
        Some(name) if !name.is_empty() => format!("<span class=\"pill pill-ok\">Signed by {}</span>", escape(name)),
        _ => "<span class=\"pill pill-wait\">Unsigned — see install steps</span>".to_string(),
    };
    let dl = match &p.url {
        Some(u) if !u.is_empty() => format!("<a class=\"btn btn-primary mt\" href=\"{}\">Download</a>", escape(u)),
        _ => "<span class=\"muted small\">Link pending</span>".to_string(),
    };
    let win_note = if p.os == "windows" {
        "<p class=\"dl-meta\">Windows 10 may download Microsoft WebView2 once during install.</p>"
    } else {
        ""
    };
    let _ = cfg;
    format!(
        "<div class=\"card\"><h3>{label}</h3>\
<p class=\"dl-meta\">File: <b>{file}</b></p>\
<p class=\"dl-meta\">Download: <b>{dlsz}</b> · Installed: <b>{insz}</b></p>\
<p class=\"dl-meta\">Requirements: {req}</p>\
<p class=\"dl-meta\">{signing}</p>{win_note}\
<p class=\"dl-meta\">SHA-256:</p><p class=\"sha\">{sha}</p>{dl}</div>",
        label = escape(&p.label),
        file = escape(&p.file),
        dlsz = fmt_mb(p.download_bytes),
        insz = fmt_mb(p.installed_bytes),
        req = escape(&p.requirements),
        sha = if p.sha256.is_empty() { "—".to_string() } else { escape(&p.sha256) },
    )
}
