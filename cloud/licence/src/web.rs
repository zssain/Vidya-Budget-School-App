//! The public website (prompts/P10 Step 3). Server-rendered HTML, the app's
//! tokens + the Welcome-screen look. Purchase model = manual UPI (owner decision):
//! the buyer scans the company UPI QR and pays, then a company admin verifies the
//! payment and issues the licence (Step 5). A browser action NEVER grants a
//! licence — submitting a UPI reference only records a claim to be verified.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Form, Router};
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use std::time::Duration;

use crate::crypto;
use crate::db::{new_id, now_iso};
use crate::error::AppError;
use crate::releases;
use crate::state::AppState;
use crate::ui::{attr, escape, page};

const LOGO_LIGHT: &str = include_str!("../assets/logo-on-light.svg");
const LOGO_DARK: &str = include_str!("../assets/logo-on-dark.svg");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/pricing", get(pricing))
        .route("/checkout", get(checkout_form).post(checkout_submit))
        .route("/pay/:order_ref", get(pay_page).post(pay_claim))
        .route("/account", get(account_form).post(account_lookup))
        .route("/downloads", get(downloads))
        .route("/support", get(support))
        .route("/assets/logo-on-light.svg", get(logo_light))
        .route("/assets/logo-on-dark.svg", get(logo_dark))
        .route("/assets/upi-qr", get(upi_qr))
}

fn svg(body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, "image/svg+xml")], body).into_response()
}
async fn logo_light() -> Response {
    svg(LOGO_LIGHT)
}
async fn logo_dark() -> Response {
    svg(LOGO_DARK)
}

/// Serve the owner-supplied UPI QR image, or a placeholder note if none is set.
async fn upi_qr(State(state): State<AppState>) -> Response {
    match &state.cfg.upi_qr_path {
        Some(p) => match std::fs::read(p) {
            Ok(bytes) => {
                let ct = match p.extension().and_then(|e| e.to_str()) {
                    Some("svg") => "image/svg+xml",
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    _ => "image/png",
                };
                ([(header::CONTENT_TYPE, ct)], bytes).into_response()
            }
            Err(_) => (StatusCode::NOT_FOUND, "no QR configured").into_response(),
        },
        None => (StatusCode::NOT_FOUND, "no QR configured").into_response(),
    }
}

// --- Home ------------------------------------------------------------------

async fn home(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let body =
        "<div class=\"panel-navy\"><div class=\"eyebrow\">School management, done simply</div>\
<h1>Vidya Budget School</h1>\
<p class=\"sub\">Admissions, attendance, marks, fees and reports for small Indian schools — \
built to work offline first, in English and Hindi. Your school's data lives on your own PC; \
nothing is ever silently overwritten or lost.</p>\
<div class=\"row mt2\"><a class=\"btn btn-primary\" href=\"/pricing\">See pricing</a>\
<a class=\"btn btn-ghost\" href=\"/downloads\">Download</a></div></div>\
<div class=\"grid cols-3 mt2\">\
<div class=\"card\"><h3>Windows, macOS & Android</h3><p class=\"muted small\">One school server on \
the Principal's PC; teachers and accountants join from phones and PCs on the school Wi-Fi or over \
the internet.</p></div>\
<div class=\"card\"><h3>Under 40 MB each</h3><p class=\"muted small\">Every download is under 40 MB \
and runs on a 4 GB PC or a 2 GB phone. The school's data is stored separately, on the school's \
own devices.</p></div>\
<div class=\"card\"><h3>Encrypted & offline-first</h3><p class=\"muted small\">Encrypted on every \
device and before anything syncs. Works with no internet; changes are sent later. English and \
Hindi throughout.</p></div></div>\
<div class=\"card mt2\"><h2>How buying works</h2><ol class=\"muted\">\
<li>Choose your plan and check out with your school's details.</li>\
<li>Pay by scanning our UPI QR code in any UPI app.</li>\
<li>We verify the payment and issue your activation code — view it any time on the \
<a href=\"/account\">Account</a> page.</li>\
<li>Install Vidya on your school's main PC and activate with the code.</li></ol></div>";
    Html(page(cfg, "Home", "home", body))
}

// --- Pricing ---------------------------------------------------------------

