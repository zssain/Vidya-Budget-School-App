//! Join-via-Drive (Phase 19, Step 5): the school PC answers a PWA's sealed join
//! request with a sealed join response the joining device (holding the invite code)
//! can open — provisioning a web device through the SAME `/join` as Android. Same
//! seed + invite + join setup as sync_e2e (the proven context).

use rusqlite::Connection;
use time::OffsetDateTime;

use vidya_lib::server::service;
use vidya_lib::sync::drive::join::{answer_join_request, DriveJoinRequest};
use vidya_lib::sync::protocol::JoinResp;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
}

fn server() -> Connection {
    let mut c = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut c).unwrap();
    seed::seed_demo_school(&mut c, now()).unwrap();
    c
}

#[test]
fn a_web_platform_device_can_join() {
    // Regression for migration 0031: a PWA joins with platform "web", which the
    // device table's CHECK must allow.
    let mut c = server();
    let code = service::create_invite(&mut c, "stf-meena", now()).unwrap();
    let jr = service::join(
        &mut c,
        &vidya_lib::sync::protocol::JoinReq {
            invite_code: code,
            device_name: "iPhone".into(),
            platform: "web".into(),
            google_email: None,
        },
        now(),
    )
    .unwrap();
    assert!(!jr.device_token.is_empty());
    let plat: String = c.query_row("SELECT platform FROM device WHERE id=?1", [&jr.device_id], |r| r.get(0)).unwrap();
    assert_eq!(plat, "web");
}

#[test]
fn pwa_join_via_drive_answers_with_a_sealed_response_the_device_can_open() {
    let mut c = server();
    let code = service::create_invite(&mut c, "stf-meena", now()).unwrap();

    let req = DriveJoinRequest {
        request_id: "req-abc".into(),
        invite_code: code.clone(),
        device_name: "iPhone".into(),
        platform: "web".into(),
    };
    let sealed = answer_join_request(&mut c, &serde_json::to_vec(&req).unwrap(), now()).unwrap();

    // The joining device derives the same key from its invite code and opens it.
    let key = vidya_core::seal::join_key(&code);
    let aad = vidya_core::seal::join_aad("req-abc");
    let plain = vidya_core::seal::open(&key, &aad, &sealed).unwrap();
    let resp: JoinResp = serde_json::from_slice(&plain).unwrap();
    assert!(!resp.device_token.is_empty(), "a device token was issued");
    assert!(!resp.session_key.is_empty(), "a session key was issued");
    assert!(!resp.audience_keys.is_empty(), "audience keys were issued");
    assert_eq!(resp.staff.id, "stf-meena");

    // A wrong invite code cannot open the response; nor a different request id.
    assert!(vidya_core::seal::open(&vidya_core::seal::join_key("WRONG"), &aad, &sealed).is_err());
    assert!(vidya_core::seal::open(&key, &vidya_core::seal::join_aad("req-other"), &sealed).is_err());

    // The single-use invite is now spent — a second answer fails.
    assert!(
        answer_join_request(&mut c, &serde_json::to_vec(&req).unwrap(), now()).is_err(),
        "invite is single-use"
    );
}
