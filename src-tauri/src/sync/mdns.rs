//! LAN discovery via mDNS (prompts/P04 Step 7) — desktop only (`mdns-sd`).
//! The server advertises `_vidya._tcp.local` with TXT `school_id`, `port`, `fp`
//! (first 16 hex of the cert fingerprint). Clients browse and filter by
//! `school_id` so two schools on one Wi-Fi don't cross. Runtime module.

#![cfg(not(target_os = "android"))]

use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

pub const SERVICE_TYPE: &str = "_vidya._tcp.local.";

/// A discovered school server on the LAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovered {
    pub school_id: String,
    pub addr: String,
    pub port: u16,
    pub fp16: String,
}

/// Advertise this server. Keep the returned daemon alive to stay advertised.
pub fn advertise(school_id: &str, port: u16, fingerprint: &str, ip: &str) -> Result<ServiceDaemon, String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let fp16: String = fingerprint.chars().take(16).collect();
    let instance = format!("vidya-{}", school_id.chars().take(8).collect::<String>());
    let host = format!("{instance}.local.");
    let props = [("school_id", school_id), ("port", &port.to_string()), ("fp", &fp16)];
    let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, ip, port, &props[..]).map_err(|e| e.to_string())?;
    daemon.register(info).map_err(|e| e.to_string())?;
    Ok(daemon)
}

/// Extract a `Discovered` for our school from a resolved service (filter by id).
fn to_discovered(info: &ServiceInfo, school_id: &str) -> Option<Discovered> {
    let sid = info.get_property("school_id")?.val_str().to_string();
    if sid != school_id {
        return None; // another school on the same Wi-Fi
    }
    let fp16 = info.get_property("fp").map(|p| p.val_str().to_string()).unwrap_or_default();
    let addr = info.get_addresses_v4().iter().next().map(|a| a.to_string())?;
    Some(Discovered { school_id: sid, addr, port: info.get_port(), fp16 })
}

/// Browse for our school for up to `timeout`, returning the first match.
pub fn discover(school_id: &str, timeout: Duration) -> Result<Option<Discovered>, String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let rx = daemon.browse(SERVICE_TYPE).map_err(|e| e.to_string())?;
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(d) = to_discovered(&info, school_id) {
                    let _ = daemon.shutdown();
                    return Ok(Some(d));
                }
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    Ok(None)
}