async fn pricing(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let body = format!(
        "<div class=\"eyebrow\">Pricing</div><h1>One-time purchase</h1>\
<p class=\"sub\">Vidya is a one-time purchase — a perpetual licence for one school, with no \
expiry and no subscription.</p>\
<div class=\"grid cols-2 mt2\"><div class=\"card\">\
<h3>Vidya for one school</h3>\
<p style=\"font-family:var(--serif);font-size:34px;margin:6px 0\">{price}</p>\
<p class=\"muted small\">Perpetual licence · one school server · unlimited staff devices.<br>\
Taxes/GST: [GST — owner to confirm].</p>\
<a class=\"btn btn-primary mt\" href=\"/checkout\">Buy now</a></div>\
<div class=\"card\"><h3>What's included</h3><ul class=\"muted small\">\
<li>Admissions, attendance, marks, fees, receipts and reports</li>\
<li>English & Hindi, report cards and receipts in both</li>\
<li>Backups and safe restore to a new PC</li>\
<li>Free updates for the 1.x series</li></ul></div></div>\
<p class=\"muted small mt2\">Prices, taxes and refund terms are set by {company}. \
See <a href=\"{terms}\">Terms</a>.</p>",
        price = escape(&cfg.price_text),
        company = escape(&cfg.company_name),
        terms = attr(&cfg.terms_url),
    );
    Html(page(cfg, "Pricing", "pricing", &body))
}

// --- Checkout --------------------------------------------------------------

async fn checkout_form(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let body = format!(
        "<div class=\"eyebrow\">Checkout</div><h1>Your school's details</h1>\
<p class=\"sub\">We use these to issue your licence and to help if you ever need support. \
The next step shows our UPI QR code to pay.</p>\
<form method=\"post\" action=\"/checkout\" class=\"card mt2\" style=\"max-width:520px\">\
<label for=\"name\">Your name</label><input id=\"name\" name=\"name\" type=\"text\" required>\
<label for=\"email\">Email</label><input id=\"email\" name=\"email\" type=\"email\" required>\
<label for=\"phone\">Mobile (10 digits)</label><input id=\"phone\" name=\"phone\" type=\"tel\" required>\
<label for=\"school_name\">School name</label><input id=\"school_name\" name=\"school_name\" type=\"text\" required>\
<div class=\"mt\"><button class=\"btn btn-primary\" type=\"submit\">Continue to payment</button></div>\
<p class=\"muted small mt\">Price: {price}. You'll pay by scanning our UPI QR in the next step.</p>\
</form>",
        price = escape(&cfg.price_text),
    );
    Html(page(cfg, "Checkout", "", &body))
}

#[derive(Deserialize)]
struct CheckoutForm {
    name: String,
    email: String,
    phone: String,
    school_name: String,
}

fn valid_email(s: &str) -> bool {
    let s = s.trim();
    s.len() >= 3 && s.contains('@') && !s.starts_with('@') && !s.ends_with('@') && !s.contains(' ')
}
fn valid_mobile(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.len() == 10 && matches!(digits.as_bytes()[0], b'6'..=b'9')
}

async fn checkout_submit(
    State(state): State<AppState>,
    Form(f): Form<CheckoutForm>,
) -> Result<Response, AppError> {
    // Basic validation mirroring §3.10 (10-digit mobile, plausible email).
    let name = f.name.trim();
    let email = f.email.trim();
    let school = f.school_name.trim();
    if name.is_empty() || school.is_empty() || !valid_email(email) || !valid_mobile(&f.phone) {
        let cfg = &state.cfg;
        let body = "<div class=\"notice notice-bad\">Please enter your name, a valid email, a \
10-digit mobile starting 6–9, and your school name.</div><p class=\"mt\"><a class=\"btn btn-line\" \
href=\"/checkout\">Go back</a></p>";
        return Ok(Html(page(cfg, "Checkout", "", body)).into_response());
    }

    let order_ref = format!("VIDYA-ORD-{}", short_ref());
    {
        let conn = state.db.lock().unwrap();
        let now = now_iso();
        let customer_id = new_id();
        conn.execute(
            "INSERT INTO customer (id, email, name, phone, school_name, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![customer_id, email, name, f.phone.trim(), school, now],
        )?;
        conn.execute(
            "INSERT INTO purchase_order
               (id, customer_id, provider, provider_order_id, amount_paise, currency, plan, status, created_at, updated_at)
             VALUES (?1,?2,'upi_manual',?3,0,'INR','perpetual','created',?4,?4)",
            params![new_id(), customer_id, order_ref, now],
        )?;
    }
    Ok(Redirect::to(&format!("/pay/{order_ref}")).into_response())
}

