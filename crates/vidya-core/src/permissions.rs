//! permissions — role-based access control, encoding `docs/00-SYSTEM-CONTEXT.md`
//! §5 (Roles and permissions) exactly. Pure Rust: no IO, no async, no clock, no
//! randomness, no floats.
//!
//! The single entry point is [`can`]: given an [`Actor`] (a staff member with
//! their current role, state and class assignments), an [`Action`] they want to
//! perform, and a [`Target`] describing the thing acted upon, it returns a
//! [`Decision`]. The same function is called in every Tauri command, again on
//! the school server for every op (using the author's *current* permissions),
//! and when building every data scope (§5, §8.4).
//!
//! ## §5 summary (source of truth)
//!
//! - **Principal** (owner): everything is allowed. Approves / rejects / returns
//!   requests. Invites, suspends and removes staff and devices. Settings,
//!   licence, Google Drive, backups, restore, session rollover. *Direct edits to
//!   locked data are allowed but always audited with a reason.*
//! - **Accountant**: admissions (create students, enroll, transfer section);
//!   student-detail edits go through a **request**; fee dues view, record
//!   payments, print/share receipts; reversal goes through a **request**; day
//!   book, fee reports, student CSV import/export (no marks/attendance columns).
//!   No marks. No attendance. No staff management.
//! - **Teacher**: attendance for classes where they are class teacher; marks for
//!   their own class-subjects; view their students (no fee data at all; guardian
//!   address only if class teacher); report cards for their classes
//!   (view/print/share); corrections go through a **request**.
//! - **Everyone**: own profile, PIN, language, own requests, inbox, Sync screen.
//! - **Staff states**: `invited`, `active`, `suspended` (cannot sign in or sync;
//!   data kept), `removed`. A non-active staff member is denied every app action.
//!
//! ## Ambiguities resolved (most literal reading)
//!
//! 1. **Teacher "attendance duty"** — §5 marks this `[OWNER]` with default
//!    "class teacher only". We encode the default: attendance is allowed only for
//!    classes in `actor.class_teacher_of`.
//! 2. **Suspended / removed / invited** — §5 says suspended "cannot sign in or
//!    sync". Taking the strictest reading, *any* non-active state denies *every*
//!    app action here (an inactive staff member should not even reach the
//!    everyone-actions). The server also refuses to apply ops from
//!    revoked/suspended authors (§8.4).
//! 3. **Locked data for non-principals** — §5 says only the Principal may edit
//!    locked data directly (audited). For teachers, editing submitted attendance
//!    / marks becomes a correction *request*. For accountants (who cannot touch
//!    marks/attendance at all) editing locked marks/attendance is simply denied.
//! 4. **Guardian address for teachers** — allowed *only if the teacher is class
//!    teacher of that student's class*. We judge this from `target.class_id` (the
//!    student's current class) against `actor.class_teacher_of`. A teacher who is
//!    only a subject teacher of the class is denied.
//! 5. **Accountant fee reports / day book vs. marks reports** — §5 grants day
//!    book and fee reports to the accountant but denies marks/attendance. We
//!    treat report-card viewing as marks/attendance data → denied for the
//!    accountant.
//! 6. **Who may create requests** — §5 gives correction requests to teachers and
//!    accountants; the Principal decides them. We allow *any active staff* to view
//!    their own requests / inbox, and let each role raise the specific requests
//!    §5 assigns to it (student-details & reversal for the accountant; marks &
//!    attendance corrections for the teacher). The Principal never needs a
//!    request (direct edit, audited).

use crate::errors::CoreError;
use crate::types::{RequestType, Role, StaffState};
use serde::{Deserialize, Serialize};

/// A staff member as the permission check sees them: role + current state, plus
/// the class assignments that decide ownership for a teacher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// `staff.id` of the actor.
    pub staff_id: String,
    /// Current role (§5).
    pub role: Role,
    /// Current account state (§5); anything other than `Active` is denied.
    pub state: StaffState,
    /// `class.id`s for which this actor is the class teacher (attendance,
    /// guardian address, report cards).
    pub class_teacher_of: Vec<String>,
    /// `class_subject.id`s this actor teaches (marks entry).
    pub class_subjects: Vec<String>,
}

impl Actor {
    /// True if this actor is class teacher of the given class id.
    fn is_class_teacher_of(&self, class_id: &str) -> bool {
        self.class_teacher_of.iter().any(|c| c == class_id)
    }

    /// True if this actor teaches the given class-subject id.
    fn teaches_class_subject(&self, class_subject_id: &str) -> bool {
        self.class_subjects.iter().any(|c| c == class_subject_id)
    }
}

/// What kind of record a [`Target`] refers to (mirrors the §7 tables that
/// permissions care about).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A `student` (and their enrollment / details / guardian info).
    Student,
    /// An `attendance_sheet` / `attendance_mark`.
    Attendance,
    /// A `marks_sheet` / `mark_entry`.
    Marks,
    /// A `fee_due` / `payment` / `payment_allocation` / `reversal`.
    Fee,
    /// A `staff` / `device` / `invite` record (staff management).
    Staff,
    /// A `request` record (a correction / access change etc.).
    Request,
    /// School-wide configuration: `school`, `licence`, `academic_session`,
    /// backups, restore, Google Drive — Principal-only administration.
    School,
    /// The acting staff member's own profile / PIN / language, or the Sync and
    /// Inbox screens available to everyone.
    #[default]
    Own,
}

