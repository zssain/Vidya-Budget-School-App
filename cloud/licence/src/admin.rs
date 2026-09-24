//! Company admin panel (prompts/P10 Step 5). Company staff only.
//!
//! Security: Argon2id passwords + lockout · session cookie `HttpOnly; Secure;
//! SameSite=Strict` (Secure toggled off only in `--dev` over http) · a per-session
//! CSRF token required on every mutating form · optional IP allowlist. Every
//! action requires a reason and is appended to the hash-chained admin audit.

use std::net::SocketAddr;
use std::time::Duration;

use argon2::password_hash::rand_core::OsRng as ArgonOsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::api::client_ip;
use crate::audit;
use crate::crypto::{self, ct_eq, sha256_hex};
use crate::db::{new_id, now_iso};
use crate::domain;
use crate::error::AppError;
use crate::state::AppState;
use crate::ui::{escape, CSS};

const COOKIE: &str = "vidya_admin";
const LOCK_THRESHOLD: i64 = 5;
const LOCK_MINUTES: i64 = 15;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin", get(|| async { Redirect::to("/admin/licences") }))
        .route("/admin/login", get(login_form).post(login_submit))
        .route("/admin/logout", post(logout))
        .route("/admin/licences", get(licences))
        .route("/admin/licences/:id", get(licence_detail))
        .route("/admin/licences/:id/reissue", post(reissue))
        .route("/admin/licences/:id/revoke", post(revoke))
        .route("/admin/licences/:id/transfer", post(approve_transfer))
        .route("/admin/licences/:id/rotate-relay", post(rotate_relay))
        .route("/admin/licences/:id/note", post(add_note))
        .route("/admin/orders", get(orders))
        .route("/admin/orders/:id/verify", post(verify_order))
        .route("/admin/orders/:id/reject", post(reject_order))
        .route("/admin/audit", get(audit_view))
}

// --- password hashing ------------------------------------------------------

fn argon() -> Argon2<'static> {
    // §9 params: m = 19 MiB, t = 2, p = 1.
    let params = Params::new(19_456, 2, 1, None).expect("argon2 params");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut ArgonOsRng);
    argon()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password")
        .to_string()
}

