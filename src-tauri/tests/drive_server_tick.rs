//! The school-PC live Drive loop (Phase C step 1): one `server_drive_tick` pass over a
//! fake Drive answers a pending PWA join request (writing the sealed response the
//! joining device can open) and is idempotent. Import+ack is covered by drive_e2e; here
//! we prove the server-side composition (join answering) over the DriveApi.

use std::collections::BTreeMap;

use time::OffsetDateTime;

use vidya_lib::server::service;
use vidya_lib::sync::drive::fake::{FakeDrive, PRINCIPAL};
use vidya_lib::sync::drive::join::DriveJoinRequest;
use vidya_lib::sync::drive::live;
use vidya_lib::sync::drive::DriveApi;
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
}

#[test]
fn server_tick_answers_a_pending_join_and_is_idempotent() {
    // A seeded school + an invite for a real staff member.
    let mut conn = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut conn).unwrap();
    seed::seed_demo_school(&mut conn, now()).unwrap();
    let code = service::create_invite(&mut conn, "stf-meena", now()).unwrap();

    // The school's sync account (one identity, full access — the model the real school
    // PC uses). A PWA dropped a plaintext join request into exchange/joins/.
    let drive = FakeDrive::new();
    let layout = drive.provision_school(&[]);
    let acct = drive.as_actor(PRINCIPAL);
    let joins = acct.ensure_folder(&layout.exchange, "joins").unwrap();
    let req = DriveJoinRequest {
        request_id: "req1".into(),
        invite_code: code,
        device_name: "iPhone".into(),
        platform: "web".into(),
    };
    acct.create(&joins, "req1.vjoin", &serde_json::to_vec(&req).unwrap(), &BTreeMap::new()).unwrap();

    // One pass answers it: a sealed response appears + a web device is provisioned.
    let out = live::server_drive_tick(&mut conn, &acct, &layout.exchange, now()).unwrap();
    assert_eq!(out.joins_answered, 1);
    assert!(acct.list(&joins).unwrap().iter().any(|f| f.name == "req1.resp.vjoin"), "sealed response written");
    let web_devices: i64 = conn.query_row("SELECT COUNT(*) FROM device WHERE platform='web'", [], |r| r.get(0)).unwrap();
    assert_eq!(web_devices, 1, "the PWA was provisioned via /join");

    // Idempotent: a second pass does not re-answer (the response already exists; the
    // single-use invite is spent anyway).
    let again = live::server_drive_tick(&mut conn, &acct, &layout.exchange, now()).unwrap();
    assert_eq!(again.joins_answered, 0);
}
