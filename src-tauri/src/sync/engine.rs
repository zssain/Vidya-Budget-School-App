//! Client sync engine (prompts/P04 Step 6). One `sync_once` cycle: send the
//! outbox in HLC order, apply per-op results (confirmed→canonical row; rejected→
//! Inbox; conflict/flagged→status), then pull and apply changes. The periodic
//! loop (every 15 s while unlocked, backoff 15 s→5 min, on write/resume/network
//! change) wraps `sync_once` at runtime.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::now_iso;
use crate::sync::protocol::{Op, OpStatus, PullResp};
use crate::sync::relay::RelayTransport;
use crate::sync::transport::{HttpsTransport, LoopbackTransport, Transport, TransportError};

/// Sync-screen route labels (P05 Step 3 — exact copy).
pub const ROUTE_LAN: &str = "On school Wi-Fi";
pub const ROUTE_RELAY: &str = "Over the internet";
/// app_kv key: the highest server epoch this device has ever trusted (§8.9 fencing).
pub const KV_KNOWN_EPOCH: &str = "known_epoch";
/// app_kv key: the route label of the last successful sync (Sync screen).
pub const KV_LAST_ROUTE: &str = "last_route";

/// Backoff between failed sync attempts: 15 s doubling, capped at 5 min (§6).
pub fn backoff_secs(attempt: u32) -> u64 {
    let base = 15u64;
    base.saturating_mul(1u64 << attempt.min(6)).min(300)
}

/// Client-visible status for a push result (§8.10 + Step 6 copy).
pub fn status_label(status: OpStatus) -> &'static str {
    match status {
        OpStatus::Confirmed => "Confirmed by school server",
        OpStatus::Conflict => "Waiting for the Principal to review",
        OpStatus::Flagged => "Sent to the Principal for a check",
        OpStatus::Rejected => "Rejected — edit and resend",
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncOutcome {
    pub pushed: usize,
    pub confirmed: usize,
    pub conflicts: usize,
    pub flagged: usize,
    pub rejected: usize,
    pub pulled: usize,
}

/// Read the outbox (HLC order, ≤200) as protocol Ops.
fn outbox_ops(conn: &Connection) -> rusqlite::Result<Vec<Op>> {
    let mut stmt = conn.prepare(
        "SELECT op_id, hlc, device_id, staff_id, audience, \"table\", record_id, kind, payload, base_version, server_epoch \
         FROM outbox ORDER BY hlc ASC LIMIT 200",
    )?;
    let rows = stmt.query_map([], |r| {
        let payload: String = r.get(8)?;
        Ok(Op {
            op_id: r.get(0)?,
            hlc: r.get(1)?,
            device_id: r.get(2)?,
            staff_id: r.get(3)?,
            audience: r.get(4)?,
            table: r.get(5)?,
            record_id: r.get(6)?,
            kind: r.get(7)?,
            payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
            base_version: r.get(9)?,
            server_epoch: r.get(10)?,
        })
    })?;
    rows.collect()
}

fn cursor(conn: &Connection) -> rusqlite::Result<i64> {
    Ok(conn
        .query_row("SELECT last_server_seq FROM sync_cursor WHERE peer='server'", [], |r| r.get::<_, Option<i64>>(0))
        .optional()?
        .flatten()
        .unwrap_or(0))
}

fn set_cursor(conn: &Connection, seq: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sync_cursor(peer, last_server_seq, updated_at) VALUES ('server', ?1, ?2) \
         ON CONFLICT(peer) DO UPDATE SET last_server_seq=excluded.last_server_seq, updated_at=excluded.updated_at",
        params![seq, now_iso()],
    )?;
    Ok(())
}

