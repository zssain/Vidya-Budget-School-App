//! Vidya licence service — binary entry point (prompts/P10, docs §10).
//!
//! A SEPARATE binary — not a member of the app workspace, so it never ships inside
//! the desktop/Android apps. It runs the company side: the public website + manual
//! UPI purchase flow, the production licence API (activate/check/transfer), and the
//! company admin panel. TLS terminates at a reverse proxy in front of it.
//!
//! CLI:
//!   cargo run -- --dev            start locally (dev keys under .dev-keys/, http)
//!   cargo run -- serve            production (all secrets from env; see README)
//!   cargo run -- gen-code         dev: issue a licence + print an activation code
//!   cargo run -- print-key        print the signing public key (base64)
//!   cargo run -- migrate          create/upgrade the DB at $LICENCE_DB, then exit
//!   cargo run -- admin-add EMAIL  create/reset an admin ($LICENCE_ADMIN_PASSWORD)

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use vidya_licence::config::{Config, Mode};
use vidya_licence::state::AppState;
use vidya_licence::{admin, app_router, db, domain, keys};

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    match arg.as_str() {
        "--dev" | "dev" => run_server(Mode::Dev).await,
        "serve" => run_server(Mode::Serve).await,
        "gen-code" => gen_code(),
        "gen-prod-key" => gen_prod_key(),
        "gen-enc-key" => println!("{}", keys::generate_enc_key_b64()),
        "print-key" => print_key(),
        "migrate" => migrate_cli(),
        "admin-add" => admin_add_cli(),
        _ => {
            eprintln!(
                "usage:\n  cargo run -- --dev            start locally (dev keys, http)\n  \
                 cargo run -- serve            production (secrets from env)\n  \
                 cargo run -- gen-code         dev: issue a licence + print an activation code\n  \
                 cargo run -- gen-prod-key     print a NEW production signing keypair (private + public)\n  \
                 cargo run -- gen-enc-key      print a NEW base64 code-encryption key\n  \
                 cargo run -- print-key        print the signing public key (base64)\n  \
                 cargo run -- migrate          create/upgrade the DB at $LICENCE_DB\n  \
                 cargo run -- admin-add EMAIL  create/reset an admin ($LICENCE_ADMIN_PASSWORD)"
            );
            std::process::exit(2);
        }
    }
}

// --- server ----------------------------------------------------------------

async fn run_server(mode: Mode) {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let cfg = match Config::from_env(mode) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("configuration error: {e}");
            std::process::exit(2);
        }
    };
    let signing = match keys::load_signing_key(mode) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("signing key error: {e}");
            std::process::exit(2);
        }
    };

    let conn = db::open(&cfg.db_path).expect("open licence DB");
    if let (Some(email), Some(pw)) = (cfg.admin_email.clone(), cfg.admin_password.clone()) {
        let email = email.to_ascii_lowercase();
        admin::ensure_bootstrap_admin(&conn, &email, &pw).expect("bootstrap admin");
    }

    if mode == Mode::Dev {
        println!("Vidya licence service (dev mode)");
        println!("  website   http://{}/", cfg.bind);
        println!(
            "  admin     http://{}/admin  (login: {} / {})",
            cfg.bind,
            cfg.admin_email.as_deref().unwrap_or("-"),
            cfg.admin_password.as_deref().unwrap_or("-")
        );
        println!("  DB        {}", cfg.db_path.display());
        println!("  licence_public_key (paste into src-tauri/build-config/dev.json):");
        println!("    {}", keys::public_key_b64(&signing));
        println!("  issue a code with:  cargo run -- gen-code");
    } else {
        tracing::info!(bind = %cfg.bind, "vidya-licence serving");
    }

    let bind = cfg.bind.clone();
    let state = AppState::new(conn, signing, cfg);
    let app = app_router(state).layer(axum::middleware::from_fn(log_requests));

    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind licence service");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve licence service");
}