fn verify_password(password: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => argon().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Create the bootstrap admin if none with that email exists (called at startup).
pub fn ensure_bootstrap_admin(conn: &Connection, email: &str, password: &str) -> rusqlite::Result<()> {
    let exists: Option<bool> = conn
        .query_row("SELECT 1 FROM admin_user WHERE email = ?1", [email], |_| Ok(true))
        .optional()?;
    if exists.is_none() {
        conn.execute(
            "INSERT INTO admin_user (id, email, password_hash, role, disabled, created_at)
             VALUES (?1, ?2, ?3, 'owner', 0, ?4)",
            params![new_id(), email, hash_password(password), now_iso()],
        )?;
    }
    Ok(())
}

// --- auth / session --------------------------------------------------------

struct Session {
    admin_email: String,
    csrf: String,
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn ip_allowed(state: &AppState, ip: &str) -> bool {
    let list = &state.cfg.admin_ip_allowlist;
    list.is_empty() || list.iter().any(|a| a == ip)
}

/// Resolve the current admin session, or return a redirect/forbidden response.
/// (The `Err` Response is intentionally the axum response type — this is an auth
/// guard, not a hot value path.)
#[allow(clippy::result_large_err)]
fn require_admin(state: &AppState, headers: &HeaderMap, peer: SocketAddr) -> Result<Session, Response> {
    let ip = client_ip(headers, peer);
    if !ip_allowed(state, &ip) {
        return Err((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }
    let token = match cookie_value(headers, COOKIE) {
        Some(t) => t,
        None => return Err(Redirect::to("/admin/login").into_response()),
    };
    let id = sha256_hex(token.as_bytes());
    let now = now_iso();
    let conn = state.db.lock().unwrap();
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT u.email, s.csrf_token
               FROM admin_session s JOIN admin_user u ON u.id = s.admin_id
              WHERE s.id = ?1 AND s.expires_at > ?2 AND u.disabled = 0",
            params![id, now],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .unwrap_or(None);
    match row {
        Some((admin_email, csrf)) => Ok(Session { admin_email, csrf }),
        None => Err(Redirect::to("/admin/login").into_response()),
    }
}

fn check_csrf(session: &Session, token: &str) -> bool {
    ct_eq(session.csrf.as_bytes(), token.as_bytes())
}

fn random_token() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(b)
}

// --- chrome ----------------------------------------------------------------

fn admin_page(title: &str, active: &str, session: Option<&Session>, body: &str) -> String {
    let nav = |key: &str, href: &str, label: &str| {
        let cls = if key == active { " style=\"color:#fff\"" } else { "" };
        format!("<a href=\"{href}\"{cls}>{label}</a>")
    };
    let (who, links) = match session {
        Some(s) => (
            format!("<span class=\"muted\" style=\"color:var(--on-navy-muted)\">{}</span>", escape(&s.admin_email)),
            format!(
                "{}{}{}<form method=\"post\" action=\"/admin/logout\" style=\"display:inline\">\
<input type=\"hidden\" name=\"csrf\" value=\"{}\">\
<button class=\"btn btn-ghost\" style=\"padding:6px 12px;font-size:13px\">Sign out</button></form>",
                nav("licences", "/admin/licences", "Licences"),
                nav("orders", "/admin/orders", "Orders"),
                nav("audit", "/admin/audit", "Audit"),
                escape(&s.csrf),
            ),
        ),
        None => (String::new(), String::new()),
    };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>{title} · Vidya admin</title><style>{CSS}\
.adminbar{{background:var(--navy);color:var(--on-navy)}}\
.adminbar .wrap{{display:flex;align-items:center;justify-content:space-between;height:60px}}\
.adminbar a{{color:var(--on-navy);font-size:14px;margin-left:18px;font-weight:500}}\
.adminbar a:hover{{color:#fff;text-decoration:none}}\
.brand{{font-family:var(--serif);font-size:20px;color:#fff}}\
</style></head><body style=\"background:var(--bg)\">\
<div class=\"adminbar\"><div class=\"wrap\"><div class=\"brand\">Vidya · Admin</div>\
<div class=\"row\">{links} {who}</div></div></div>\
<main><div class=\"wrap\">{body}</div></main></body></html>",
        title = escape(title),
    )
}

// --- login -----------------------------------------------------------------

async fn login_form(State(_state): State<AppState>) -> Html<String> {
    let body = "<div style=\"max-width:420px;margin:40px auto\"><div class=\"card\">\
<div class=\"eyebrow\">Company admin</div><h2>Sign in</h2>\
<form method=\"post\" action=\"/admin/login\">\
<label for=\"email\">Email</label><input id=\"email\" name=\"email\" type=\"email\" required>\
<label for=\"password\">Password</label><input id=\"password\" name=\"password\" type=\"password\" required>\
<div class=\"mt\"><button class=\"btn btn-primary\" type=\"submit\">Sign in</button></div></form></div></div>";
    Html(admin_page("Sign in", "", None, body))
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

async fn login_submit(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(f): Form<LoginForm>,
) -> Result<Response, AppError> {
    let ip = client_ip(&headers, peer);
    if !ip_allowed(&state, &ip) {
        return Ok((StatusCode::FORBIDDEN, "Forbidden").into_response());
    }
    // Per-IP throttle on the login endpoint.
    if !state.rl.lock().unwrap().allow(&format!("login:{ip}"), 20, Duration::from_secs(600)) {
        let body = "<div style=\"max-width:420px;margin:40px auto\"><div class=\"notice notice-warn\">\
Too many attempts. Please wait and try again.</div></div>";
        return Ok(Html(admin_page("Sign in", "", None, body)).into_response());
    }

    let email = f.email.trim().to_ascii_lowercase();
    let now = now_iso();
    let (ok, token, csrf) = {
        let conn = state.db.lock().unwrap();
        let row: Option<(String, String, i64, Option<String>, i64)> = conn
            .query_row(
                "SELECT id, password_hash, failed_count, locked_until, disabled FROM admin_user WHERE email = ?1",
                [&email],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;

        match row {
            Some((id, phc, failed, locked_until, disabled)) => {
                let is_locked = disabled == 1
                    || locked_until.as_deref().map(|lu| lu > now.as_str()).unwrap_or(false);
                if is_locked {
                    (false, String::new(), String::new())
                } else if verify_password(&f.password, &phc) {
                    // Success: reset counter, mint a session.
                    conn.execute(
                        "UPDATE admin_user SET failed_count = 0, locked_until = NULL WHERE id = ?1",
                        [&id],
                    )?;
                    let token = random_token();
                    let csrf = random_token();
                    let expires = expires_at(state.cfg.session_ttl_hours);
                    conn.execute(
                        "INSERT INTO admin_session (id, admin_id, csrf_token, created_at, expires_at) VALUES (?1,?2,?3,?4,?5)",
                        params![sha256_hex(token.as_bytes()), id, csrf, now, expires],
                    )?;
                    audit::append(&conn, &email, "admin_login", None, None, None, serde_json::json!({ "ip": ip }))?;
                    (true, token, csrf)
                } else {
                    // Failure: bump counter, lock at the threshold.
                    let nf = failed + 1;
                    let lock = if nf >= LOCK_THRESHOLD { Some(lock_until(LOCK_MINUTES)) } else { None };
                    conn.execute(
                        "UPDATE admin_user SET failed_count = ?2, locked_until = ?3 WHERE id = ?1",
                        params![id, nf, lock],
                    )?;
                    (false, String::new(), String::new())
                }
            }
            None => {
                // Unknown email: same generic answer (do a dummy verify to blunt timing).
                let _ = verify_password(&f.password, "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$0000000000000000000000000000000000000000000");
                (false, String::new(), String::new())
            }
        }
    };

    if !ok {
        let body = "<div style=\"max-width:420px;margin:40px auto\"><div class=\"notice notice-bad\">\
Sign-in failed. Check your email and password. Repeated failures lock the account temporarily.</div>\
<p class=\"mt\"><a class=\"btn btn-line\" href=\"/admin/login\">Try again</a></p></div>";
        return Ok(Html(admin_page("Sign in", "", None, body)).into_response());
    }

    let _ = csrf;
    let cookie = format!(
        "{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/admin; Max-Age={}{}",
        state.cfg.session_ttl_hours * 3600,
        if state.cfg.cookie_secure { "; Secure" } else { "" },
    );
    Ok(([(header::SET_COOKIE, cookie)], Redirect::to("/admin/licences")).into_response())
}

async fn logout(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(f): Form<CsrfForm>,
) -> Result<Response, AppError> {
    if let Ok(session) = require_admin(&state, &headers, peer) {
        if check_csrf(&session, &f.csrf) {
            if let Some(token) = cookie_value(&headers, COOKIE) {
                let conn = state.db.lock().unwrap();
                conn.execute("DELETE FROM admin_session WHERE id = ?1", [sha256_hex(token.as_bytes())])?;
            }
        }
    }
    let clear = format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/admin; Max-Age=0");
    Ok(([(header::SET_COOKIE, clear)], Redirect::to("/admin/login")).into_response())
}

// --- licences list ---------------------------------------------------------

#[derive(Deserialize)]
struct SearchQ {
    #[serde(default)]
    q: String,
}

async fn licences(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(sq): Query<SearchQ>,
) -> Result<Response, AppError> {
    let session = match require_admin(&state, &headers, peer) {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let q = sq.q.trim();
    let like = format!("%{q}%");
    let rows = {
        let conn = state.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT l.licence_id, l.status, l.server_epoch, c.school_name, c.email, o.provider_order_id, l.app_version_last_seen
               FROM licence l
               JOIN purchase_order o ON o.id = l.order_id
               JOIN customer c ON c.id = o.customer_id
              WHERE (?1 = '' OR c.school_name LIKE ?2 OR c.email LIKE ?2 OR o.provider_order_id LIKE ?2 OR l.licence_id LIKE ?2)
              ORDER BY l.created_at DESC LIMIT 200",
        )?;
        let mapped = stmt.query_map(params![q, like], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };

    let mut trs = String::new();
    for (lid, status, epoch, school, email, order, ver) in &rows {
        trs.push_str(&format!(
            "<tr><td><a href=\"/admin/licences/{lid}\">{school}</a><div class=\"small muted\">{email}</div></td>\
<td>{order}</td><td>{pill}</td><td>{epoch}</td><td>{ver}</td></tr>",
            lid = escape(lid),
            school = escape(school),
            email = escape(email),
            order = escape(order),
            pill = status_pill(status),
            epoch = epoch,
            ver = escape(ver.as_deref().unwrap_or("—")),
        ));
    }
    if rows.is_empty() {
        trs.push_str("<tr><td colspan=\"5\" class=\"muted\">No licences match.</td></tr>");
    }

    let body = format!(
        "<h2 class=\"mt\">Licences</h2>\
<form method=\"get\" action=\"/admin/licences\" class=\"row mb\">\
<input type=\"text\" name=\"q\" value=\"{q}\" placeholder=\"Search school, email, order or licence id\" style=\"max-width:420px\">\
<button class=\"btn btn-line\">Search</button></form>\
<div class=\"card\" style=\"padding:0\"><table><thead><tr>\
<th>School</th><th>Order</th><th>Status</th><th>Epoch</th><th>App version</th></tr></thead>\
<tbody>{trs}</tbody></table></div>",
        q = escape(q),
    );
    Ok(Html(admin_page("Licences", "licences", Some(&session), &body)).into_response())
}

fn status_pill(status: &str) -> String {
    let (cls, label) = match status {
        "active" => ("pill-ok", "active"),
        "revoked" => ("pill-bad", "revoked"),
        "moved" => ("pill-info", "moved"),
        "paid" => ("pill-ok", "paid"),
        "awaiting_verification" => ("pill-wait", "awaiting verification"),
        "failed" => ("pill-bad", "failed"),
        "refunded" => ("pill-info", "refunded"),
        "created" => ("pill-wait", "awaiting payment"),
        other => ("pill-info", other),
    };
    format!("<span class=\"pill {cls}\">{}</span>", escape(label))
}

// --- licence detail --------------------------------------------------------

#[allow(clippy::type_complexity)]
async fn licence_detail(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let session = match require_admin(&state, &headers, peer) {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let conn = state.db.lock().unwrap();

    let lic: Option<(String, String, String, Option<i64>, Option<i64>, i64, Option<String>, Option<String>, Option<String>, Option<String>, String)> = conn
        .query_row(
            "SELECT l.licence_id, l.school_id, l.status, l.max_students, l.max_devices, l.server_epoch,
                    l.server_machine_id, l.app_version_last_seen, l.last_check_at, l.revoked_reason, l.plan
               FROM licence l WHERE l.licence_id = ?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
        )
        .optional()?;
    let Some((lid, school_id, status, maxs, maxd, epoch, machine, ver, last_check, revoked_reason, plan)) = lic else {
        drop(conn);
        return Ok((StatusCode::NOT_FOUND, "not found").into_response());
    };

    let (school, email, phone, order): (String, String, String, String) = conn.query_row(
        "SELECT c.school_name, c.email, c.phone, o.provider_order_id
           FROM licence l JOIN purchase_order o ON o.id = l.order_id JOIN customer c ON c.id = o.customer_id
          WHERE l.licence_id = ?1",
        [&id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    // Verified payment reference (the admin_verify event).
    let pay_ref: Option<(i64, Option<String>, String)> = conn
        .query_row(
            "SELECT amount_paise, upi_ref, actor FROM payment_event
              WHERE order_id = (SELECT id FROM purchase_order WHERE provider_order_id = ?1)
                AND kind = 'admin_verify' AND verified = 1 ORDER BY received_at DESC LIMIT 1",
            [&order],
            |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0), r.get(1)?, r.get(2)?)),
        )
        .optional()?;

    // Activation history + transfers.
    let acts = collect_strings(&conn,
        "SELECT redeemed_at, redeemed_machine_id, revoked_at FROM activation_code WHERE licence_id = ?1 ORDER BY created_at DESC",
        &id, 3)?;
    let transfers = collect_strings(&conn,
        "SELECT at, old_machine_id, new_machine_id, method, reason FROM transfer WHERE licence_id = ?1 ORDER BY at DESC",
        &id, 5)?;
    drop(conn);

    let csrf = escape(&session.csrf);
    let masked_machine = machine.as_deref().map(mask).unwrap_or_else(|| "— not activated —".to_string());
    let pay_line = match pay_ref {
        Some((amt, upi, actor)) => format!(
            "₹{} · UPI ref {} · verified by {}",
            amt / 100,
            escape(upi.as_deref().unwrap_or("—")),
            escape(&actor)
        ),
        None => "—".to_string(),
    };

    let mut act_html = String::new();
    for a in &acts {
        act_html.push_str(&format!("<li class=\"small\">{}</li>", escape(a)));
    }
    if acts.is_empty() { act_html.push_str("<li class=\"small muted\">none</li>"); }
    let mut tr_html = String::new();
    for t in &transfers {
        tr_html.push_str(&format!("<li class=\"small\">{}</li>", escape(t)));
    }
    if transfers.is_empty() { tr_html.push_str("<li class=\"small muted\">none</li>"); }

    let actions = if status == "revoked" {
        format!(
            "<div class=\"notice notice-bad\">This licence is revoked{}.</div>",
            revoked_reason.map(|r| format!(" — {}", escape(&r))).unwrap_or_default()
        )
    } else {
        format!(
            "<div class=\"grid cols-2 mt2\">\
{reissue}{revoke}{transfer}{rotate}</div>{note}",
            reissue = action_card(
                "Reissue activation code",
                &format!("/admin/licences/{lid}/reissue"),
                "The current code is revoked and a new one is shown once.",
                &csrf, &[],
            ),
            revoke = action_card(
                "Revoke licence",
                &format!("/admin/licences/{lid}/revoke"),
                "The app will report revoked at its next check. This cannot be undone here.",
                &csrf, &[("confirm", "Type REVOKE to confirm", "")],
            ),
            transfer = action_card(
                "Approve transfer to a new PC",
                &format!("/admin/licences/{lid}/transfer"),
                "Rebind the licence to a new machine id (from the school). Epoch increases; the old PC becomes 'moved'.",
                &csrf, &[("new_machine_id", "New machine id", "")],
            ),
            rotate = action_card(
                "Rotate relay secret",
                &format!("/admin/licences/{lid}/rotate-relay"),
                "Records a rotation. Note: with the stateless relay the effective secret is unchanged (see handoff).",
                &csrf, &[],
            ),
            note = action_card(
                "Add a note",
                &format!("/admin/licences/{lid}/note"),
                "Recorded in the audit log.",
                &csrf, &[("note", "Note", "")],
            ),
        )
    };

    let body = format!(
        "<p class=\"mt small\"><a href=\"/admin/licences\">← Licences</a></p>\
<h2>{school} {pill}</h2>\
<div class=\"card\"><dl class=\"kv\">\
<dt>Licence id</dt><dd class=\"sha\">{lid}</dd>\
<dt>School id</dt><dd class=\"sha\">{school_id}</dd>\
<dt>Customer</dt><dd>{email} · {phone}</dd>\
<dt>Order</dt><dd>{order}</dd>\
<dt>Plan / limits</dt><dd>{plan} · students {maxs} · devices {maxd}</dd>\
<dt>Verified payment</dt><dd>{pay}</dd>\
<dt>Server machine</dt><dd>{machine}</dd>\
<dt>Epoch</dt><dd>{epoch}</dd>\
<dt>App version last seen</dt><dd>{ver}</dd>\
<dt>Last check</dt><dd>{last_check}</dd>\
</dl></div>\
<div class=\"grid cols-2 mt\"><div class=\"card\"><h3>Activation history</h3><ul>{acts}</ul></div>\
<div class=\"card\"><h3>Transfers</h3><ul>{trs}</ul></div></div>\
{actions}",
        school = escape(&school),
        pill = status_pill(&status),
        lid = escape(&lid),
        school_id = escape(&school_id),
        email = escape(&email),
        phone = escape(&mask_phone(&phone)),
        order = escape(&order),
        plan = escape(&plan),
        maxs = maxs.map(|v| v.to_string()).unwrap_or_else(|| "unlimited".into()),
        maxd = maxd.map(|v| v.to_string()).unwrap_or_else(|| "unlimited".into()),
        pay = pay_line,
        machine = escape(&masked_machine),
        epoch = epoch,
        ver = escape(ver.as_deref().unwrap_or("—")),
        last_check = escape(last_check.as_deref().unwrap_or("—")),
        acts = act_html,
        trs = tr_html,
        actions = actions,
    );
    Ok(Html(admin_page("Licence", "licences", Some(&session), &body)).into_response())
}

fn action_card(title: &str, action: &str, help: &str, csrf: &str, fields: &[(&str, &str, &str)]) -> String {
    let mut inputs = String::new();
    for (name, label, placeholder) in fields {
        inputs.push_str(&format!(
            "<label>{}</label><input type=\"text\" name=\"{}\" placeholder=\"{}\">",
            escape(label), escape(name), escape(placeholder)
        ));
    }
    format!(
        "<div class=\"card\"><h3>{title}</h3><p class=\"small muted\">{help}</p>\
<form method=\"post\" action=\"{action}\">{inputs}\
<label>Reason (required)</label><input type=\"text\" name=\"reason\" required>\
<input type=\"hidden\" name=\"csrf\" value=\"{csrf}\">\
<div class=\"mt\"><button class=\"btn btn-primary\">Confirm</button></div></form></div>",
        title = escape(title),
        help = escape(help),
        action = escape(action),
    )
}

fn collect_strings(conn: &Connection, sql: &str, id: &str, cols: usize) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([id], |r| {
        let mut parts = Vec::new();
        for i in 0..cols {
            parts.push(r.get::<_, Option<String>>(i)?.unwrap_or_else(|| "—".into()));
        }
        Ok(parts.join(" · "))
    })?;
    rows.collect()
}

fn mask(s: &str) -> String {
    if s.len() <= 8 {
        s.to_string()
    } else {
        format!("{}…{}", &s[..6], &s[s.len() - 4..])
    }
}
fn mask_phone(s: &str) -> String {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() >= 4 {
        format!("••••••{}", &d[d.len() - 4..])
    } else {
        "—".into()
    }
}

// --- mutating actions ------------------------------------------------------

#[derive(Deserialize)]
struct CsrfForm {
    #[serde(default)]
    csrf: String,
}

#[derive(Deserialize)]
struct ActionForm {
    #[serde(default)]
    csrf: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    new_machine_id: String,
    #[serde(default)]
    confirm: String,
    #[serde(default)]
    note: String,
}

/// Guard shared by every mutating admin action: auth + CSRF + non-empty reason.
#[allow(clippy::result_large_err)]
fn guard<'a>(
    state: &AppState,
    headers: &HeaderMap,
    peer: SocketAddr,
    csrf: &str,
    reason: &'a str,
) -> Result<(Session, &'a str), Response> {
    let session = require_admin(state, headers, peer)?;
    if !check_csrf(&session, csrf) {
        return Err((StatusCode::FORBIDDEN, "Invalid CSRF token").into_response());
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "A reason is required").into_response());
    }
    Ok((session, reason))
}

fn back(id: &str) -> Response {
    Redirect::to(&format!("/admin/licences/{id}")).into_response()
}

async fn reissue(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Form(f): Form<ActionForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let code = {
        let conn = state.db.lock().unwrap();
        let order_id: Option<String> = conn
            .query_row("SELECT order_id FROM licence WHERE licence_id = ?1", [&id], |r| r.get(0))
            .optional()?;
        let Some(order_id) = order_id else { return Ok((StatusCode::NOT_FOUND, "not found").into_response()) };
        let now = now_iso();
        conn.execute(
            "UPDATE activation_code SET revoked_at = ?2 WHERE licence_id = ?1 AND revoked_at IS NULL",
            params![id, now],
        )?;
        let code = crypto::generate_code();
        conn.execute(
            "INSERT INTO activation_code (id, code_hash, code_enc, licence_id, order_id, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![new_id(), crypto::hash_code(&code), crypto::encrypt(&state.cfg.code_enc_key, code.as_bytes()), id, order_id, now],
        )?;
        audit::append(&conn, &session.admin_email, "reissue_activation_code", Some("licence"), Some(&id), Some(reason), serde_json::json!({}))?;
        code
    };
    let body = format!(
        "<div class=\"notice notice-info\">A new activation code was issued (the old one is revoked). \
Share it with the school; it is also re-viewable on the Account page.</div>\
<div class=\"code-box mt\">{}</div><p class=\"mt\"><a class=\"btn btn-line\" href=\"/admin/licences/{}\">Back to licence</a></p>",
        escape(&code), escape(&id),
    );
    Ok(Html(admin_page("Reissued", "licences", Some(&session), &body)).into_response())
}

async fn revoke(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Form(f): Form<ActionForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    if f.confirm.trim() != "REVOKE" {
        return Ok((StatusCode::BAD_REQUEST, "Type REVOKE to confirm").into_response());
    }
    {
        let conn = state.db.lock().unwrap();
        let n = conn.execute(
            "UPDATE licence SET status = 'revoked', revoked_reason = ?2, revoked_at = ?3 WHERE licence_id = ?1 AND status != 'revoked'",
            params![id, reason, now_iso()],
        )?;
        if n == 0 {
            return Ok((StatusCode::NOT_FOUND, "not found or already revoked").into_response());
        }
        audit::append(&conn, &session.admin_email, "revoke_licence", Some("licence"), Some(&id), Some(reason), serde_json::json!({}))?;
    }
    Ok(back(&id))
}

async fn approve_transfer(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Form(f): Form<ActionForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let new_machine = f.new_machine_id.trim();
    if new_machine.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, "New machine id is required").into_response());
    }
    {
        let mut conn = state.db.lock().unwrap();
        match domain::transfer_admin(&mut conn, &id, new_machine, &session.admin_email, reason)? {
            Ok(()) => {}
            Err(_) => return Ok((StatusCode::NOT_FOUND, "not found or revoked").into_response()),
        }
    }
    Ok(back(&id))
}

async fn rotate_relay(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Form(f): Form<ActionForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    {
        let conn = state.db.lock().unwrap();
        let now = now_iso();
        let gen: i64 = conn
            .query_row("SELECT COALESCE(MAX(generation),0) FROM relay_secret WHERE licence_id = ?1", [&id], |r| r.get(0))
            .optional()?
            .unwrap_or(0);
        let school_id: Option<String> = conn.query_row("SELECT school_id FROM licence WHERE licence_id = ?1", [&id], |r| r.get(0)).optional()?;
        let Some(school_id) = school_id else { return Ok((StatusCode::NOT_FOUND, "not found").into_response()) };
        conn.execute("UPDATE relay_secret SET active = 0, rotated_at = ?2 WHERE licence_id = ?1", params![id, now])?;
        conn.execute(
            "INSERT INTO relay_secret (id, licence_id, secret_hash, generation, active, created_at) VALUES (?1,?2,?3,?4,1,?5)",
            params![new_id(), id, sha256_hex(crypto::relay_secret(&state.cfg.relay_shared_key, &school_id).as_bytes()), gen + 1, now],
        )?;
        audit::append(&conn, &session.admin_email, "rotate_relay_secret", Some("licence"), Some(&id), Some(reason), serde_json::json!({ "generation": gen + 1 }))?;
    }
    Ok(back(&id))
}

async fn add_note(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Form(f): Form<ActionForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    {
        let conn = state.db.lock().unwrap();
        audit::append(&conn, &session.admin_email, "add_note", Some("licence"), Some(&id), Some(reason), serde_json::json!({ "note": f.note.trim() }))?;
    }
    Ok(back(&id))
}

// --- orders ----------------------------------------------------------------

async fn orders(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let session = match require_admin(&state, &headers, peer) {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let (pending, recent) = {
        let conn = state.db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT o.provider_order_id, o.status, o.upi_ref, c.school_name, c.email, o.id
               FROM purchase_order o JOIN customer c ON c.id = o.customer_id
              ORDER BY CASE o.status WHEN 'awaiting_verification' THEN 0 WHEN 'created' THEN 1 ELSE 2 END, o.created_at DESC
              LIMIT 200",
        )?;
        let rows: Vec<(String, String, Option<String>, String, String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
            .collect::<Result<_, _>>()?;
        let pending: Vec<_> = rows.iter().filter(|r| r.1 == "awaiting_verification").cloned().collect();
        (pending, rows)
    };

    let csrf = escape(&session.csrf);
    let mut pend_html = String::new();
    for (order, _status, upi, school, email, oid) in &pending {
        pend_html.push_str(&format!(
            "<div class=\"card mb\"><div class=\"row\" style=\"justify-content:space-between\">\
<div><b>{school}</b> · {email}<div class=\"small muted\">{order} · UPI ref {upi}</div></div></div>\
<form method=\"post\" action=\"/admin/orders/{oid}/verify\" class=\"row mt\">\
<input type=\"hidden\" name=\"csrf\" value=\"{csrf}\">\
<input type=\"text\" name=\"amount_paise\" placeholder=\"Amount in paise (e.g. 500000)\" style=\"max-width:220px\">\
<input type=\"text\" name=\"upi_ref\" placeholder=\"UPI ref you see in your account\" style=\"max-width:220px\">\
<input type=\"text\" name=\"reason\" placeholder=\"Reason (required)\" required style=\"max-width:220px\">\
<button class=\"btn btn-primary\">Verify & issue</button></form>\
<form method=\"post\" action=\"/admin/orders/{oid}/reject\" class=\"row mt\">\
<input type=\"hidden\" name=\"csrf\" value=\"{csrf}\">\
<input type=\"text\" name=\"reason\" placeholder=\"Reason to reject (required)\" required style=\"max-width:320px\">\
<button class=\"btn btn-line\">Reject</button></form></div>",
            school = escape(school), email = escape(email), order = escape(order),
            upi = escape(upi.as_deref().unwrap_or("—")), oid = escape(oid),
        ));
    }
    if pending.is_empty() {
        pend_html.push_str("<p class=\"muted\">No payments awaiting verification.</p>");
    }

    let mut rows_html = String::new();
    for (order, status, _upi, school, email, oid) in &recent {
        rows_html.push_str(&format!(
            "<tr><td><a href=\"/admin/orders/{oid}#e\">{order}</a></td><td>{school}<div class=\"small muted\">{email}</div></td><td>{pill}</td></tr>",
            oid = escape(oid), order = escape(order), school = escape(school), email = escape(email), pill = status_pill(status),
        ));
    }

    let body = format!(
        "<h2 class=\"mt\">Payments awaiting verification</h2><div class=\"notice notice-warn mb\">\
Only verify after you have confirmed the money actually arrived in your UPI account. Verifying \
issues the licence + activation code — it cannot be taken back except by revoking.</div>{pend}\
<h2 class=\"mt2\">All orders</h2><div class=\"card\" style=\"padding:0\"><table><thead><tr>\
<th>Order</th><th>School</th><th>Status</th></tr></thead><tbody>{rows}</tbody></table></div>",
        pend = pend_html, rows = rows_html,
    );
    Ok(Html(admin_page("Orders", "orders", Some(&session), &body)).into_response())
}

#[derive(Deserialize)]
struct VerifyForm {
    #[serde(default)]
    csrf: String,
    #[serde(default)]
    amount_paise: String,
    #[serde(default)]
    upi_ref: String,
    #[serde(default)]
    reason: String,
}

async fn verify_order(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(oid): Path<String>,
    Form(f): Form<VerifyForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let amount: i64 = f.amount_paise.trim().parse().unwrap_or(0);
    let upi = f.upi_ref.trim();
    let result = {
        let mut conn = state.db.lock().unwrap();
        domain::verify_payment_and_issue(
            &mut conn,
            &state.cfg,
            &session.admin_email,
            &oid,
            amount,
            if upi.is_empty() { None } else { Some(upi) },
            reason,
        )?
    };
    let body = match result {
        Ok(issue) => format!(
            "<div class=\"notice notice-info\">{head} School <b>{}</b>. The activation code is shown \
once below and is re-viewable by the school on the Account page.</div>\
<div class=\"code-box mt\">{}</div>\
<p class=\"mt\"><a class=\"btn btn-line\" href=\"/admin/orders\">Back to orders</a> \
<a class=\"btn btn-line\" href=\"/admin/licences/{}\">View licence</a></p>",
            escape(&issue.school_id),
            escape(&issue.code),
            escape(&issue.licence_id),
            head = if issue.already_issued { "This order was already issued — showing the existing code (no second licence)." } else { "Payment verified and licence issued." },
        ),
        Err(_) => "<div class=\"notice notice-bad\">Order not found.</div>".to_string(),
    };
    Ok(Html(admin_page("Verify", "orders", Some(&session), &body)).into_response())
}

#[derive(Deserialize)]
struct RejectForm {
    #[serde(default)]
    csrf: String,
    #[serde(default)]
    reason: String,
}

async fn reject_order(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(oid): Path<String>,
    Form(f): Form<RejectForm>,
) -> Result<Response, AppError> {
    let (session, reason) = match guard(&state, &headers, peer, &f.csrf, &f.reason) {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    {
        let conn = state.db.lock().unwrap();
        // Only reject an order that has NOT been issued (no licence).
        let has_licence: Option<bool> = conn
            .query_row("SELECT 1 FROM licence WHERE order_id = ?1", [&oid], |_| Ok(true))
            .optional()?;
        if has_licence.is_some() {
            return Ok((StatusCode::BAD_REQUEST, "Order already issued a licence; revoke the licence instead").into_response());
        }
        let now = now_iso();
        conn.execute("UPDATE purchase_order SET status = 'failed', updated_at = ?2 WHERE id = ?1", params![oid, now])?;
        conn.execute(
            "INSERT OR IGNORE INTO payment_event (id, provider_event_id, order_id, kind, actor, verified, raw_json, received_at)
             VALUES (?1, ?2, ?3, 'admin_reject', ?4, 0, ?5, ?6)",
            params![new_id(), format!("reject:{oid}:{now}"), oid, session.admin_email, serde_json::json!({ "reason": reason }).to_string(), now],
        )?;
        audit::append(&conn, &session.admin_email, "reject_order", Some("order"), Some(&oid), Some(reason), serde_json::json!({}))?;
    }
    Ok(Redirect::to("/admin/orders").into_response())
}

// --- audit view ------------------------------------------------------------

#[allow(clippy::type_complexity)]
async fn audit_view(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let session = match require_admin(&state, &headers, peer) {
        Ok(s) => s,
        Err(r) => return Ok(r),
    };
    let (status, rows) = {
        let conn = state.db.lock().unwrap();
        let status = audit::verify_chain(&conn)?;
        let mut stmt = conn.prepare(
            "SELECT seq, at, admin_email, action, target_type, target_id, reason FROM admin_audit ORDER BY seq DESC LIMIT 300",
        )?;
        let rows: Vec<(i64, String, String, String, Option<String>, Option<String>, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)))?
            .collect::<Result<_, _>>()?;
        (status, rows)
    };

    let chain = if status.ok {
        format!("<span class=\"pill pill-ok\">chain OK · {} entries</span>", status.count)
    } else {
        format!("<span class=\"pill pill-bad\">CHAIN BROKEN at seq {}</span>", status.first_bad_seq.unwrap_or(0))
    };
    let mut trs = String::new();
    for (seq, at, email, action, tt, tid, reason) in &rows {
        trs.push_str(&format!(
            "<tr><td>{seq}</td><td class=\"small\">{at}</td><td>{email}</td><td><b>{action}</b></td>\
<td class=\"small\">{target}</td><td class=\"small\">{reason}</td></tr>",
            at = escape(at), email = escape(email), action = escape(action),
            target = escape(&format!("{} {}", tt.as_deref().unwrap_or(""), tid.as_deref().unwrap_or(""))),
            reason = escape(reason.as_deref().unwrap_or("—")),
        ));
    }

    let body = format!(
        "<h2 class=\"mt\">Admin audit {chain}</h2>\
<div class=\"card\" style=\"padding:0\"><table><thead><tr>\
<th>Seq</th><th>At</th><th>Admin</th><th>Action</th><th>Target</th><th>Reason</th></tr></thead>\
<tbody>{trs}</tbody></table></div>",
    );
    Ok(Html(admin_page("Audit", "audit", Some(&session), &body)).into_response())
}

// --- time helpers ----------------------------------------------------------

fn expires_at(hours: i64) -> String {
    (time::OffsetDateTime::now_utc() + time::Duration::hours(hours))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
fn lock_until(minutes: i64) -> String {
    (time::OffsetDateTime::now_utc() + time::Duration::minutes(minutes))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
