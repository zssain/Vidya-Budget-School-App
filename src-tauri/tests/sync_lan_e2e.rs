//! LAN client join + authenticated sync over REAL pinned TLS (Phase B harness).
//!
//! The existing sync harness exercises the in-process service layer; this one starts
//! the ACTUAL axum server (`net::serve`) on localhost with a generated cert, then a
//! client joins via `sync::client::join_school` over HTTPS (pinned to the cert's
//! fingerprint) and authenticates a real `pull` with the returned device token. It
//! proves the client join + pinned transport work end-to-end — the piece that could
//! previously only be checked on two machines. (The real two-PC run is still the
//! owner's final check; this covers everything up to the physical network.)

use std::sync::Arc;

use time::OffsetDateTime;

use vidya_lib::db;
use vidya_lib::server::{cert, net, service};
use vidya_lib::sync::client;
use vidya_lib::sync::protocol::JoinPayload;
use vidya_lib::sync::transport::{HttpsTransport, Transport};

const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[tokio::test]
async fn client_joins_over_pinned_tls_then_authenticates_a_pull() {
    // ---- server: a seeded, migrated DB with one invited staff + a pending invite ----
    let mut server_db = db::open_in_memory(KEY).unwrap();
    db::run_migrations(&mut server_db).unwrap();
    server_db
        .execute(
            "INSERT INTO school(id,name,backup_salt,server_epoch,created_at,updated_at) \
             VALUES ('sch','Test School', x'00', 1, 't','t')",
            [],
        )
        .unwrap();
    server_db
        .execute(
            "INSERT INTO staff(id,name,role,created_at,updated_at) VALUES ('stf-1','Meena','teacher','t','t')",
            [],
        )
        .unwrap();
    // Create the invite against REAL time — the server validates expiry against the
    // real clock (net.rs `now()`), so a fixed past date would read as expired.
    let code = service::create_invite(&mut server_db, "stf-1", OffsetDateTime::now_utc()).unwrap();

    // ---- real TLS + a localhost listener; spawn the actual server ----
    let material = cert::generate_school_cert().unwrap();
    let fingerprint = material.fingerprint.clone();
    let tls = cert::server_config(&material.cert_der, &material.key_der).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let state = Arc::new(net::ServerState::new(server_db));
    let stop = Arc::new(tokio::sync::Notify::new());
    let server_task = {
        let stop = stop.clone();
        tokio::spawn(async move { net::serve(state, tls, listener, stop).await })
    };

    // ---- client: join over the pinned HTTPS using the invitation ----
    let client_db = db::open_in_memory(KEY).unwrap();
    {
        let mut c = client_db;
        db::run_migrations(&mut c).unwrap();
        let payload = JoinPayload {
            school_id: "sch".into(),
            school_name: "Test School".into(),
            lan_addrs: vec!["127.0.0.1".into()],
            port,
            cert_sha256: fingerprint.clone(),
            relay_url: None,
            code,
        };

        let identity = client::join_school(&c, &payload, "Test PC", "macos")
            .await
            .expect("join over pinned TLS succeeds");
        assert!(!identity.device_id.is_empty(), "device id provisioned");
        assert!(!identity.device_token.is_empty(), "device token issued");
        assert_eq!(identity.school_id, "sch");
        assert_eq!(identity.staff_id, "stf-1");
        assert!(client::is_client(&c), "device is now a client");

        // The join credentials authenticate a real pull over the same pinned TLS.
        let base = format!("https://127.0.0.1:{port}");
        let transport = HttpsTransport::new(&base, &identity.device_token, &fingerprint).unwrap();
        let pulled = transport.pull(0, 10).await;
        assert!(pulled.is_ok(), "authenticated pull over pinned TLS: {pulled:?}");
    }

    stop.notify_waiters();
    server_task.abort();
}