/// Upsert a canonical row (from a push `record` or a pull `change`) into the
/// client DB, marking it confirmed. Local unsynced edits are only replaced once
/// acknowledged (the outbox op is removed on confirm).
fn upsert_canonical(conn: &Connection, table: &str, payload: &serde_json::Value) -> rusqlite::Result<()> {
    let obj = match payload.as_object() {
        Some(o) => o,
        None => return Ok(()),
    };
    if obj.is_empty() {
        return Ok(());
    }
    let cols: Vec<String> = obj.keys().cloned().collect();
    let place: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let sql = format!("INSERT OR REPLACE INTO {} ({}) VALUES ({})", table, cols.join(","), place.join(","));
    let vals: Vec<Box<dyn rusqlite::ToSql>> = cols
        .iter()
        .map(|c| match &obj[c] {
            serde_json::Value::Null => Box::new(Option::<String>::None) as Box<dyn rusqlite::ToSql>,
            serde_json::Value::Bool(b) => Box::new(if *b { 1i64 } else { 0 }),
            serde_json::Value::Number(n) => n.as_i64().map(|i| Box::new(i) as Box<dyn rusqlite::ToSql>).unwrap_or_else(|| Box::new(n.to_string())),
            serde_json::Value::String(s) => Box::new(s.clone()),
            other => Box::new(other.to_string()),
        })
        .collect();
    let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())?;
    Ok(())
}

fn apply_pull(conn: &Connection, pull: &PullResp) -> rusqlite::Result<usize> {
    for ch in &pull.changes {
        upsert_canonical(conn, &ch.table, &ch.payload)?;
    }
    for del in &pull.deletes {
        conn.execute(&format!("DELETE FROM {} WHERE id=?1", del.table), params![del.record_id]).ok();
    }
    set_cursor(conn, pull.next_cursor)?;
    Ok(pull.changes.len())
}

/// The highest server epoch this device has ever trusted (§8.9 fencing).
pub fn known_epoch(conn: &Connection) -> rusqlite::Result<i64> {
    Ok(crate::kv::get::<i64>(conn, KV_KNOWN_EPOCH)?.unwrap_or(0))
}

/// Advance the stored high-water epoch (only ever upward).
fn advance_known_epoch(conn: &Connection, epoch: i64) -> rusqlite::Result<()> {
    let cur = known_epoch(conn)?;
    if epoch > cur {
        crate::kv::set(conn, KV_KNOWN_EPOCH, &epoch)?;
    }
    Ok(())
}

/// Fence a server whose epoch is older than the highest we trust (§8.9): the caller
/// turns this into `EPOCH_OLD` and refuses the response, applying nothing.
fn fence(server_epoch: i64, known: i64) -> Result<(), TransportError> {
    if server_epoch < known {
        Err(TransportError::EpochOld)
    } else {
        Ok(())
    }
}