// --- Pay (UPI QR) ----------------------------------------------------------

struct OrderView {
    provider_order_id: String,
    status: String,
    school_name: String,
    email: String,
}

fn load_order(state: &AppState, order_ref: &str) -> rusqlite::Result<Option<OrderView>> {
    let conn = state.db.lock().unwrap();
    conn.query_row(
        "SELECT o.provider_order_id, o.status, c.school_name, c.email
           FROM purchase_order o JOIN customer c ON c.id = o.customer_id
          WHERE o.provider_order_id = ?1",
        [order_ref],
        |r| {
            Ok(OrderView {
                provider_order_id: r.get(0)?,
                status: r.get(1)?,
                school_name: r.get(2)?,
                email: r.get(3)?,
            })
        },
    )
    .optional()
}

async fn pay_page(State(state): State<AppState>, Path(order_ref): Path<String>) -> Result<Response, AppError> {
    let cfg = &state.cfg;
    let order = match load_order(&state, &order_ref)? {
        Some(o) => o,
        None => return Ok(not_found_page(&state)),
    };
    if order.status == "paid" {
        let body = format!(
            "<div class=\"notice notice-info\">This order is already paid and your licence is issued. \
View your activation code on the <a href=\"/account\">Account</a> page \
(order <b>{}</b> + your email).</div>",
            escape(&order.provider_order_id)
        );
        return Ok(Html(page(cfg, "Payment", "", &body)).into_response());
    }
    let claimed = order.status == "awaiting_verification";
    let qr_area = if cfg.upi_qr_path.is_some() {
        "<img src=\"/assets/upi-qr\" alt=\"UPI QR code\">".to_string()
    } else {
        "Your UPI QR code<br>[UPI QR — owner to add]".to_string()
    };
    let claim_state = if claimed {
        "<div class=\"notice notice-warn mt\">We've recorded that you've paid and are verifying it. \
Once verified, your activation code appears on the <a href=\"/account\">Account</a> page. \
You can re-submit your reference below if it changed.</div>"
    } else {
        ""
    };
    let body = format!(
        "<div class=\"eyebrow\">Payment</div><h1>Scan & pay</h1>\
<p class=\"sub\">Open any UPI app, scan the QR code and pay <b>{price}</b> for order \
<b>{order}</b> ({school}). After paying, tell us below so we can verify and issue your code.</p>\
<div class=\"grid cols-2 mt2\">\
<div class=\"card\"><div class=\"qr\">{qr}</div>\
<p class=\"dl-meta mt\">UPI ID: <b>{upi}</b></p>\
<p class=\"dl-meta\">Amount: <b>{price}</b></p></div>\
<div class=\"card\"><h3>After you've paid</h3>\
<p class=\"muted small\">Enter the UPI reference / UTR from your payment (optional but it helps us \
match your payment faster), then tap <b>I've paid</b>. A company admin verifies the payment before \
your licence is issued — paying is what grants the licence, never this button.</p>{claim_state}\
<form method=\"post\" action=\"/pay/{order}\" class=\"mt\">\
<label for=\"upi_ref\">UPI reference / UTR (optional)</label>\
<input id=\"upi_ref\" name=\"upi_ref\" type=\"text\">\
<div class=\"mt\"><button class=\"btn btn-primary\" type=\"submit\">I've paid</button></div></form>\
</div></div>\
<p class=\"muted small mt2\">Keep your order number <b>{order}</b> — you'll use it with your email on \
the <a href=\"/account\">Account</a> page to see your activation code once it's issued.</p>",
        price = escape(&cfg.price_text),
        order = escape(&order.provider_order_id),
        school = escape(&order.school_name),
        qr = qr_area,
        upi = escape(&cfg.upi_id),
        claim_state = claim_state,
    );
    Ok(Html(page(cfg, "Payment", "", &body)).into_response())
}

#[derive(Deserialize)]
struct ClaimForm {
    #[serde(default)]
    upi_ref: String,
}