/// The thing an action is performed on. Only the fields relevant to the ownership
/// / lock checks in §5 are carried; a field left `None`/`false` means "not
/// applicable / not locked".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// What sort of record this is.
    pub kind: TargetKind,
    /// The `class.id` this target belongs to, when relevant (attendance sheet's
    /// class, a student's current class for guardian-address / report-card
    /// checks).
    pub class_id: Option<String>,
    /// The `class_subject.id` this target belongs to, when relevant (marks
    /// entry / exam subject).
    pub class_subject_id: Option<String>,
    /// True when the underlying sheet is *submitted* (locked): a submitted
    /// `attendance_sheet` or a submitted `marks_sheet` (§3 rule 6). Direct edits
    /// then need a reason (Principal) or a correction request (Teacher).
    pub is_locked: bool,
    /// True when the target belongs to the acting staff member themselves (their
    /// own profile, or a request they raised). Used for `Own` / `Request`.
    pub is_own: bool,
}

impl Target {
    /// A minimal target of the given kind (no class / lock context).
    pub fn of(kind: TargetKind) -> Target {
        Target { kind, ..Target::default() }
    }
}

/// Every capability derived from §5. Each variant documents which role(s) it is
/// for and the §5 sentence it encodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // ---- Admissions & students (Accountant + Principal) ----
    /// Create a new student record (admission). Accountant + Principal.
    CreateStudent,
    /// Enroll a student into a class for a session. Accountant + Principal.
    EnrollStudent,
    /// Transfer a student to another section (closes one enrollment, opens
    /// another). Accountant + Principal.
    TransferSection,
    /// Mark a student as left / record a leave. Accountant + Principal.
    MarkStudentLeft,
    /// Edit a student's details (name, guardian, address, …). Accountant → this
    /// is a `StudentDetails` **request**; Principal edits directly (audited if
    /// the record is locked).
    EditStudentDetails,
    /// View a student's basic profile. Accountant + Principal always; Teacher
    /// only for their own students.
    ViewStudent,
    /// View a student's guardian address. Teacher only if class teacher of that
    /// student's class; Accountant + Principal always.
    ViewGuardianAddress,
    /// Import students from CSV (no marks/attendance columns). Accountant +
    /// Principal.
    StudentCsvImport,
    /// Export students to CSV (no marks/attendance columns). Accountant +
    /// Principal.
    StudentCsvExport,

    // ---- Fees (Accountant + Principal; never Teacher) ----
    /// View fee dues / balances. Accountant + Principal. **Teacher: never.**
    ViewFees,
    /// Record a payment against a student's dues. Accountant + Principal.
    RecordPayment,
    /// Print or share a receipt. Accountant + Principal.
    PrintShareReceipt,
    /// Reverse a payment. Accountant → a `PaymentReversal` **request**; Principal
    /// approves / applies directly.
    PaymentReversal,
    /// View the day book (cash movement). Accountant + Principal.
    DayBook,
    /// View fee reports. Accountant + Principal.
    FeeReports,

    // ---- Attendance (Teacher for own classes + Principal) ----
    /// Take (create / fill / submit) an attendance sheet for a class. Teacher
    /// only if class teacher of `target.class_id`; Principal always.
    TakeAttendance,
    /// Edit a *submitted* (locked) attendance sheet. Teacher → an
    /// `AttendanceCorrection` **request**; Principal edits directly (audited).
    EditSubmittedAttendance,
    /// View attendance registers. Teacher for their own classes; Principal
    /// always. (Accountant: never — attendance data.)
    ViewAttendance,

    // ---- Marks (Teacher for own class-subjects + Principal) ----
    /// Enter marks for an exam subject. Teacher only for their own
    /// `class_subject_id`; Principal always.
    EnterMarks,
    /// Edit a *submitted* (locked) marks sheet. Teacher → a `MarksCorrection`
    /// **request**; Principal edits directly (audited).
    EditSubmittedMarks,
    /// View marks. Teacher for their own class-subjects; Principal always.
    /// (Accountant: never — marks data.)
    ViewMarks,
    /// View / print / share a report card. Teacher for their own classes;
    /// Principal always. (Accountant: never — marks/attendance data.)
    ViewReportCard,

    // ---- Staff & access (Principal only) ----
    /// Manage staff & access (umbrella). Principal only.
    ManageStaff,
    /// Invite a staff member. Principal only.
    InviteStaff,
    /// Suspend a staff member. Principal only.
    SuspendStaff,
    /// Remove a staff member. Principal only.
    RemoveStaff,
    /// Manage devices (register / revoke / replace). Principal only.
    ManageDevices,

    // ---- School administration (Principal only) ----
    /// Change school settings. Principal only.
    Settings,
    /// Manage the licence (activate / transfer / check). Principal only.
    Licence,
    /// Configure / connect Google Drive. Principal only.
    Drive,
    /// Run / configure backups. Principal only.
    Backups,
    /// Restore from a backup. Principal only.
    Restore,
    /// Roll the academic session over. Principal only.
    SessionRollover,

    // ---- Requests / approvals ----
    /// Approve / reject / return a correction request. Principal only.
    ApproveRequest,
    /// View one's own requests. Any active staff.
    ViewOwnRequests,

    // ---- Everyone (any active role) ----
    /// View the shared inbox / notifications. Any active staff.
    ViewInbox,
    /// View the Sync & devices screen. Any active staff.
    ViewSync,
    /// Edit one's own profile (name, mobile, …). Any active staff.
    EditOwnProfile,
    /// Change one's own PIN. Any active staff.
    ChangeOwnPin,
    /// Change the app language. Any active staff.
    ChangeLanguage,
}

