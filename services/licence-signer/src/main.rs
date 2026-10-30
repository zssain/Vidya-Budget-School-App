//! `licence-signer` — the isolated Vidya licence signer worker. No HTTP server.
//!
//! USAGE:
//!   licence-signer                 run the worker loop (needs the env vars below)
//!   licence-signer fingerprint     print the public key + fingerprint of LICENCE_SIGNING_KEY_FILE
//!   licence-signer fingerprint <public.key>
//!                                  print the fingerprint of a licence-maker public.key file
//!
//! Env: LICENCE_SIGNING_KEY_FILE, LICENCE_SIGNING_KEY_ID,
//! EXPECTED_PUBLIC_KEY_FINGERPRINT, SIGNER_DATABASE_URL,
//! [SIGNER_POLL_INTERVAL_SECS=5], [SIGNER_WORKER_ID].

use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use licence_signer::key::{fingerprint, read_key_file, read_public_key_file, Signer};
use licence_signer::worker::{process_one, recover_expired_leases, Outcome};
use licence_signer::Config;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Notify;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("run") if args.len() <= 1 => run_worker(),
        Some("fingerprint") if args.len() <= 2 => print_fingerprint(args.get(1).map(String::as_str)),
        _ => {
            eprintln!("usage: licence-signer [run] | licence-signer fingerprint [public.key]");
            ExitCode::from(2)
        }
    }
}

fn print_fingerprint(public_file: Option<&str>) -> ExitCode {
    let public = match public_file {
        Some(p) => read_public_key_file(Path::new(p)),
        None => match std::env::var("LICENCE_SIGNING_KEY_FILE") {
            Ok(p) => read_key_file(Path::new(&p)).map(|k| k.verifying_key().to_bytes()),
            Err(_) => {
                eprintln!("error: LICENCE_SIGNING_KEY_FILE is not set (or pass a public.key path)");
                return ExitCode::FAILURE;
            }
        },
    };
    match public {
        Ok(pk) => {
            println!("public_key_base64: {}", STANDARD.encode(pk));
            println!("fingerprint:       {}", fingerprint(&pk));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_worker() -> ExitCode {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Config and key are validated BEFORE any network/DB activity: a wrong key
    // never gets as far as claiming a job.
    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("configuration error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let signer = match Signer::load(&cfg.key_file, &cfg.key_id, &cfg.expected_fingerprints) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("{e}");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(key_id = signer.key_id(), fingerprint = signer.fingerprint(), worker_id = %cfg.worker_id, "signing key loaded and matches the expected fingerprint");

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("cannot start runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    rt.block_on(worker_loop(cfg, signer))
}

async fn worker_loop(cfg: Config, signer: Signer) -> ExitCode {
    let pool = match PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&cfg.database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            // Do not echo the error: it can include parts of the connection string.
            tracing::error!("cannot connect to SIGNER_DATABASE_URL");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(
        "connected; polling for issue_licence jobs every {:?}",
        cfg.poll_interval
    );

    // SIGINT / SIGTERM: finish the job in hand, then exit (never mid-transaction).
    let stop = Arc::new(AtomicBool::new(false));
    let wake = Arc::new(Notify::new());
    {
        let (stop, wake) = (stop.clone(), wake.clone());
        tokio::spawn(async move {
            shutdown_signal().await;
            stop.store(true, Ordering::SeqCst);
            wake.notify_one();
        });
    }
    while !stop.load(Ordering::SeqCst) {
        match recover_expired_leases(&pool).await {
            Ok(0) => {}
            Ok(n) => tracing::warn!(recovered = n, "returned abandoned job leases to the queue"),
            Err(e) => tracing::warn!(error = e.message(), "lease recovery failed"),
        }
        // Drain ready jobs, then sleep.
        while !stop.load(Ordering::SeqCst) {
            match process_one(&pool, &signer, &cfg.worker_id).await {
                Ok(Outcome::Idle) => break,
                Ok(_) => continue,
                Err(e) => {
                    tracing::warn!(
                        error = e.message(),
                        "job processing error; will retry after the poll interval"
                    );
                    break;
                }
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(cfg.poll_interval) => {}
            _ = wake.notified() => {}
        }
    }
    tracing::info!("shutdown signal received; exiting");
    pool.close().await;
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