async fn pay_claim(
    State(state): State<AppState>,
    Path(order_ref): Path<String>,
    Form(f): Form<ClaimForm>,
) -> Result<Response, AppError> {
    let cfg = &state.cfg;
    let order = match load_order(&state, &order_ref)? {
        Some(o) => o,
        None => return Ok(not_found_page(&state)),
    };
    if order.status == "created" || order.status == "awaiting_verification" {
        let conn = state.db.lock().unwrap();
        let now = now_iso();
        // Record the buyer's CLAIM (never verified here) — idempotent per order.
        let oid: String = conn.query_row(
            "SELECT id FROM purchase_order WHERE provider_order_id = ?1",
            [&order_ref],
            |r| r.get(0),
        )?;
        let upi = f.upi_ref.trim();
        conn.execute(
            "INSERT OR REPLACE INTO payment_event
               (id, provider_event_id, order_id, kind, amount_paise, upi_ref, actor, verified, raw_json, received_at)
             VALUES (?1, ?2, ?3, 'buyer_claim', NULL, ?4, 'buyer', 0, ?5, ?6)",
            params![
                new_id(),
                format!("claim:{oid}"),
                oid,
                if upi.is_empty() { None } else { Some(upi) },
                serde_json::json!({ "upi_ref": upi }).to_string(),
                now,
            ],
        )?;
        conn.execute(
            "UPDATE purchase_order SET status = 'awaiting_verification', upi_ref = COALESCE(NULLIF(?2,''), upi_ref), updated_at = ?3 WHERE id = ?1",
            params![oid, upi, now],
        )?;
    }
    let body = format!(
        "<div class=\"notice notice-info\">Thank you — we've recorded that order <b>{order}</b> is paid. \
A company admin will verify the payment and issue your activation code, usually within a business day. \
Check the <a href=\"/account\">Account</a> page (order number + your email) to see your code once it's ready.</div>",
        order = escape(&order.provider_order_id),
    );
    Ok(Html(page(cfg, "Payment received", "", &body)).into_response())
}

// --- Account ---------------------------------------------------------------

async fn account_form(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let body = "<div class=\"eyebrow\">Account</div><h1>Find your activation code</h1>\
<p class=\"sub\">Enter your order number and the email you used at checkout to see your order \
status and your activation code once it's issued.</p>\
<form method=\"post\" action=\"/account\" class=\"card mt2\" style=\"max-width:520px\">\
<label for=\"order\">Order number</label><input id=\"order\" name=\"order\" type=\"text\" placeholder=\"VIDYA-ORD-XXXXXX\" required>\
<label for=\"email\">Email</label><input id=\"email\" name=\"email\" type=\"email\" required>\
<div class=\"mt\"><button class=\"btn btn-primary\" type=\"submit\">View order</button></div></form>";
    Html(page(cfg, "Account", "account", body))
}

#[derive(Deserialize)]
struct AccountForm {
    order: String,
    email: String,
}

