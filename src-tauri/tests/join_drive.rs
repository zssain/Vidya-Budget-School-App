//! Join-via-Drive, protocol v2 (Phase 19 Step 5 + P21): the school PC answers a PWA's
//! join request — which carries only an HMAC proof of the invite's web secret — with a
//! response sealed so ONLY the holder of that secret can open it. Includes the attack
//! cases: a reader of the joins folder (request + response, no secret) learns nothing;
//! a wrong/forged proof, altered request fields, replay and v1 requests are refused.

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

/// What the PWA does: invite id + secret from the link → request with proof.
fn web_request(c: &Connection, code: &str, request_id: &str) -> (DriveJoinRequest, String, Vec<u8>) {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let (invite_id, secret_b64) = service::web_invite(c, code).unwrap().expect("web invite");
    let secret = URL_SAFE_NO_PAD.decode(&secret_b64).unwrap();
    assert_eq!(secret.len(), 32, "256-bit web secret");
    let proof = vidya_core::seal::join_v2_proof(&secret, &vidya_core::seal::join_v2_transcript(&invite_id, request_id, "iPhone", "web"));
    let req = DriveJoinRequest {
        v: 2,
        invite_id: invite_id.clone(),
        request_id: request_id.into(),
        device_name: "iPhone".into(),
        platform: "web".into(),
        proof: URL_SAFE_NO_PAD.encode(proof),
    };
    (req, invite_id, secret)
}

fn open_resp(secret: &[u8], invite_id: &str, request_id: &str, sealed: &[u8]) -> Option<JoinResp> {
    let key = vidya_core::seal::join_v2_response_key(secret, request_id);
    let aad = vidya_core::seal::join_v2_aad(invite_id, request_id, "sch-demo");
    vidya_core::seal::open(&key, &aad, sealed).ok().map(|p| serde_json::from_slice(&p).unwrap())
}

#[test]
fn pwa_join_via_drive_answers_with_a_sealed_response_the_device_can_open() {
    let mut c = server();
    let code = service::create_invite(&mut c, "stf-meena", now()).unwrap();
    let (req, invite_id, secret) = web_request(&c, &code, "req-abc");
    let req_bytes = serde_json::to_vec(&req).unwrap();

    // The request file carries neither the code nor the secret.
    let req_text = String::from_utf8(req_bytes.clone()).unwrap();
    assert!(!req_text.contains(&code), "short code never uploaded");
    use base64::Engine;
    assert!(!req_text.contains(&base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&secret)), "secret never uploaded");

    let sealed = answer_join_request(&mut c, &req_bytes, now()).unwrap();
    let resp = open_resp(&secret, &invite_id, "req-abc", &sealed).expect("device opens its response");
    assert!(!resp.device_token.is_empty() && !resp.session_key.is_empty() && !resp.audience_keys.is_empty());
    assert_eq!(resp.staff.id, "stf-meena");

    // A Drive reader holds the request + response but not the secret: nothing derivable
    // from those bytes (code hash, invite id, request id, proof) opens it.
    let proof = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&req.proof).unwrap();
    for guess in [proof.clone(), invite_id.as_bytes().to_vec(), code.as_bytes().to_vec(), vec![0u8; 32]] {
        assert!(open_resp(&guess, &invite_id, "req-abc", &sealed).is_none());
    }
    // Wrong request id / binding cannot open it either.
    assert!(open_resp(&secret, &invite_id, "req-other", &sealed).is_none());

    // The invite is single-use: replaying the same request, or a fresh valid request,
    // after consumption fails; and the web secret is cleared from the DB.
    assert!(answer_join_request(&mut c, &req_bytes, now()).is_err(), "replay refused");
    let left: Option<String> = c.query_row("SELECT web_secret FROM invite WHERE id=?1", [&invite_id], |r| r.get(0)).unwrap();
    assert!(left.is_none(), "secret wiped after use");
}

#[test]
fn forged_or_altered_join_requests_are_refused_and_do_not_consume_the_invite() {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let mut c = server();
    let code = service::create_invite(&mut c, "stf-meena", now()).unwrap();
    let (good, invite_id, _secret) = web_request(&c, &code, "req-1");

    // 1) Someone who only saw the folder: proof computed with a guessed secret.
    let mut forged = good.clone();
    forged.proof = URL_SAFE_NO_PAD.encode(vidya_core::seal::join_v2_proof(&[7u8; 32], &vidya_core::seal::join_v2_transcript(&invite_id, "req-1", "iPhone", "web")));
    assert!(answer_join_request(&mut c, &serde_json::to_vec(&forged).unwrap(), now()).is_err());
    // 2) A valid proof replayed under another request id / device name / platform.
    for tweak in 0..3 {
        let mut t = good.clone();
        match tweak {
            0 => t.request_id = "req-attacker".into(),
            1 => t.device_name = "Evil phone".into(),
            _ => t.platform = "android".into(),
        }
        assert!(answer_join_request(&mut c, &serde_json::to_vec(&t).unwrap(), now()).is_err(), "tweak {tweak}");
    }
    // 3) An old v1-style request (plaintext invite code) is refused outright.
    let v1 = serde_json::json!({ "request_id": "r", "invite_code": code, "device_name": "iPhone", "platform": "web" });
    assert!(answer_join_request(&mut c, &serde_json::to_vec(&v1).unwrap(), now()).is_err());
    // None of the above consumed the invite: the genuine request still works.
    assert!(answer_join_request(&mut c, &serde_json::to_vec(&good).unwrap(), now()).is_ok());
}
