//! HTTP-level Step-8 tests: generic API errors, admin session + CSRF, the
//! buyer-claim path never issuing a licence ("a browser action never grants a
//! licence"), and rate limits. Drives the real router with `oneshot`.

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use std::net::SocketAddr;
use tower::util::ServiceExt;
use vidya_licence::config::Config;
use vidya_licence::state::AppState;
use vidya_licence::{admin, app_router, db};

fn setup() -> (Router, AppState) {
    let conn = db::open_memory().unwrap();
    admin::ensure_bootstrap_admin(&conn, "admin@test", "pw-12345678").unwrap();
    let signing = ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]);
    let state = AppState::new(conn, signing, Config::test_default());
    (app_router(state.clone()), state)
}

struct Resp {
    status: StatusCode,
    location: Option<String>,
    set_cookie: Option<String>,
    body: String,
}

async fn send(router: &Router, method: Method, uri: &str, cookie: Option<&str>, form: Option<&str>, json: Option<&str>) -> Resp {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let body = if let Some(f) = form {
        b = b.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        Body::from(f.to_string())
    } else if let Some(j) = json {
        b = b.header(header::CONTENT_TYPE, "application/json");
        Body::from(j.to_string())
    } else {
        Body::empty()
    };
    let mut req = b.body(body).unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let location = resp.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()).map(String::from);
    let set_cookie = resp.headers().get(header::SET_COOKIE).and_then(|v| v.to_str().ok()).map(String::from);
    let bytes = axum::body::to_bytes(resp.into_body(), 2_000_000).await.unwrap();
    Resp { status, location, set_cookie, body: String::from_utf8_lossy(&bytes).to_string() }
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    let rest = &s[i..];
    let j = rest.find(end)?;
    Some(&rest[..j])
}

#[tokio::test]
async fn activate_unknown_code_is_generic_404() {
    let (app, _) = setup();
    let r = send(
        &app,
        Method::POST,
        "/v1/activate",
        None,
        None,
        Some(r#"{"code":"VIDYA-AAAA-AAAA-AAAA","school_name":"S","machine_id":"m","app_version":"1.0.0"}"#),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert!(r.body.contains("CODE_NOT_FOUND"), "generic error body: {}", r.body);
    // Must not leak whether other schools exist.
    assert!(!r.body.to_lowercase().contains("school_id"));
}

#[tokio::test]
async fn admin_requires_a_session() {
    let (app, _) = setup();
    let r = send(&app, Method::GET, "/admin/licences", None, None, None).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    assert_eq!(r.location.as_deref(), Some("/admin/login"));
}

#[tokio::test]
async fn login_sets_session_and_csrf_is_enforced() {
    let (app, _) = setup();

    // Wrong password → generic failure, no cookie.
    let bad = send(&app, Method::POST, "/admin/login", None, Some("email=admin@test&password=nope"), None).await;
    assert!(bad.set_cookie.is_none());

    // Correct login → 303 + Set-Cookie.
    let ok = send(&app, Method::POST, "/admin/login", None, Some("email=admin@test&password=pw-12345678"), None).await;
    assert_eq!(ok.status, StatusCode::SEE_OTHER);
    let cookie_hdr = ok.set_cookie.expect("Set-Cookie");
    assert!(cookie_hdr.contains("HttpOnly") && cookie_hdr.contains("SameSite=Strict"));
    let cookie = cookie_hdr.split(';').next().unwrap().to_string(); // vidya_admin=TOKEN

    // Authenticated page loads and carries a CSRF token.
    let page = send(&app, Method::GET, "/admin/licences", Some(&cookie), None, None).await;
    assert_eq!(page.status, StatusCode::OK);
    let csrf = between(&page.body, "name=\"csrf\" value=\"", "\"").expect("csrf in page").to_string();

    // A mutating action with a WRONG csrf is refused.
    let forged = send(&app, Method::POST, "/admin/licences/lic_x/note", Some(&cookie), Some("csrf=wrong&reason=hi&note=x"), None).await;
    assert_eq!(forged.status, StatusCode::FORBIDDEN);

    // The correct csrf is accepted (add-note audits regardless of licence id → 303).
    let good = send(
        &app,
        Method::POST,
        "/admin/licences/lic_x/note",
        Some(&cookie),
        Some(&format!("csrf={csrf}&reason=called+the+school&note=ok")),
        None,
    )
    .await;
    assert_eq!(good.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn buyer_claim_never_issues_a_licence() {
    let (app, state) = setup();
    // Checkout → order created, redirect to the pay page.
    let co = send(
        &app,
        Method::POST,
        "/checkout",
        None,
        Some("name=Ravi&email=ravi@school.test&phone=9812345678&school_name=Test+School"),
        None,
    )
    .await;
    assert_eq!(co.status, StatusCode::SEE_OTHER);
    let pay = co.location.expect("redirect to /pay");
    let order_ref = pay.strip_prefix("/pay/").unwrap().to_string();

    // Buyer says "I've paid" (a claim) — must NOT issue anything.
    let claim = send(&app, Method::POST, &pay, None, Some("upi_ref=UTR-999"), None).await;
    assert_eq!(claim.status, StatusCode::OK);

    let conn = state.db.lock().unwrap();
    let status: String = conn.query_row("SELECT status FROM purchase_order WHERE provider_order_id=?1", [&order_ref], |r| r.get(0)).unwrap();
    assert_eq!(status, "awaiting_verification");
    let licences: i64 = conn.query_row("SELECT COUNT(*) FROM licence", [], |r| r.get(0)).unwrap();
    assert_eq!(licences, 0, "a browser claim never grants a licence");
}

#[tokio::test]
async fn activate_is_rate_limited_per_code() {
    let (app, _) = setup();
    let body = r#"{"code":"VIDYA-BBBB-BBBB-BBBB","school_name":"S","machine_id":"m","app_version":"1.0.0"}"#;
    // CODE_MAX = 8 within the window; the 9th same-code attempt is throttled.
    let mut last = StatusCode::OK;
    for _ in 0..9 {
        last = send(&app, Method::POST, "/v1/activate", None, None, Some(body)).await.status;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn healthz_ok() {
    let (app, _) = setup();
    let r = send(&app, Method::GET, "/healthz", None, None, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body, "ok");
}
