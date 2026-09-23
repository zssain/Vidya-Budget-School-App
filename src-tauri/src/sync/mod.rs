//! Sync: wire protocol, role scopes, op application, client engine (prompts/P04).
pub mod apply;
pub mod engine;
#[cfg(not(target_os = "android"))]
pub mod mdns;
pub mod protocol;
pub mod relay;
pub mod scope;
pub mod seal;
pub mod transport;
