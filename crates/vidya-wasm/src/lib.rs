//! Thin wasm-bindgen wrapper around `vidya-core` (Phase 19, Step 1).
//!
//! The iPhone PWA runs the SAME business rules, HLC ordering, sealing and Argon2id
//! as the Android/desktop app by calling `vidya-core` compiled to WebAssembly —
//! never a re-implemented rule. This module only marshals values across the JS↔WASM
//! boundary (primitives + JSON strings, so no `serde-wasm-bindgen` dependency).
//!
//! Data across the boundary:
//!   * `wall_ms` is passed as `f64` (a JS number is exact to 2^53; a millisecond
//!     wall clock ~1.8e12 is far below that) and cast to the `u64` vidya-core uses.
//!   * A validation result is `None` on OK, or `Some(code)` — the stable
//!     `CoreError` code string the UI already maps (e.g. `"p_or_a"` → VALIDATION).
//!
//! Decisions are added here as the Web backend needs them; sealing/unsealing and
//! Argon2id land once their pure logic is shared into `vidya-core` (Step 1b/1c).

use vidya_core::attendance;
use vidya_core::hlc::{self, Hlc};
use wasm_bindgen::prelude::*;

/// The vidya-core version compiled into the WASM — a smoke test that the module
/// loaded and that it matches the app version.
#[wasm_bindgen]
pub fn vidya_core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Validate a NEW attendance mark. `"P"`/`"A"` are accepted; `"L"` (legacy Leave)
/// is rejected exactly as on Android (`VALIDATION{rule:"p_or_a"}`). Returns `null`
/// when the mark is valid, or the stable error code string when it is rejected.
#[wasm_bindgen]
pub fn validate_new_mark(mark: &str) -> Option<String> {
    match attendance::parse_mark(mark) {
        Err(e) => Some(e.code().to_string()),
        Ok(m) => attendance::validate_new_mark(m).err().map(|e| e.code().to_string()),
    }
}

/// Attendance percentage in **tenths of a percent** (e.g. 913 = 91.3%), identical
/// to the native `percent_present` (legacy `L` counts in the denominator).
#[wasm_bindgen]
pub fn percent_present(p: u32, a: u32, l: u32) -> u32 {
    attendance::percent_present(p, a, l)
}

/// Build the canonical HLC string form from parts (for tests / a fixed clock).
#[wasm_bindgen]
pub fn hlc_new(wall_ms: f64, counter: u32, device_id: &str) -> String {
    Hlc::new(wall_ms as u64, counter as u16, device_id).to_string_form()
}

/// Advance the HLC for a new local event: `now(prev, wall_ms, device_id)`. `prev`
/// is the last HLC string this device produced (or `null` for the first). Returns
/// the new HLC string; the `Err(code)` is thrown to JS if `prev` is malformed.
/// (`Result<_, String>` rather than `JsError` so the same code is testable on the
/// host too — `JsError` cannot be constructed on a non-wasm target.)
#[wasm_bindgen]
pub fn hlc_next(prev: Option<String>, wall_ms: f64, device_id: &str) -> Result<String, String> {
    let prev_hlc = match prev {
        Some(s) => Some(Hlc::parse(&s).map_err(|e| e.code().to_string())?),
        None => None,
    };
    Ok(hlc::now(prev_hlc.as_ref(), wall_ms as u64, device_id).to_string_form())
}

/// Parse an HLC string into `{wall_ms, counter, device_id}` JSON; throws the error
/// code on a malformed HLC. (String compare on the canonical form is the total
/// order, so JS orders ops by plain string comparison — no wrapper needed for that.)
#[wasm_bindgen]
pub fn hlc_parse(s: &str) -> Result<String, String> {
    let h = Hlc::parse(s).map_err(|e| e.code().to_string())?;
    Ok(serde_json::json!({
        "wall_ms": h.wall_ms,
        "counter": h.counter,
        "device_id": h.device_id,
    })
    .to_string())
}

/// True when the device and server wall clocks differ enough to flag (same rule as
/// the native check-in clock-skew warning).
#[wasm_bindgen]
pub fn hlc_skew(local_wall_ms: f64, remote_wall_ms: f64) -> bool {
    hlc::skew_flag(local_wall_ms as u64, remote_wall_ms as u64)
}

// ---- Parity tests: the SAME vectors run natively (`cargo test`) and in WASM
// (`wasm-pack test --node`), proving the wrapper is faithful to vidya-core. ----
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn version_matches_crate() {
        assert_eq!(vidya_core_version(), env!("CARGO_PKG_VERSION"));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn attendance_mark_and_percent_match_core() {
        // P/A accepted; legacy L rejected with the same code as Android.
        assert_eq!(validate_new_mark("P"), None);
        assert_eq!(validate_new_mark("A"), None);
        assert_eq!(validate_new_mark("L").as_deref(), Some("VALIDATION"));
        assert_eq!(validate_new_mark("Z").as_deref(), Some("VALIDATION"));
        // Same vector as the vidya-core doctest: 612 present of 670 → 913 tenths.
        assert_eq!(percent_present(612, 58, 0), 913);
        assert_eq!(percent_present(0, 0, 0), 0);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn hlc_round_trips_and_orders() {
        let a = hlc_new(1_800_000_000_000.0, 0, "dev-a1");
        let b = hlc_new(1_800_000_000_000.0, 1, "dev-a1");
        assert!(a < b, "string compare is the total order");
        // parse round-trips the parts.
        let parsed = hlc_parse(&a).unwrap();
        assert!(parsed.contains("\"device_id\":\"dev-a1\""));
        assert!(parsed.contains("\"counter\":0"));
        // next() advances monotonically even when the wall clock does not move.
        let n1 = hlc_next(Some(a.clone()), 1_800_000_000_000.0, "dev-a1").unwrap();
        assert!(n1 > a, "counter advances when wall_ms is unchanged");
        // a malformed HLC throws.
        assert!(hlc_parse("not-an-hlc").is_err());
    }
}