/// The outcome of a permission check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    /// The action is permitted. `audit_reason_required` is true only for a
    /// Principal's direct edit of *locked* data (§5: "Direct edits to locked
    /// data are allowed but always audited with a reason").
    Allow { audit_reason_required: bool },
    /// The action is refused outright. `reason` is a stable, human-readable code
    /// (not user-facing copy).
    Deny { reason: String },
    /// The action is not permitted directly but may proceed as a correction /
    /// change **request** of the given type, decided by the Principal.
    NeedsRequest(RequestType),
}

impl Decision {
    /// Allow with no audit reason required — the common case.
    pub fn allow() -> Decision {
        Decision::Allow { audit_reason_required: false }
    }

    /// Allow, requiring an audit reason (Principal editing locked data).
    fn allow_with_reason() -> Decision {
        Decision::Allow { audit_reason_required: true }
    }

    /// Deny with the given stable reason code.
    fn deny(reason: impl Into<String>) -> Decision {
        Decision::Deny { reason: reason.into() }
    }

    /// True if this is any `Allow` variant.
    pub fn is_allow(&self) -> bool {
        matches!(self, Decision::Allow { .. })
    }

    /// Convert a denial into a [`CoreError::Forbidden`]; other variants map to
    /// `None` (the caller keeps handling Allow / NeedsRequest itself).
    pub fn as_error(&self) -> Option<CoreError> {
        match self {
            Decision::Deny { reason } => {
                Some(CoreError::Forbidden { reason: reason.clone() })
            }
            _ => None,
        }
    }
}

/// The universe of §5: decide whether `actor` may perform `action` on `target`.
///
/// See the module docs for the exact §5 rules and the ambiguities resolved.
pub fn can(actor: &Actor, action: Action, target: &Target) -> Decision {
    // §5: only an active staff member may do anything at all. Suspended cannot
    // sign in or sync; invited/removed likewise have no app access.
    if actor.state != StaffState::Active {
        return Decision::deny("inactive_staff");
    }

    // Actions available to *every* active staff member, regardless of role.
    if let Some(d) = everyone(action, target) {
        return d;
    }

    match actor.role {
        Role::Principal => principal(action, target),
        Role::Accountant => accountant(action, target),
        Role::Teacher => teacher(actor, action, target),
    }
}

/// Actions any active staff member may perform (§5 "Everyone"). Returns `None`
/// when the action is not an everyone-action, so role handlers can take over.
fn everyone(action: Action, _target: &Target) -> Option<Decision> {
    match action {
        Action::EditOwnProfile
        | Action::ChangeOwnPin
        | Action::ChangeLanguage
        | Action::ViewOwnRequests
        | Action::ViewInbox
        | Action::ViewSync => Some(Decision::allow()),
        _ => None,
    }
}

/// Principal: everything is allowed. Direct edits of *locked* data require an
/// audit reason (§5).
fn principal(action: Action, target: &Target) -> Decision {
    match action {
        // Editing locked (submitted) attendance / marks directly is allowed but
        // audited with a reason.
        Action::EditSubmittedAttendance | Action::EditSubmittedMarks => {
            Decision::allow_with_reason()
        }
        // Any other direct edit of a locked record is likewise audited.
        Action::EditStudentDetails if target.is_locked => Decision::allow_with_reason(),
        // Everything else the Principal may do plainly.
        _ => Decision::allow(),
    }
}

/// Accountant (§5): admissions, fees, day book / fee reports, student CSV.
/// Student-detail edits and reversals go through requests. No marks, no
/// attendance, no staff management.
fn accountant(action: Action, _target: &Target) -> Decision {
    match action {
        // Admissions.
        Action::CreateStudent
        | Action::EnrollStudent
        | Action::TransferSection
        | Action::MarkStudentLeft
        | Action::ViewStudent
        | Action::ViewGuardianAddress
        | Action::StudentCsvImport
        | Action::StudentCsvExport => Decision::allow(),

        // Student-detail edits go through a request.
        Action::EditStudentDetails => Decision::NeedsRequest(RequestType::StudentDetails),

        // Fees.
        Action::ViewFees
        | Action::RecordPayment
        | Action::PrintShareReceipt
        | Action::DayBook
        | Action::FeeReports => Decision::allow(),

        // A reversal goes through a request.
        Action::PaymentReversal => Decision::NeedsRequest(RequestType::PaymentReversal),

        // No marks, no attendance (including report cards, which are
        // marks/attendance data).
        Action::TakeAttendance
        | Action::EditSubmittedAttendance
        | Action::ViewAttendance
        | Action::EnterMarks
        | Action::EditSubmittedMarks
        | Action::ViewMarks
        | Action::ViewReportCard => Decision::deny("accountant_no_academic"),

        // No staff management / administration.
        Action::ManageStaff
        | Action::InviteStaff
        | Action::SuspendStaff
        | Action::RemoveStaff
        | Action::ManageDevices
        | Action::Settings
        | Action::Licence
        | Action::Drive
        | Action::Backups
        | Action::Restore
        | Action::SessionRollover
        | Action::ApproveRequest => Decision::deny("accountant_no_admin"),

        // Everyone-actions are handled before we get here.
        Action::EditOwnProfile
        | Action::ChangeOwnPin
        | Action::ChangeLanguage
        | Action::ViewOwnRequests
        | Action::ViewInbox
        | Action::ViewSync => Decision::allow(),
    }
}

