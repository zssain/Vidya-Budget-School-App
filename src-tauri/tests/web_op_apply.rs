//! Cross-language contract (Phase 20, C3): an op shaped exactly as the PWA's op-builder
//! produces — a `class:<id>` homework_note INSERT whose payload carries the full row
//! columns (minus the bookkeeping apply_upsert adds) — is accepted by the REAL server
//! apply path (`sync::apply::apply_op`) and creates a complete, confirmed row. If the web
//! payload columns / audience / kind drift from what the server expects, this fails. It is
//! the guard the write path needs: a wrong op shape otherwise silently breaks sync.

use serde_json::json;
use time::OffsetDateTime;

use vidya_lib::sync::apply::apply_op;
use vidya_lib::sync::protocol::{Op, OpStatus};
use vidya_lib::{db, seed};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Build the op the web `save_homework_note` emits (payload is a JSON OBJECT, matching
/// the web op-builder — the desktop command stringifies, but the web sends an object and
/// apply_upsert reads `payload.as_object()`).
fn web_homework_op() -> Op {
    Op {
        op_id: "op-web-1".into(),
        hlc: "2026-10-03T12:00:00Z".into(),
        device_id: "dev-web".into(),
        staff_id: "stf-meena".into(), // class-teaches cls-5a → holds class:cls-5a
        audience: "class:cls-5a".into(),
        table: "homework_note".into(),
        record_id: "hw-web-1".into(),
        kind: "insert".into(),
        // homework_note is not `is_synced` → the payload must carry EVERY column itself
        // (the exact set the web op-builder sends).
        payload: json!({
            "class_id": "cls-5a",
            "class_subject_id": "cs-5a-eng",
            "kind": "homework",
            "text": "Read chapter 3",
            "attachments_json": "[]",
            "shared_json": "[]",
            "created_by": "stf-meena",
            "school_id": "sch-demo",
            "created_at": "2026-10-03T12:00:00Z",
            "updated_at": "2026-10-03T12:00:00Z",
            "updated_by_staff": "stf-meena",
            "updated_by_device": "dev-web",
            "sync_state": "confirmed"
        }),
        base_version: None,
        server_epoch: 1,
    }
}

#[test]
fn web_homework_note_insert_op_applies_on_the_server() {
    let mut conn = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut conn).unwrap(); // seeds module_setting: classroom ON
    seed::seed_demo_school(&mut conn, OffsetDateTime::now_utc()).unwrap();

    let op = web_homework_op();
    let res = apply_op(&mut conn, &op).unwrap();
    assert_eq!(res.status, OpStatus::Confirmed, "server accepted the web op: {res:?}");

    // The row exists, complete, with the server's sync bookkeeping applied.
    let (text, class_id, sync_state): (String, String, String) = conn
        .query_row(
            "SELECT text, class_id, sync_state FROM homework_note WHERE id='hw-web-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(text, "Read chapter 3");
    assert_eq!(class_id, "cls-5a");
    assert_eq!(sync_state, "confirmed", "apply_upsert marks an imported op confirmed");

    // Idempotent: the same op_id re-applies to the remembered result, no duplicate row.
    let again = apply_op(&mut conn, &op).unwrap();
    assert_eq!(again.status, OpStatus::Confirmed);
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM homework_note WHERE id='hw-web-1'", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
}