/// One sync cycle against a transport. Pushes the outbox, applies results, pulls.
/// Enforces epoch fencing: a response from a lower-epoch server is refused (§8.9).
pub async fn sync_once<T: Transport>(conn: &mut Connection, t: &T) -> Result<SyncOutcome, TransportError> {
    let mut outcome = SyncOutcome::default();
    let known = known_epoch(conn).map_err(|e| TransportError::Other(e.to_string()))?;
    let mut seen_epoch = known;
    let ops = outbox_ops(conn).map_err(|e| TransportError::Other(e.to_string()))?;

    if !ops.is_empty() {
        let resp = t.push(&ops).await?;
        // Refuse (and change nothing) if this server is older than the one we know.
        fence(resp.server_epoch, known)?;
        seen_epoch = seen_epoch.max(resp.server_epoch);
        outcome.pushed = resp.results.len();
        for res in &resp.results {
            match res.status {
                OpStatus::Confirmed => {
                    outcome.confirmed += 1;
                    if let (Some(rec), Some(op)) = (&res.record, ops.iter().find(|o| o.op_id == res.op_id)) {
                        upsert_canonical(conn, &op.table, rec).ok();
                    }
                }
                OpStatus::Conflict => outcome.conflicts += 1,
                OpStatus::Flagged => outcome.flagged += 1,
                OpStatus::Rejected => {
                    outcome.rejected += 1;
                    // Inbox item so the user can Edit & resend / Discard (§6).
                    conn.execute(
                        "INSERT INTO notification(id, staff_id, kind, title_key, vars_json, link) \
                         VALUES (?1, (SELECT staff_id FROM outbox WHERE op_id=?2), 'rejected', 'sync.rejected', ?3, 'inbox')",
                        params![format!("ntf-{}", res.op_id), res.op_id, serde_json::json!({ "reason": res.reason_code }).to_string()],
                    ).ok();
                }
            }
            // The server has recorded the op (idempotent) → drop it from the outbox.
            conn.execute("DELETE FROM outbox WHERE op_id=?1", params![res.op_id]).ok();
        }
    }

    // Pull canonical changes.
    match t.pull(cursor(conn).map_err(|e| TransportError::Other(e.to_string()))?, 500).await {
        Ok(pull) => {
            // Fence before applying anything from a lower-epoch server (§8.9).
            fence(pull.server_epoch, known)?;
            seen_epoch = seen_epoch.max(pull.server_epoch);
            outcome.pulled = apply_pull(conn, &pull).map_err(|e| TransportError::Other(e.to_string()))?;
        }
        Err(TransportError::Unreachable) => {
            // If we already pushed, a transient pull failure just means "retry the
            // pull later" — keep the confirmed push. But if we contacted the server
            // for NOTHING (empty outbox) and the pull is unreachable, this route is
            // down: report it so the engine falls through to the next route.
            if outcome.pushed == 0 {
                return Err(TransportError::Unreachable);
            }
        }
        Err(e) => return Err(e),
    }
    // Remember the highest epoch we trusted this cycle.
    advance_known_epoch(conn, seen_epoch).map_err(|e| TransportError::Other(e.to_string()))?;
    Ok(outcome)
}

/// A delivery route (docs §8.3). The engine tries them in order (LAN → relay →
/// queue), skipping one that is unreachable; the first that works is used and its
/// label is shown on the Sync screen (P05 Step 3).
pub enum Route {
    /// LAN: pinned-cert HTTPS to the school server ("On school Wi-Fi").
    Lan(HttpsTransport),
    /// Internet: sealed via the Vidya relay ("Over the internet").
    Relay(RelayTransport),
    /// In-process (tests / harness) — behaves as the LAN route.
    Loopback(LoopbackTransport),
}

impl Route {
    /// The Sync-screen label for this route.
    pub fn label(&self) -> &'static str {
        match self {
            Route::Lan(_) | Route::Loopback(_) => ROUTE_LAN,
            Route::Relay(_) => ROUTE_RELAY,
        }
    }
}

impl Transport for Route {
    async fn hello(&self) -> Result<crate::sync::protocol::HelloResp, TransportError> {
        match self {
            Route::Lan(t) => t.hello().await,
            Route::Relay(t) => t.hello().await,
            Route::Loopback(t) => t.hello().await,
        }
    }
    async fn push(&self, ops: &[Op]) -> Result<crate::sync::protocol::PushResp, TransportError> {
        match self {
            Route::Lan(t) => t.push(ops).await,
            Route::Relay(t) => t.push(ops).await,
            Route::Loopback(t) => t.push(ops).await,
        }
    }
    async fn pull(&self, since: i64, limit: i64) -> Result<PullResp, TransportError> {
        match self {
            Route::Lan(t) => t.pull(since, limit).await,
            Route::Relay(t) => t.pull(since, limit).await,
            Route::Loopback(t) => t.pull(since, limit).await,
        }
    }
    async fn heartbeat(&self, req: &crate::sync::protocol::HeartbeatReq) -> Result<crate::sync::protocol::HeartbeatResp, TransportError> {
        match self {
            Route::Lan(t) => t.heartbeat(req).await,
            Route::Relay(t) => t.heartbeat(req).await,
            Route::Loopback(t) => t.heartbeat(req).await,
        }
    }
}