/// Structured access log — method, path, status, latency. No bodies, no query
/// strings, no cookies (Step 7: logs never carry secrets; codes only ever travel
/// in POST bodies, never in a path).
async fn log_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let resp = next.run(req).await;
    tracing::info!(
        method = %method,
        path = %path,
        status = resp.status().as_u16(),
        ms = start.elapsed().as_millis() as u64,
        "req"
    );
    resp
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}

// --- dev / ops CLI ---------------------------------------------------------

fn dev_config() -> Config {
    Config::from_env(Mode::Dev).expect("dev config")
}

/// Dev helper: create a paid order and issue a licence, then print the code so the
/// app's local activation (P03/P05) keeps working end-to-end.
fn gen_code() {
    let cfg = dev_config();
    let mut conn = db::open(&cfg.db_path).expect("open dev DB");
    let now = db::now_iso();
    let customer_id = db::new_id();
    let order_ref = format!("VIDYA-ORD-DEV{}", &db::new_id()[..6]);
    conn.execute(
        "INSERT INTO customer (id, email, name, phone, school_name, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![customer_id, "dev@vidya.local", "Dev School", "9000000000", "Dev School", now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO purchase_order (id, customer_id, provider, provider_order_id, amount_paise, currency, plan, status, created_at, updated_at)
         VALUES (?1,?2,'upi_manual',?3,0,'INR','perpetual','created',?4,?4)",
        rusqlite::params![db::new_id(), customer_id, order_ref, now],
    )
    .unwrap();
    match domain::verify_payment_and_issue(&mut conn, &cfg, "dev-cli", &order_ref, 0, None, "dev gen-code") {
        Ok(Ok(issue)) => {
            println!("{}", issue.code);
            eprintln!("(licence_id {} · order {})", issue.licence_id, order_ref);
        }
        other => {
            eprintln!("gen-code failed: {other:?}");
            std::process::exit(1);
        }
    }
}

/// Generate the production signing keypair for deployment (docs/DEPLOY-FLY.md §4).
fn gen_prod_key() {
    let (priv_b64, pub_b64) = keys::generate_seed_b64();
    println!("LICENCE_SIGNING_KEY (PRIVATE — set as a secret, e.g. `fly secrets set`, NEVER commit):");
    println!("  {priv_b64}");
    println!();
    println!("licence_public_key (paste into src-tauri/build-config/release.json):");
    println!("  {pub_b64}");
}

fn print_key() {
    let mode = if std::env::var("LICENCE_SIGNING_KEY").is_ok() { Mode::Serve } else { Mode::Dev };
    match keys::load_signing_key(mode) {
        Ok(s) => println!("{}", keys::public_key_b64(&s)),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

fn db_path_from_env() -> std::path::PathBuf {
    std::env::var("LICENCE_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".dev-keys/licence-dev.db"))
}

fn migrate_cli() {
    let path = db_path_from_env();
    db::open(&path).expect("open + migrate DB");
    println!("migrated: {}", path.display());
}

fn admin_add_cli() {
    let email = match std::env::args().nth(2) {
        Some(e) => e.to_ascii_lowercase(),
        None => {
            eprintln!("usage: admin-add EMAIL  (password from $LICENCE_ADMIN_PASSWORD)");
            std::process::exit(2);
        }
    };
    let password = std::env::var("LICENCE_ADMIN_PASSWORD").unwrap_or_default();
    if password.trim().is_empty() {
        eprintln!("set LICENCE_ADMIN_PASSWORD to the new password");
        std::process::exit(2);
    }
    let conn = db::open(&db_path_from_env()).expect("open DB");
    let now = db::now_iso();
    let hash = admin::hash_password(&password);
    let updated = conn
        .execute(
            "UPDATE admin_user SET password_hash = ?2, disabled = 0, failed_count = 0, locked_until = NULL WHERE email = ?1",
            rusqlite::params![email, hash],
        )
        .expect("update admin");
    if updated == 0 {
        conn.execute(
            "INSERT INTO admin_user (id, email, password_hash, role, disabled, created_at) VALUES (?1,?2,?3,'owner',0,?4)",
            rusqlite::params![db::new_id(), email, hash, now],
        )
        .expect("insert admin");
        println!("created admin {email}");
    } else {
        println!("reset password for {email}");
    }
}