/// Teacher (§5): attendance for classes they class-teach; marks for their own
/// class-subjects; view their students (no fees; guardian address only if class
/// teacher); report cards for their classes; corrections → request.
fn teacher(actor: &Actor, action: Action, target: &Target) -> Decision {
    match action {
        // ---- Attendance: only for classes they are class teacher of. ----
        Action::TakeAttendance | Action::ViewAttendance => {
            if class_owned(actor, target) {
                Decision::allow()
            } else {
                Decision::deny("teacher_not_class_teacher")
            }
        }
        // Editing a *submitted* attendance sheet is a correction request — but
        // only for a class they class-teach; otherwise denied outright.
        Action::EditSubmittedAttendance => {
            if !class_owned(actor, target) {
                Decision::deny("teacher_not_class_teacher")
            } else if target.is_locked {
                Decision::NeedsRequest(RequestType::AttendanceCorrection)
            } else {
                // A not-yet-submitted sheet is edited directly via TakeAttendance.
                Decision::allow()
            }
        }

        // ---- Marks: only for their own class-subjects. ----
        Action::EnterMarks | Action::ViewMarks => {
            if class_subject_owned(actor, target) {
                Decision::allow()
            } else {
                Decision::deny("teacher_not_subject_teacher")
            }
        }
        // Editing a *submitted* marks sheet is a correction request — only for
        // their own class-subject; otherwise denied outright.
        Action::EditSubmittedMarks => {
            if !class_subject_owned(actor, target) {
                Decision::deny("teacher_not_subject_teacher")
            } else if target.is_locked {
                Decision::NeedsRequest(RequestType::MarksCorrection)
            } else {
                Decision::allow()
            }
        }

        // ---- Students: view their own; guardian address only if class teacher. ----
        Action::ViewStudent => {
            if class_owned(actor, target) {
                Decision::allow()
            } else {
                Decision::deny("teacher_not_own_student")
            }
        }
        Action::ViewGuardianAddress => {
            if class_owned(actor, target) {
                Decision::allow()
            } else {
                Decision::deny("teacher_guardian_class_teacher_only")
            }
        }

        // Report cards for their classes.
        Action::ViewReportCard => {
            if class_owned(actor, target) {
                Decision::allow()
            } else {
                Decision::deny("teacher_not_own_class")
            }
        }

        // ---- No fees at all (§5: "no fee data at all"). ----
        Action::ViewFees
        | Action::RecordPayment
        | Action::PrintShareReceipt
        | Action::PaymentReversal
        | Action::DayBook
        | Action::FeeReports => Decision::deny("teacher_no_fees"),

        // ---- No admissions / student management. ----
        Action::CreateStudent
        | Action::EnrollStudent
        | Action::TransferSection
        | Action::MarkStudentLeft
        | Action::EditStudentDetails
        | Action::StudentCsvImport
        | Action::StudentCsvExport => Decision::deny("teacher_no_admissions"),

        // ---- No staff management / administration / approvals. ----
        Action::ManageStaff
        | Action::InviteStaff
        | Action::SuspendStaff
        | Action::RemoveStaff
        | Action::ManageDevices
        | Action::Settings
        | Action::Licence
        | Action::Drive
        | Action::Backups
        | Action::Restore
        | Action::SessionRollover
        | Action::ApproveRequest => Decision::deny("teacher_no_admin"),

        // Everyone-actions are handled before we get here.
        Action::EditOwnProfile
        | Action::ChangeOwnPin
        | Action::ChangeLanguage
        | Action::ViewOwnRequests
        | Action::ViewInbox
        | Action::ViewSync => Decision::allow(),
    }
}

/// True if the target's class is one this actor class-teaches. Absent
/// `class_id` (no ownership context) is treated as not owned.
fn class_owned(actor: &Actor, target: &Target) -> bool {
    match &target.class_id {
        Some(class_id) => actor.is_class_teacher_of(class_id),
        None => false,
    }
}

/// True if the target's class-subject is one this actor teaches. Absent
/// `class_subject_id` is treated as not owned.
fn class_subject_owned(actor: &Actor, target: &Target) -> bool {
    match &target.class_subject_id {
        Some(cs) => actor.teaches_class_subject(cs),
        None => false,
    }
}

// ======================================================= roles as data =======
//
// P13: the role × action matrix is represented as data — `role` +
// `role_permission(role, action, effect allow|request)` — so future role
// templates can be added without code. `can` remains the authoritative decision
// point (the exhaustive matrix test still asserts §5 over it); the data is DERIVED
// from `can` (`default_permissions`), so it can never drift, and a loaded set can
// be read back with `effect_of`. A missing (role, action) entry means deny.

impl Action {
    /// Every action variant (for the matrix-as-data seed and exhaustive checks).
    pub const ALL: [Action; 40] = [
        Action::CreateStudent, Action::EnrollStudent, Action::TransferSection, Action::MarkStudentLeft,
        Action::EditStudentDetails, Action::ViewStudent, Action::ViewGuardianAddress,
        Action::StudentCsvImport, Action::StudentCsvExport,
        Action::ViewFees, Action::RecordPayment, Action::PrintShareReceipt, Action::PaymentReversal,
        Action::DayBook, Action::FeeReports,
        Action::TakeAttendance, Action::EditSubmittedAttendance, Action::ViewAttendance,
        Action::EnterMarks, Action::EditSubmittedMarks, Action::ViewMarks, Action::ViewReportCard,
        Action::ManageStaff, Action::InviteStaff, Action::SuspendStaff, Action::RemoveStaff,
        Action::ManageDevices, Action::Settings, Action::Licence, Action::Drive, Action::Backups,
        Action::Restore, Action::SessionRollover, Action::ApproveRequest,
        Action::ViewOwnRequests, Action::ViewInbox, Action::ViewSync,
        Action::EditOwnProfile, Action::ChangeOwnPin, Action::ChangeLanguage,
    ];

    /// The stable string stored in `role_permission.action` (serde snake_case).
    pub fn as_key(&self) -> String {
        serde_json::to_value(self).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
    }