async fn account_lookup(
    State(state): State<AppState>,
    Form(f): Form<AccountForm>,
) -> Result<Response, AppError> {
    let cfg = &state.cfg;
    let order_ref = f.order.trim();
    let email = f.email.trim().to_ascii_lowercase();

    // Rate-limit lookups per IP-less key on the order to slow guessing.
    if !state.rl.lock().unwrap().allow(&format!("acct:{order_ref}"), 10, Duration::from_secs(600)) {
        let body = "<div class=\"notice notice-warn\">Too many attempts. Please wait a few minutes and try again.</div>";
        return Ok(Html(page(cfg, "Account", "account", body)).into_response());
    }

    let (order, code) = {
        let conn = state.db.lock().unwrap();
        let order: Option<OrderView> = conn
            .query_row(
                "SELECT o.provider_order_id, o.status, c.school_name, c.email
                   FROM purchase_order o JOIN customer c ON c.id = o.customer_id
                  WHERE o.provider_order_id = ?1",
                [order_ref],
                |r| Ok(OrderView { provider_order_id: r.get(0)?, status: r.get(1)?, school_name: r.get(2)?, email: r.get(3)? }),
            )
            .optional()?;

        match order {
            // Generic mismatch answer (does not reveal which field was wrong).
            Some(o) if o.email.trim().to_ascii_lowercase() == email => {
                let code = if o.status == "paid" {
                    let enc: Option<String> = conn
                        .query_row(
                            "SELECT code_enc FROM activation_code
                              WHERE order_id = (SELECT id FROM purchase_order WHERE provider_order_id = ?1)
                                AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1",
                            [order_ref],
                            |r| r.get(0),
                        )
                        .optional()?;
                    enc.and_then(|e| crypto::decrypt(&cfg.code_enc_key, &e)).and_then(|b| String::from_utf8(b).ok())
                } else {
                    None
                };
                (Some(o), code)
            }
            _ => (None, None),
        }
    };

    let body = match order {
        None => "<div class=\"notice notice-bad\">We couldn't find an order with that number and email. \
Please check both and try again, or contact support.</div><p class=\"mt\"><a class=\"btn btn-line\" href=\"/account\">Try again</a></p>".to_string(),
        Some(o) => {
            let status_pill = match o.status.as_str() {
                "paid" => "<span class=\"pill pill-ok\">Paid · licence issued</span>",
                "awaiting_verification" => "<span class=\"pill pill-wait\">Payment being verified</span>",
                "failed" => "<span class=\"pill pill-bad\">Payment not verified</span>",
                "refunded" => "<span class=\"pill pill-info\">Refunded</span>",
                _ => "<span class=\"pill pill-wait\">Awaiting payment</span>",
            };
            let code_block = match code {
                Some(c) => format!(
                    "<h3 class=\"mt2\">Your activation code</h3><div class=\"code-box\">{}</div>\
<p class=\"muted small mt\">Enter this on the school's main PC when you install Vidya. \
Keep it private — anyone with the code could activate your school.</p>\
<a class=\"btn btn-primary mt\" href=\"/downloads\">Go to downloads</a>",
                    escape(&c)
                ),
                None if o.status == "awaiting_verification" => "<p class=\"muted mt\">We're verifying your \
payment. Your activation code will appear here once it's confirmed — usually within a business day.</p>".to_string(),
                None => "<p class=\"muted mt\">No activation code yet. Complete payment on the \
payment page, then return here.</p>".to_string(),
            };
            format!(
                "<div class=\"eyebrow\">Account</div><h1>Order {order}</h1>\
<div class=\"card mt2\"><dl class=\"kv\">\
<dt>School</dt><dd>{school}</dd>\
<dt>Status</dt><dd>{pill}</dd></dl>{code}</div>",
                order = escape(&o.provider_order_id),
                school = escape(&o.school_name),
                pill = status_pill,
                code = code_block,
            )
        }
    };
    Ok(Html(page(cfg, "Account", "account", &body)).into_response())
}

// --- Downloads / Support ---------------------------------------------------

async fn downloads(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let rel = releases::load(cfg);
    let body = releases::render(cfg, rel.as_ref());
    Html(page(cfg, "Downloads", "downloads", &body))
}

async fn support(State(state): State<AppState>) -> Html<String> {
    let cfg = &state.cfg;
    let body = format!(
        "<div class=\"eyebrow\">Support</div><h1>We're here to help</h1>\
<div class=\"grid cols-2 mt2\">\
<div class=\"card\"><h3>Contact</h3><p class=\"muted\">Email: <b>{email}</b><br>Phone: <b>{phone}</b></p>\
<p class=\"muted small\">Have your order number ready (<a href=\"/account\">Account</a> page) so we \
can find your licence quickly.</p></div>\
<div class=\"card\"><h3>Common questions</h3><ul class=\"muted small\">\
<li><b>Lost your code?</b> View it any time on the <a href=\"/account\">Account</a> page.</li>\
<li><b>New PC?</b> Vidya can move to a new computer safely — see the app's Restore screen, or \
contact us to approve the move.</li>\
<li><b>Install help?</b> The <a href=\"/downloads\">Downloads</a> page lists requirements and \
install steps for each platform.</li></ul></div></div>",
        email = escape(&cfg.support_email),
        phone = escape(&cfg.support_phone),
    );
    Html(page(cfg, "Support", "support", &body))
}

// --- helpers ---------------------------------------------------------------

fn not_found_page(state: &AppState) -> Response {
    let body = "<div class=\"notice notice-bad\">We couldn't find that page or order.</div>\
<p class=\"mt\"><a class=\"btn btn-line\" href=\"/\">Home</a></p>";
    (StatusCode::NOT_FOUND, Html(page(&state.cfg, "Not found", "", body))).into_response()
}

/// A short, human-friendly order reference from Crockford characters.
fn short_ref() -> String {
    use rand::RngCore;
    const A: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut b = [0u8; 6];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b.iter().map(|x| A[(*x % 32) as usize] as char).collect()
}
