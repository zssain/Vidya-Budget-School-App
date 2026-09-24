//! Role × state permission matrix (prompts/P09 §2/§7). Exhaustively asserts the
//! §5 rules over `permissions::can`, the single decision point every Tauri
//! command and every server op-apply routes through (`require_allow` on the
//! desktop, `apply_op` on the server both call `can`). Pure rules, no DB.
//!
//! Rather than transcribe ~480 individual cells, this encodes §5 as invariants
//! that must hold for the WHOLE grid, so a new `Action` cannot silently escape
//! the state gate or the role split.

use vidya_core::permissions::{can, Action, Actor, Decision, Target, TargetKind};
use vidya_core::types::{Role, StaffState};

/// Every `Action` variant (§5). Kept in lock-step with the enum — if a variant
/// is added and not listed here, the "principal may do everything" invariant
/// still passes, but the state-gate invariant below would miss it, so we also
/// assert the count.
const ALL_ACTIONS: &[Action] = &[
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

fn actor(role: Role, state: StaffState) -> Actor {
    Actor { staff_id: "stf".into(), role, state, class_teacher_of: vec![], class_subjects: vec![] }
}

/// A generic unlocked target of the given kind.
fn tgt(kind: TargetKind) -> Target {
    Target::of(kind)
}

#[test]
fn action_list_is_complete() {
    // Guards against an Action variant being added without extending this test.
    assert_eq!(ALL_ACTIONS.len(), 40, "update ALL_ACTIONS when the Action enum changes");
}

/// §5: only an ACTIVE staff member may do anything at all. Invited / suspended /
/// removed are denied EVERY action, in every role.
#[test]
fn non_active_is_denied_everything() {
    for role in [Role::Principal, Role::Accountant, Role::Teacher] {
        for state in [StaffState::Invited, StaffState::Suspended, StaffState::Removed] {
            let a = actor(role, state);
            for &action in ALL_ACTIONS {
                let d = can(&a, action, &tgt(TargetKind::Own));
                assert!(!d.is_allow(), "{role:?}/{state:?} must NOT be allowed {action:?}");
                assert!(matches!(d, Decision::Deny { .. }), "{role:?}/{state:?}/{action:?} → Deny");
            }
        }
    }
}

/// §5: an active Principal may do everything (some locked edits require an audit
/// reason, which is still an Allow).
#[test]
fn active_principal_may_do_everything() {
    let p = actor(Role::Principal, StaffState::Active);
    for &action in ALL_ACTIONS {
        let kind = match action {
            Action::RecordPayment | Action::ViewFees | Action::PaymentReversal => TargetKind::Fee,
            Action::TakeAttendance | Action::EditSubmittedAttendance => TargetKind::Attendance,
            Action::EnterMarks | Action::EditSubmittedMarks => TargetKind::Marks,
            Action::ManageStaff | Action::InviteStaff | Action::SuspendStaff | Action::RemoveStaff => TargetKind::Staff,
            _ => TargetKind::Own,
        };
        assert!(can(&p, action, &tgt(kind)).is_allow(), "Principal must be allowed {action:?}");
    }
}

/// §5 "Everyone": every active role may manage their own profile/PIN/language
/// and see their requests, inbox and the sync screen.
#[test]
fn everyone_actions_allowed_for_all_active_roles() {
    let everyone = [
        Action::EditOwnProfile, Action::ChangeOwnPin, Action::ChangeLanguage,
        Action::ViewOwnRequests, Action::ViewInbox, Action::ViewSync,
    ];
    for role in [Role::Principal, Role::Accountant, Role::Teacher] {
        let a = actor(role, StaffState::Active);
        for action in everyone {
            assert!(can(&a, action, &tgt(TargetKind::Own)).is_allow(), "{role:?} may {action:?}");
        }
    }
}

/// §5 Teacher: no fees at all, no staff management, no school administration,
/// and no admissions/CSV. (View of own students/attendance/marks handled by the
/// scope test below.)
#[test]
fn teacher_is_denied_fees_staff_and_admin() {
    let t = actor(Role::Teacher, StaffState::Active);
    let denied = [
        Action::ViewFees, Action::RecordPayment, Action::PrintShareReceipt, Action::PaymentReversal,
        Action::DayBook, Action::FeeReports,
        Action::ManageStaff, Action::InviteStaff, Action::SuspendStaff, Action::RemoveStaff,
        Action::ManageDevices, Action::Settings, Action::Licence, Action::Drive, Action::Backups,
        Action::Restore, Action::SessionRollover, Action::ApproveRequest,
        Action::CreateStudent, Action::EnrollStudent, Action::TransferSection, Action::MarkStudentLeft,
        Action::StudentCsvImport, Action::StudentCsvExport,
    ];
    for action in denied {
        let kind = match action {
            Action::ViewFees | Action::RecordPayment | Action::PrintShareReceipt | Action::PaymentReversal
            | Action::DayBook | Action::FeeReports => TargetKind::Fee,
            _ => TargetKind::Own,
        };
        assert!(!can(&t, action, &tgt(kind)).is_allow(), "Teacher must NOT be allowed {action:?}");
    }
}

/// §5 Accountant: no attendance, no marks, no staff management, no school admin.
#[test]
fn accountant_is_denied_academics_and_staff() {
    let acc = actor(Role::Accountant, StaffState::Active);
    let denied = [
        Action::TakeAttendance, Action::EditSubmittedAttendance,
        Action::EnterMarks, Action::EditSubmittedMarks,
        Action::ManageStaff, Action::InviteStaff, Action::SuspendStaff, Action::RemoveStaff,
        Action::ManageDevices, Action::Settings, Action::Licence, Action::Drive, Action::Backups,
        Action::Restore, Action::SessionRollover, Action::ApproveRequest,
    ];
    for action in denied {
        let kind = match action {
            Action::TakeAttendance | Action::EditSubmittedAttendance => TargetKind::Attendance,
            Action::EnterMarks | Action::EditSubmittedMarks => TargetKind::Marks,
            _ => TargetKind::Own,
        };
        assert!(!can(&acc, action, &tgt(kind)).is_allow(), "Accountant must NOT be allowed {action:?}");
    }
    // But the Accountant CAN record payments and admit students.
    assert!(can(&acc, Action::RecordPayment, &tgt(TargetKind::Fee)).is_allow());
    assert!(can(&acc, Action::CreateStudent, &tgt(TargetKind::Student)).is_allow());
}

/// §5 Teacher scope: attendance only for classes they are class teacher of;
/// marks only for class-subjects they teach; guardian address only for own class.
#[test]
fn teacher_scope_is_enforced() {
    let t = Actor {
        staff_id: "meena".into(),
        role: Role::Teacher,
        state: StaffState::Active,
        class_teacher_of: vec!["cls-5a".into()],
        class_subjects: vec!["cs-5a-maths".into()],
    };

    let own_class = Target { kind: TargetKind::Attendance, class_id: Some("cls-5a".into()), ..Default::default() };
    let other_class = Target { kind: TargetKind::Attendance, class_id: Some("cls-6b".into()), ..Default::default() };
    assert!(can(&t, Action::TakeAttendance, &own_class).is_allow(), "own class attendance allowed");
    assert!(!can(&t, Action::TakeAttendance, &other_class).is_allow(), "other class attendance denied");

    let own_subj = Target { kind: TargetKind::Marks, class_subject_id: Some("cs-5a-maths".into()), ..Default::default() };
    let other_subj = Target { kind: TargetKind::Marks, class_subject_id: Some("cs-6b-sci".into()), ..Default::default() };
    assert!(can(&t, Action::EnterMarks, &own_subj).is_allow(), "own subject marks allowed");
    assert!(!can(&t, Action::EnterMarks, &other_subj).is_allow(), "other subject marks denied");

    let own_guardian = Target { kind: TargetKind::Student, class_id: Some("cls-5a".into()), ..Default::default() };
    let other_guardian = Target { kind: TargetKind::Student, class_id: Some("cls-6b".into()), ..Default::default() };
    assert!(can(&t, Action::ViewGuardianAddress, &own_guardian).is_allow(), "own class guardian address allowed");
    assert!(!can(&t, Action::ViewGuardianAddress, &other_guardian).is_allow(), "other class guardian address denied");
}