    /// Parse a `role_permission.action` string back into an `Action`.
    pub fn from_key(s: &str) -> Option<Action> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

/// The effect a role has for an action in the matrix (`role_permission.effect`).
/// `Deny` is represented by the ABSENCE of a row, not a value here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Allow,
    Request,
}

impl Effect {
    pub fn as_key(&self) -> &'static str {
        match self {
            Effect::Allow => "allow",
            Effect::Request => "request",
        }
    }
    pub fn from_key(s: &str) -> Option<Effect> {
        match s {
            "allow" => Some(Effect::Allow),
            "request" => Some(Effect::Request),
            _ => None,
        }
    }
}

/// The natural target kind for an action (used to derive the coarse matrix).
fn target_kind_for(action: Action) -> TargetKind {
    use Action::*;
    match action {
        CreateStudent | EnrollStudent | TransferSection | MarkStudentLeft | EditStudentDetails
        | ViewStudent | ViewGuardianAddress | StudentCsvImport | StudentCsvExport => TargetKind::Student,
        ViewFees | RecordPayment | PrintShareReceipt | PaymentReversal | DayBook | FeeReports => TargetKind::Fee,
        TakeAttendance | EditSubmittedAttendance | ViewAttendance => TargetKind::Attendance,
        EnterMarks | EditSubmittedMarks | ViewMarks | ViewReportCard => TargetKind::Marks,
        ManageStaff | InviteStaff | SuspendStaff | RemoveStaff | ManageDevices => TargetKind::Staff,
        Settings | Licence | Drive | Backups | Restore | SessionRollover => TargetKind::School,
        ApproveRequest => TargetKind::Request,
        ViewOwnRequests | ViewInbox | ViewSync | EditOwnProfile | ChangeOwnPin | ChangeLanguage => TargetKind::Own,
    }
}

/// An active actor of `role` with a class assignment, for deriving capability.
fn representative_actor(role: Role) -> Actor {
    Actor {
        staff_id: "seed".into(),
        role,
        state: StaffState::Active,
        class_teacher_of: vec!["c1".into()],
        class_subjects: vec!["cs1".into()],
    }
}

/// A favourable, unlocked target that the actor would own — so `can` returns the
/// role's *capability* (allow / request) rather than a target-specific deny.
fn representative_target(action: Action) -> Target {
    Target {
        kind: target_kind_for(action),
        class_id: Some("c1".to_string()),
        class_subject_id: Some("cs1".to_string()),
        is_locked: false,
        is_own: true,
    }
}

/// The built-in role × action matrix as data — every `(role, action)` a role can
/// perform, with its effect (allow / request). Derived from `can`, so the seed can
/// never disagree with the code. Denials are omitted. Seeds `role_permission`.
pub fn default_permissions() -> Vec<(Role, Action, Effect)> {
    let mut out = Vec::new();
    for role in [Role::Principal, Role::Accountant, Role::Teacher] {
        let actor = representative_actor(role);
        for action in Action::ALL {
            match can(&actor, action, &representative_target(action)) {
                Decision::Allow { .. } => out.push((role, action, Effect::Allow)),
                Decision::NeedsRequest(_) => out.push((role, action, Effect::Request)),
                Decision::Deny { .. } => {}
            }
        }
    }
    out
}

