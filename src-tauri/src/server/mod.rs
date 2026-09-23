//! School server: TLS cert + pinned verifier (cert), request service layer
//! (service), and the axum/tokio-rustls listener (net) — prompts/P04 Step 2/3.
pub mod cert;
pub mod service;
