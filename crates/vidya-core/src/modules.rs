//! modules — module switches (foundation item 8, 00-SYSTEM-CONTEXT §14).
//!
//! Pure mapping only. `module_for(action)` says which [`Module`] an action
//! belongs to (Core by default); `require_module` / `require_enabled` reject an
//! action whose module is switched off with `MODULE_OFF{module}`. The *enabled
//! set* (the `module_setting` keys that are ON) is loaded by `src-tauri` from the
//! DB and passed in — vidya-core stays IO-free.
//!
//! In Phase 11 every built action maps to [`Module::Core`] (always on), so no
//! existing command is ever blocked; P14–P17 remap their new actions to their
//! module as those features are built. The mapping is **exhaustive** over
//! [`Action`], so adding a new action forces a module decision here.

use std::collections::BTreeSet;

use crate::errors::{CoreError, CoreResult};
use crate::permissions::Action;

/// A switchable feature area (00-SYSTEM-CONTEXT §14). `Core` is always on and has
/// no `module_setting` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Module {
    Core,
    Accounts,
    Classroom,
    Hr,
    Circulars,
    WaAuto,
    Store,
    InstantSync,
}

impl Module {
    /// The stable `module_setting.key`. `Core` returns `"core"` but is never
    /// stored (it is always on).
    pub fn key(self) -> &'static str {
        match self {
            Module::Core => "core",
            Module::Accounts => "accounts",
            Module::Classroom => "classroom",
            Module::Hr => "hr",
            Module::Circulars => "circulars",
            Module::WaAuto => "wa_auto",
            Module::Store => "store",
            Module::InstantSync => "instant_sync",
        }
    }

    /// Core is always on; every other module is gated by the enabled set.
    pub fn always_on(self) -> bool {
        matches!(self, Module::Core)
    }
}

/// The keys that appear in `module_setting` (everything except Core), in the
/// order the migration seeds them. §11 gives the on/off defaults:
/// `accounts, classroom, hr, circulars` on · `wa_auto, store, instant_sync` off.
pub const TOGGLEABLE_KEYS: &[&str] =
    &["accounts", "classroom", "hr", "circulars", "wa_auto", "store", "instant_sync"];

/// Which module an action belongs to (Core by default). Exhaustive over
/// [`Action`] so a newly added action must be classified here.
pub fn module_for(action: Action) -> Module {
    use Action::*;
    match action {
        // Every action built through Phase 11 is Core (always on). Future phases
        // move their new actions to Accounts / Classroom / Hr / Circulars / Store.
        CreateStudent | EnrollStudent | TransferSection | MarkStudentLeft
        | EditStudentDetails | ViewStudent | ViewGuardianAddress | StudentCsvImport
        | StudentCsvExport | ViewFees | RecordPayment | PrintShareReceipt
        | PaymentReversal | DayBook | FeeReports | TakeAttendance
        | EditSubmittedAttendance | ViewAttendance | EnterMarks | EditSubmittedMarks
        | ViewMarks | ViewReportCard | ManageStaff | InviteStaff | SuspendStaff
        | RemoveStaff | ManageDevices | Settings | Licence | Drive | Backups
        | Restore | SessionRollover | ApproveRequest | ViewOwnRequests | ViewInbox
        | ViewSync | EditOwnProfile | ChangeOwnPin | ChangeLanguage => Module::Core,
    }
}

/// Reject `module` when it is switched off. `enabled` holds the keys of the
/// modules that are ON (Core is implicit). Off → `MODULE_OFF{module}`.
pub fn require_enabled(enabled: &BTreeSet<String>, module: Module) -> CoreResult<()> {
    if module.always_on() || enabled.contains(module.key()) {
        Ok(())
    } else {
        Err(CoreError::ModuleOff { module: module.key().to_string() })
    }
}

/// Reject `action` when its module is switched off (see [`module_for`]).
pub fn require_module(enabled: &BTreeSet<String>, action: Action) -> CoreResult<()> {
    require_enabled(enabled, module_for(action))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn every_action_maps_to_a_module() {
        // Exhaustiveness of `module_for` is enforced by the compiler; in Phase 11
        // every action is Core, so `require_module` never blocks with an empty set.
        let empty = set(&[]);
        for a in [
            Action::RecordPayment,
            Action::TakeAttendance,
            Action::EnterMarks,
            Action::DayBook,
            Action::ManageStaff,
            Action::Settings,
            Action::ChangeLanguage,
        ] {
            assert_eq!(module_for(a), Module::Core);
            assert!(require_module(&empty, a).is_ok(), "core action never blocked");
        }
    }

    #[test]
    fn core_is_always_on() {
        assert!(Module::Core.always_on());
        assert!(require_enabled(&set(&[]), Module::Core).is_ok());
    }

    #[test]
    fn disabled_module_is_rejected() {
        // A toggle-able module NOT in the enabled set → MODULE_OFF{key}.
        assert_eq!(
            require_enabled(&set(&[]), Module::Store),
            Err(CoreError::ModuleOff { module: "store".into() })
        );
        assert_eq!(
            require_enabled(&set(&["accounts"]), Module::InstantSync),
            Err(CoreError::ModuleOff { module: "instant_sync".into() })
        );
    }

    #[test]
    fn enabled_module_is_allowed() {
        assert!(require_enabled(&set(&["store"]), Module::Store).is_ok());
        assert!(require_enabled(&set(&["instant_sync"]), Module::InstantSync).is_ok());
    }

    #[test]
    fn keys_are_stable_and_complete() {
        assert_eq!(
            TOGGLEABLE_KEYS,
            &["accounts", "classroom", "hr", "circulars", "wa_auto", "store", "instant_sync"]
        );
        // Core has a key but never appears in the toggle-able set.
        assert!(!TOGGLEABLE_KEYS.contains(&Module::Core.key()));
    }
}