/// Read a loaded permission set (as loaded from `role_permission`): the effect for
/// `(role, action)`, or `None` (deny) if absent. Pure — the caller supplies the set.
pub fn effect_of(set: &[(Role, Action, Effect)], role: Role, action: Action) -> Option<Effect> {
    set.iter().find(|(r, a, _)| *r == role && *a == action).map(|(_, _, e)| *e)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Test fixtures ---------------------------------------------------

    const OWNED_CLASS: &str = "class-owned";
    const OTHER_CLASS: &str = "class-other";
    const OWNED_CS: &str = "cs-owned";
    const OTHER_CS: &str = "cs-other";

    fn principal() -> Actor {
        Actor {
            staff_id: "p1".into(),
            role: Role::Principal,
            state: StaffState::Active,
            class_teacher_of: vec![],
            class_subjects: vec![],
        }
    }

    fn accountant() -> Actor {
        Actor {
            staff_id: "a1".into(),
            role: Role::Accountant,
            state: StaffState::Active,
            class_teacher_of: vec![],
            class_subjects: vec![],
        }
    }

    fn teacher() -> Actor {
        Actor {
            staff_id: "t1".into(),
            role: Role::Teacher,
            state: StaffState::Active,
            class_teacher_of: vec![OWNED_CLASS.into()],
            class_subjects: vec![OWNED_CS.into()],
        }
    }

    /// Target for the actor's *own* class / class-subject (locked or not).
    fn own_target(kind: TargetKind, is_locked: bool) -> Target {
        Target {
            kind,
            class_id: Some(OWNED_CLASS.into()),
            class_subject_id: Some(OWNED_CS.into()),
            is_locked,
            is_own: true,
        }
    }

    /// Target for a class / class-subject the actor does NOT own.
    fn other_target(kind: TargetKind, is_locked: bool) -> Target {
        Target {
            kind,
            class_id: Some(OTHER_CLASS.into()),
            class_subject_id: Some(OTHER_CS.into()),
            is_locked,
            is_own: false,
        }
    }

    fn needs(rt: RequestType) -> Decision {
        Decision::NeedsRequest(rt)
    }

    fn allow() -> Decision {
        Decision::allow()
    }

    fn allow_reason() -> Decision {
        Decision::Allow { audit_reason_required: true }
    }

    fn is_deny(d: &Decision) -> bool {
        matches!(d, Decision::Deny { .. })
    }

    /// Every action variant, for the exhaustive table.
    const ALL_ACTIONS: &[Action] = &[
        Action::CreateStudent,
        Action::EnrollStudent,
        Action::TransferSection,
        Action::MarkStudentLeft,
        Action::EditStudentDetails,
        Action::ViewStudent,
        Action::ViewGuardianAddress,
        Action::StudentCsvImport,
        Action::StudentCsvExport,
        Action::ViewFees,
        Action::RecordPayment,
        Action::PrintShareReceipt,
        Action::PaymentReversal,
        Action::DayBook,
        Action::FeeReports,
        Action::TakeAttendance,
        Action::EditSubmittedAttendance,
        Action::ViewAttendance,
        Action::EnterMarks,
        Action::EditSubmittedMarks,
        Action::ViewMarks,
        Action::ViewReportCard,
        Action::ManageStaff,
        Action::InviteStaff,
        Action::SuspendStaff,
        Action::RemoveStaff,
        Action::ManageDevices,
        Action::Settings,
        Action::Licence,
        Action::Drive,
        Action::Backups,
        Action::Restore,
        Action::SessionRollover,
        Action::ApproveRequest,
        Action::ViewOwnRequests,
        Action::ViewInbox,
        Action::ViewSync,
        Action::EditOwnProfile,
        Action::ChangeOwnPin,
        Action::ChangeLanguage,
    ];

    const EVERYONE_ACTIONS: &[Action] = &[
        Action::ViewOwnRequests,
        Action::ViewInbox,
        Action::ViewSync,
        Action::EditOwnProfile,
        Action::ChangeOwnPin,
        Action::ChangeLanguage,
    ];

    fn is_everyone(a: Action) -> bool {
        EVERYONE_ACTIONS.contains(&a)
    }

    // ---- Named §5 cases --------------------------------------------------

    #[test]
    fn suspended_denied_everything() {
        for state in [StaffState::Suspended, StaffState::Removed, StaffState::Invited] {
            for role in [Role::Principal, Role::Accountant, Role::Teacher] {
                let actor = Actor {
                    staff_id: "x".into(),
                    role,
                    state,
                    class_teacher_of: vec![OWNED_CLASS.into()],
                    class_subjects: vec![OWNED_CS.into()],
                };
                for &action in ALL_ACTIONS {
                    let d = can(&actor, action, &own_target(TargetKind::Own, false));
                    assert!(
                        is_deny(&d),
                        "non-active {role:?}/{state:?} must be denied {action:?}, got {d:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn principal_allows_everything() {
        let p = principal();
        for &action in ALL_ACTIONS {
            let d = can(&p, action, &other_target(TargetKind::School, false));
            assert!(
                d.is_allow(),
                "principal must be allowed {action:?}, got {d:?}"
            );
        }
    }

    #[test]
    fn principal_locked_edit_needs_reason() {
        let p = principal();
        assert_eq!(
            can(&p, Action::EditSubmittedAttendance, &own_target(TargetKind::Attendance, true)),
            allow_reason()
        );
        assert_eq!(
            can(&p, Action::EditSubmittedMarks, &own_target(TargetKind::Marks, true)),
            allow_reason()
        );
        // A locked student-details edit is also audited.
        let mut t = own_target(TargetKind::Student, true);
        t.kind = TargetKind::Student;
        assert_eq!(can(&p, Action::EditStudentDetails, &t), allow_reason());
        // An unlocked student-details edit is a plain allow.
        assert_eq!(
            can(&p, Action::EditStudentDetails, &own_target(TargetKind::Student, false)),
            allow()
        );
    }

    #[test]
    fn principal_approves_requests() {
        let p = principal();
        assert!(can(&p, Action::ApproveRequest, &Target::of(TargetKind::Request)).is_allow());
    }

    #[test]
    fn accountant_admissions_allowed() {
        let a = accountant();
        for action in [
            Action::CreateStudent,
            Action::EnrollStudent,
            Action::TransferSection,
            Action::MarkStudentLeft,
        ] {
            assert_eq!(
                can(&a, action, &Target::of(TargetKind::Student)),
                allow(),
                "accountant should be allowed {action:?}"
            );
        }
    }

    #[test]
    fn accountant_student_details_needs_request() {
        let a = accountant();
        assert_eq!(
            can(&a, Action::EditStudentDetails, &Target::of(TargetKind::Student)),
            needs(RequestType::StudentDetails)
        );
    }

    #[test]
    fn accountant_fees_allowed() {
        let a = accountant();
        for action in [
            Action::ViewFees,
            Action::RecordPayment,
            Action::PrintShareReceipt,
            Action::DayBook,
            Action::FeeReports,
        ] {
            assert_eq!(
                can(&a, action, &Target::of(TargetKind::Fee)),
                allow(),
                "accountant should be allowed {action:?}"
            );
        }
    }

    #[test]
    fn accountant_reversal_needs_request() {
        let a = accountant();
        assert_eq!(
            can(&a, Action::PaymentReversal, &Target::of(TargetKind::Fee)),
            needs(RequestType::PaymentReversal)
        );
    }

    #[test]
    fn accountant_csv_allowed() {
        let a = accountant();
        assert_eq!(
            can(&a, Action::StudentCsvImport, &Target::of(TargetKind::Student)),
            allow()
        );
        assert_eq!(
            can(&a, Action::StudentCsvExport, &Target::of(TargetKind::Student)),
            allow()
        );
    }

    #[test]
    fn accountant_no_marks_attendance_staff() {
        let a = accountant();
        for action in [
            Action::TakeAttendance,
            Action::EditSubmittedAttendance,
            Action::ViewAttendance,
            Action::EnterMarks,
            Action::EditSubmittedMarks,
            Action::ViewMarks,
            Action::ViewReportCard,
            Action::ManageStaff,
            Action::InviteStaff,
            Action::SuspendStaff,
            Action::RemoveStaff,
            Action::ManageDevices,
            Action::Settings,
            Action::Licence,
            Action::Drive,
            Action::Backups,
            Action::Restore,
            Action::SessionRollover,
            Action::ApproveRequest,
        ] {
            let d = can(&a, action, &own_target(TargetKind::Marks, false));
            assert!(is_deny(&d), "accountant must be denied {action:?}, got {d:?}");
        }
    }

    #[test]
    fn teacher_attendance_only_own_class() {
        let t = teacher();
        // Own class → allow.
        assert_eq!(
            can(&t, Action::TakeAttendance, &own_target(TargetKind::Attendance, false)),
            allow()
        );
        // Other class → deny.
        assert!(is_deny(&can(
            &t,
            Action::TakeAttendance,
            &other_target(TargetKind::Attendance, false)
        )));
    }

    #[test]
    fn teacher_marks_only_own_class_subject() {
        let t = teacher();
        assert_eq!(
            can(&t, Action::EnterMarks, &own_target(TargetKind::Marks, false)),
            allow()
        );
        assert!(is_deny(&can(
            &t,
            Action::EnterMarks,
            &other_target(TargetKind::Marks, false)
        )));
    }

    #[test]
    fn teacher_submitted_marks_needs_request() {
        let t = teacher();
        assert_eq!(
            can(&t, Action::EditSubmittedMarks, &own_target(TargetKind::Marks, true)),
            needs(RequestType::MarksCorrection)
        );
        // But not for a subject they don't teach.
        assert!(is_deny(&can(
            &t,
            Action::EditSubmittedMarks,
            &other_target(TargetKind::Marks, true)
        )));
    }

    #[test]
    fn teacher_submitted_attendance_needs_request() {
        let t = teacher();
        assert_eq!(
            can(&t, Action::EditSubmittedAttendance, &own_target(TargetKind::Attendance, true)),
            needs(RequestType::AttendanceCorrection)
        );
        assert!(is_deny(&can(
            &t,
            Action::EditSubmittedAttendance,
            &other_target(TargetKind::Attendance, true)
        )));
    }

    #[test]
    fn teacher_no_fees_at_all() {
        let t = teacher();
        for action in [
            Action::ViewFees,
            Action::RecordPayment,
            Action::PrintShareReceipt,
            Action::PaymentReversal,
            Action::DayBook,
            Action::FeeReports,
        ] {
            let d = can(&t, action, &own_target(TargetKind::Fee, false));
            assert!(is_deny(&d), "teacher must be denied fee action {action:?}, got {d:?}");
        }
    }

    #[test]
    fn teacher_guardian_address_only_class_teacher() {
        let t = teacher();
        // Class teacher of the student's class → allow.
        assert_eq!(
            can(&t, Action::ViewGuardianAddress, &own_target(TargetKind::Student, false)),
            allow()
        );
        // Not class teacher of that class → deny.
        assert!(is_deny(&can(
            &t,
            Action::ViewGuardianAddress,
            &other_target(TargetKind::Student, false)
        )));
    }

    #[test]
    fn teacher_report_card_own_class() {
        let t = teacher();
        assert_eq!(
            can(&t, Action::ViewReportCard, &own_target(TargetKind::Marks, false)),
            allow()
        );
        assert!(is_deny(&can(
            &t,
            Action::ViewReportCard,
            &other_target(TargetKind::Marks, false)
        )));
    }

    #[test]
    fn everyone_active_can_do_everyone_actions() {
        for actor in [principal(), accountant(), teacher()] {
            for &action in EVERYONE_ACTIONS {
                assert_eq!(
                    can(&actor, action, &Target::of(TargetKind::Own)),
                    allow(),
                    "{:?} should be allowed everyone-action {action:?}",
                    actor.role
                );
            }
        }
    }

    // ---- Exhaustive table: every Action × every Role × (own vs not-own) ---

    /// Expected decision for (role, action, own?) — the single source of truth
    /// for the exhaustive sweep. Returns the exact `Decision`.
    fn expected(role: Role, action: Action, own: bool) -> Decision {
        // Everyone-actions: always allow for an active actor.
        if is_everyone(action) {
            return allow();
        }
        match role {
            Role::Principal => match action {
                Action::EditSubmittedAttendance | Action::EditSubmittedMarks => allow_reason(),
                // In this sweep a "locked" flag is only set on the own+locked
                // path used below; principal student-details edit stays plain
                // allow because the sweep targets are unlocked.
                _ => allow(),
            },
            Role::Accountant => match action {
                Action::CreateStudent
                | Action::EnrollStudent
                | Action::TransferSection
                | Action::MarkStudentLeft
                | Action::ViewStudent
                | Action::ViewGuardianAddress
                | Action::StudentCsvImport
                | Action::StudentCsvExport
                | Action::ViewFees
                | Action::RecordPayment
                | Action::PrintShareReceipt
                | Action::DayBook
                | Action::FeeReports => allow(),
                Action::EditStudentDetails => needs(RequestType::StudentDetails),
                Action::PaymentReversal => needs(RequestType::PaymentReversal),
                _ => Decision::deny("_deny_"),
            },
            Role::Teacher => match action {
                Action::TakeAttendance
                | Action::ViewAttendance
                | Action::EnterMarks
                | Action::ViewMarks
                | Action::ViewStudent
                | Action::ViewGuardianAddress
                | Action::ViewReportCard => {
                    if own {
                        allow()
                    } else {
                        Decision::deny("_deny_")
                    }
                }
                // Unlocked "edit submitted" on an owned class/subject falls
                // through to a plain allow; not-owned is deny.
                Action::EditSubmittedAttendance | Action::EditSubmittedMarks => {
                    if own {
                        allow()
                    } else {
                        Decision::deny("_deny_")
                    }
                }
                _ => Decision::deny("_deny_"),
            },
        }
    }

    #[test]
    fn exhaustive_table() {
        let mut count = 0usize;
        for role in [Role::Principal, Role::Accountant, Role::Teacher] {
            for &action in ALL_ACTIONS {
                for own in [true, false] {
                    let actor = match role {
                        Role::Principal => principal(),
                        Role::Accountant => accountant(),
                        Role::Teacher => teacher(),
                    };
                    // Sweep uses UNLOCKED targets so the "edit submitted" cases
                    // resolve to their unlocked behaviour (plain allow / deny);
                    // the locked → request / audit paths have dedicated tests.
                    let target = if own {
                        own_target(TargetKind::Student, false)
                    } else {
                        other_target(TargetKind::Student, false)
                    };
                    let got = can(&actor, action, &target);
                    let want = expected(role, action, own);
                    match want {
                        Decision::Deny { .. } => assert!(
                            is_deny(&got),
                            "{role:?} {action:?} own={own}: expected Deny, got {got:?}"
                        ),
                        other => assert_eq!(
                            got, other,
                            "{role:?} {action:?} own={own}: expected {other:?}, got {got:?}"
                        ),
                    }
                    count += 1;
                }
            }
        }
        // 3 roles × 40 actions × 2 ownerships.
        assert_eq!(count, 3 * ALL_ACTIONS.len() * 2);
        assert_eq!(count, 240);
    }

    #[test]
    fn decision_helpers() {
        assert_eq!(Decision::allow(), Decision::Allow { audit_reason_required: false });
        assert!(Decision::allow().is_allow());
        assert!(allow_reason().is_allow());
        let deny = Decision::deny("x");
        assert!(!deny.is_allow());
        assert_eq!(
            deny.as_error(),
            Some(CoreError::Forbidden { reason: "x".into() })
        );
        assert_eq!(Decision::allow().as_error(), None);
        assert_eq!(needs(RequestType::MarksCorrection).as_error(), None);
    }

    #[test]
    fn target_constructors() {
        let t = Target::of(TargetKind::Fee);
        assert_eq!(t.kind, TargetKind::Fee);
        assert_eq!(t.class_id, None);
        assert!(!t.is_locked);
        assert_eq!(Target::default().kind, TargetKind::Own);
    }

    #[test]
    fn serde_roundtrip() {
        let d = Decision::NeedsRequest(RequestType::PaymentReversal);
        let j = serde_json::to_string(&d).unwrap();
        let back: Decision = serde_json::from_str(&j).unwrap();
        assert_eq!(d, back);

        let a = Action::RecordPayment;
        let j = serde_json::to_string(&a).unwrap();
        assert_eq!(j, "\"record_payment\"");
    }

    // ---- roles as data (P13) --------------------------------------------

    #[test]
    fn action_all_covers_every_variant_and_keys_round_trip() {
        assert_eq!(Action::ALL.len(), 40);
        for a in Action::ALL {
            assert_eq!(Action::from_key(&a.as_key()), Some(a), "{a:?}");
        }
        assert_eq!(Action::from_key("nope"), None);
        assert_eq!(Effect::from_key(Effect::Allow.as_key()), Some(Effect::Allow));
    }

    #[test]
    fn default_permissions_are_consistent_with_can_and_readable() {
        let set = default_permissions();
        for role in [Role::Principal, Role::Accountant, Role::Teacher] {
            let actor = representative_actor(role);
            for action in Action::ALL {
                let expected = match can(&actor, action, &representative_target(action)) {
                    Decision::Allow { .. } => Some(Effect::Allow),
                    Decision::NeedsRequest(_) => Some(Effect::Request),
                    Decision::Deny { .. } => None,
                };
                assert_eq!(effect_of(&set, role, action), expected, "{role:?} {action:?}");
            }
        }
    }

    #[test]
    fn matrix_shape_matches_section_5() {
        let set = default_permissions();
        // Principal can do every action (allow).
        for action in Action::ALL {
            assert_eq!(effect_of(&set, Role::Principal, action), Some(Effect::Allow), "principal {action:?}");
        }
        // Accountant: fees yes, marks/attendance no; student-detail edits via request.
        assert_eq!(effect_of(&set, Role::Accountant, Action::ViewFees), Some(Effect::Allow));
        assert_eq!(effect_of(&set, Role::Accountant, Action::ViewMarks), None);
        assert_eq!(effect_of(&set, Role::Accountant, Action::EditStudentDetails), Some(Effect::Request));
        // Teacher: attendance yes (own class), fees never. EditSubmitted* is the
        // base capability (allow); the correction-request layer is lock-triggered
        // inside `can`, not a role-level policy, so it isn't in the coarse matrix.
        assert_eq!(effect_of(&set, Role::Teacher, Action::TakeAttendance), Some(Effect::Allow));
        assert_eq!(effect_of(&set, Role::Teacher, Action::ViewFees), None);
        assert_eq!(effect_of(&set, Role::Teacher, Action::EnterMarks), Some(Effect::Allow));
        // Everyone: own profile.
        for role in [Role::Principal, Role::Accountant, Role::Teacher] {
            assert_eq!(effect_of(&set, role, Action::EditOwnProfile), Some(Effect::Allow));
        }
    }
}