/// Run one sync cycle over the route list in order (LAN → relay → queue). Skips a
/// route that is unreachable; returns the outcome + the label of the route that
/// worked (persisted for the Sync screen). All routes unreachable → `Unreachable`
/// (work stays queued — the Drive route arrives in Phase 6).
pub async fn sync_once_routed(conn: &mut Connection, routes: &[Route]) -> Result<(SyncOutcome, &'static str), TransportError> {
    for route in routes {
        match sync_once(conn, route).await {
            Ok(outcome) => {
                let label = route.label();
                crate::kv::set(conn, KV_LAST_ROUTE, &label).ok();
                return Ok((outcome, label));
            }
            Err(TransportError::Unreachable) => continue, // try the next route
            Err(e) => return Err(e), // a real error (revoked / epoch / protocol) — stop
        }
    }
    Err(TransportError::Unreachable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::service::DeviceAuth;
    use crate::sync::transport::LoopbackTransport;
    use crate::{db, seed};
    use std::sync::{Arc, Mutex};

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::parse("2026-09-23T12:00:00Z", &time::format_description::well_known::Rfc3339).unwrap()
    }

    fn client() -> Connection {
        let mut c = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut c).unwrap();
        c
    }

    #[test]
    fn backoff_curve() {
        assert_eq!(backoff_secs(0), 15);
        assert_eq!(backoff_secs(1), 30);
        assert_eq!(backoff_secs(2), 60);
        assert_eq!(backoff_secs(10), 300); // capped at 5 min
    }

    #[test]
    fn status_copy_is_exact() {
        assert_eq!(status_label(OpStatus::Confirmed), "Confirmed by school server");
    }

    #[tokio::test]
    async fn sync_once_pushes_outbox_and_confirms() {
        let mut server = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut server).unwrap();
        seed::seed_demo_school(&mut server, now()).unwrap();
        let server = Arc::new(Mutex::new(server));

        // A fresh client with a local student + an outbox op to insert it server-side.
        let mut cl = client();
        cl.execute("INSERT INTO student(id,name,status,created_at,updated_at,sync_state) VALUES ('stu-x','Local Kid','active','t','t','on_device')", []).unwrap();
        cl.execute(
            "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
             VALUES ('op-x','00000000000000000001x','dev-a1','stf-priya','admin','student','stu-x','insert', ?1, NULL, 1)",
            params![serde_json::json!({ "name": "Local Kid", "status": "active" }).to_string()],
        ).unwrap();

        let transport = LoopbackTransport::new(server.clone(), DeviceAuth { device_id: "dev-a1".into(), staff_id: "stf-priya".into() }, now());
        let outcome = sync_once(&mut cl, &transport).await.unwrap();
        assert_eq!(outcome.confirmed, 1);

        // Server received it; client outbox drained; local row confirmed.
        let on_server: i64 = server.lock().unwrap().query_row("SELECT COUNT(*) FROM student WHERE id='stu-x'", [], |r| r.get(0)).unwrap();
        assert_eq!(on_server, 1);
        let pending: i64 = cl.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0)).unwrap();
        assert_eq!(pending, 0);
        let sync_state: String = cl.query_row("SELECT sync_state FROM student WHERE id='stu-x'", [], |r| r.get(0)).unwrap();
        assert_eq!(sync_state, "confirmed");
    }

    #[tokio::test]
    async fn route_order_falls_through_to_the_next_reachable_route() {
        // P05 Step 3: LAN offline → relay/next route serves it; the working route's
        // label is returned and persisted.
        let mut server = db::open_in_memory(KEY).unwrap();
        db::run_migrations(&mut server).unwrap();
        seed::seed_demo_school(&mut server, now()).unwrap();
        let server = Arc::new(Mutex::new(server));
        let mut cl = client();
        cl.execute("INSERT INTO student(id,name,status,created_at,updated_at,sync_state) VALUES ('stu-r','Kid','active','t','t','on_device')", []).unwrap();
        cl.execute(
            "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
             VALUES ('op-r','00000000000000000001r','dev-a1','stf-priya','admin','student','stu-r','insert', ?1, NULL, 1)",
            params![serde_json::json!({ "name": "Kid", "status": "active" }).to_string()],
        ).unwrap();

        let auth = DeviceAuth { device_id: "dev-a1".into(), staff_id: "stf-priya".into() };
        let mut lan = LoopbackTransport::new(server.clone(), auth.clone(), now());
        lan.offline = true; // LAN down
        let online = LoopbackTransport::new(server.clone(), auth, now());
        let routes = vec![Route::Loopback(lan), Route::Loopback(online)];

        let (outcome, label) = sync_once_routed(&mut cl, &routes).await.unwrap();
        assert_eq!(outcome.confirmed, 1);
        assert_eq!(label, ROUTE_LAN);
        assert_eq!(crate::kv::get::<String>(&cl, KV_LAST_ROUTE).unwrap().as_deref(), Some(ROUTE_LAN));
    }

    #[tokio::test]
    async fn all_routes_unreachable_keeps_the_queue() {
        let server = Arc::new(Mutex::new({
            let mut s = db::open_in_memory(KEY).unwrap();
            db::run_migrations(&mut s).unwrap();
            seed::seed_demo_school(&mut s, now()).unwrap();
            s
        }));
        let mut cl = client();
        let auth = DeviceAuth { device_id: "dev-a1".into(), staff_id: "stf-priya".into() };
        let mut off = LoopbackTransport::new(server, auth, now());
        off.offline = true;
        let routes = vec![Route::Loopback(off)];
        assert_eq!(sync_once_routed(&mut cl, &routes).await, Err(TransportError::Unreachable));
    }

    #[test]
    fn route_labels_match_the_prompt_copy() {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let relay = RelayTransport::new("ws://127.0.0.1:1", "sch", "dev", &STANDARD.encode([9u8; 32]), 1, 0).unwrap();
        assert_eq!(Route::Relay(relay).label(), "Over the internet");
    }

    #[tokio::test]
    async fn a_lower_epoch_server_is_refused() {
        // §8.9 / DONE MEANS #5: once a device trusts epoch 2, an epoch-1 server's
        // response is refused and nothing is applied.
        let server = Arc::new(Mutex::new({
            let mut s = db::open_in_memory(KEY).unwrap();
            db::run_migrations(&mut s).unwrap();
            seed::seed_demo_school(&mut s, now()).unwrap(); // epoch 1
            s
        }));
        let mut cl = client();
        crate::kv::set(&cl, KV_KNOWN_EPOCH, &2i64).unwrap(); // we already know epoch 2
        let auth = DeviceAuth { device_id: "dev-a1".into(), staff_id: "stf-priya".into() };
        let transport = LoopbackTransport::new(server, auth, now());
        assert_eq!(sync_once(&mut cl, &transport).await, Err(TransportError::EpochOld));
    }

    #[tokio::test]
    async fn offline_keeps_the_outbox() {
        let server = Arc::new(Mutex::new({
            let mut s = db::open_in_memory(KEY).unwrap();
            db::run_migrations(&mut s).unwrap();
            seed::seed_demo_school(&mut s, now()).unwrap();
            s
        }));
        let mut cl = client();
        cl.execute("INSERT INTO student(id,name,status,created_at,updated_at,sync_state) VALUES ('stu-y','Kid','active','t','t','on_device')", []).unwrap();
        cl.execute(
            "INSERT INTO outbox(op_id,hlc,device_id,staff_id,audience,\"table\",record_id,kind,payload,base_version,server_epoch) \
             VALUES ('op-y','00000000000000000001y','dev-a1','stf-priya','admin','student','stu-y','insert','{}',NULL,1)",
            [],
        ).unwrap();
        let mut transport = LoopbackTransport::new(server, DeviceAuth { device_id: "dev-a1".into(), staff_id: "stf-priya".into() }, now());
        transport.offline = true;
        assert_eq!(sync_once(&mut cl, &transport).await, Err(TransportError::Unreachable));
        let pending: i64 = cl.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0)).unwrap();
        assert_eq!(pending, 1, "offline keeps the queued op");
    }
}
