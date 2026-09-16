use std::{collections::BTreeSet, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    error::{DomainError, ErrorKind},
    roles::{Actor, Role},
};

/// Every role-controlled action, in the same order as `docs/PERMISSIONS.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Action {
    StudentsView,
    StudentsViewFees,
    StudentsViewContact,
    StudentsAdd,
    StudentsEdit,
    StudentsSetConcession,
    StudentsMarkLeft,
    StudentsImport,
    StudentsExport,
    StudentsIssueTc,
    AttendanceView,
    AttendanceMarkToday,
    AttendanceEditPast,
    AttendancePrintRegister,
    MarksView,
    MarksEnter,
    ReportcardView,
    ReportcardPrint,
    FeesView,
    FeesCollect,
    FeesCancelReceipt,
    FeesDaybook,
    FeesExport,
    ReportsView,
    ReportsExport,
    UsersView,
    UsersManage,
    ActivityView,
    SettingsView,
    SettingsEdit,
    SessionChange,
    BackupManage,
    DevicesView,
    DevicesApprove,
    DevicesManage,
    AlertsView,
    LicenseView,
    AccountChangeOwnPassword,
    AccountSetOwnLanguage,
}

impl Action {
    /// All actions in documentation order.
    pub const ALL: &[Self] = &[
        Self::StudentsView,
        Self::StudentsViewFees,
        Self::StudentsViewContact,
        Self::StudentsAdd,
        Self::StudentsEdit,
        Self::StudentsSetConcession,
        Self::StudentsMarkLeft,
        Self::StudentsImport,
        Self::StudentsExport,
        Self::StudentsIssueTc,
        Self::AttendanceView,
        Self::AttendanceMarkToday,
        Self::AttendanceEditPast,
        Self::AttendancePrintRegister,
        Self::MarksView,
        Self::MarksEnter,
        Self::ReportcardView,
        Self::ReportcardPrint,
        Self::FeesView,
        Self::FeesCollect,
        Self::FeesCancelReceipt,
        Self::FeesDaybook,
        Self::FeesExport,
        Self::ReportsView,
        Self::ReportsExport,
        Self::UsersView,
        Self::UsersManage,
        Self::ActivityView,
        Self::SettingsView,
        Self::SettingsEdit,
        Self::SessionChange,
        Self::BackupManage,
        Self::DevicesView,
        Self::DevicesApprove,
        Self::DevicesManage,
        Self::AlertsView,
        Self::LicenseView,
        Self::AccountChangeOwnPassword,
        Self::AccountSetOwnLanguage,
    ];

    /// Returns the stable permission name used by commands and the UI.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StudentsView => "students.view",
            Self::StudentsViewFees => "students.view_fees",
            Self::StudentsViewContact => "students.view_contact",
            Self::StudentsAdd => "students.add",
            Self::StudentsEdit => "students.edit",
            Self::StudentsSetConcession => "students.set_concession",
            Self::StudentsMarkLeft => "students.mark_left",
            Self::StudentsImport => "students.import",
            Self::StudentsExport => "students.export",
            Self::StudentsIssueTc => "students.issue_tc",
            Self::AttendanceView => "attendance.view",
            Self::AttendanceMarkToday => "attendance.mark_today",
            Self::AttendanceEditPast => "attendance.edit_past",
            Self::AttendancePrintRegister => "attendance.print_register",
            Self::MarksView => "marks.view",
            Self::MarksEnter => "marks.enter",
            Self::ReportcardView => "reportcard.view",
            Self::ReportcardPrint => "reportcard.print",
            Self::FeesView => "fees.view",
            Self::FeesCollect => "fees.collect",
            Self::FeesCancelReceipt => "fees.cancel_receipt",
            Self::FeesDaybook => "fees.daybook",
            Self::FeesExport => "fees.export",
            Self::ReportsView => "reports.view",
            Self::ReportsExport => "reports.export",
            Self::UsersView => "users.view",
            Self::UsersManage => "users.manage",
            Self::ActivityView => "activity.view",
            Self::SettingsView => "settings.view",
            Self::SettingsEdit => "settings.edit",
            Self::SessionChange => "session.change",
            Self::BackupManage => "backup.manage",
            Self::DevicesView => "devices.view",
            Self::DevicesApprove => "devices.approve",
            Self::DevicesManage => "devices.manage",
            Self::AlertsView => "alerts.view",
            Self::LicenseView => "license.view",
            Self::AccountChangeOwnPassword => "account.change_own_password",
            Self::AccountSetOwnLanguage => "account.set_own_language",
        }
    }
}

impl FromStr for Action {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|action| action.as_str() == value)
            .ok_or_else(|| DomainError::new(ErrorKind::Internal, "permission.unknown_action"))
    }
}

/// A role's access level for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Yes,
    No,
    Own,
}

/// Returns the documented access level for one role/action pair.
pub const fn access(role: Role, action: Action) -> Access {
    match role {
        Role::Principal => match action {
            Action::StudentsView => Access::Yes,
            Action::StudentsViewFees => Access::Yes,
            Action::StudentsViewContact => Access::Yes,
            Action::StudentsAdd => Access::Yes,
            Action::StudentsEdit => Access::Yes,
            Action::StudentsSetConcession => Access::Yes,
            Action::StudentsMarkLeft => Access::Yes,
            Action::StudentsImport => Access::Yes,
            Action::StudentsExport => Access::Yes,
            Action::StudentsIssueTc => Access::Yes,
            Action::AttendanceView => Access::Yes,
            Action::AttendanceMarkToday => Access::Yes,
            Action::AttendanceEditPast => Access::Yes,
            Action::AttendancePrintRegister => Access::Yes,
            Action::MarksView => Access::Yes,
            Action::MarksEnter => Access::Yes,
            Action::ReportcardView => Access::Yes,
            Action::ReportcardPrint => Access::Yes,
            Action::FeesView => Access::Yes,
            Action::FeesCollect => Access::Yes,
            Action::FeesCancelReceipt => Access::Yes,
            Action::FeesDaybook => Access::Yes,
            Action::FeesExport => Access::Yes,
            Action::ReportsView => Access::Yes,
            Action::ReportsExport => Access::Yes,
            Action::UsersView => Access::Yes,
            Action::UsersManage => Access::Yes,
            Action::ActivityView => Access::Yes,
            Action::SettingsView => Access::Yes,
            Action::SettingsEdit => Access::Yes,
            Action::SessionChange => Access::Yes,
            Action::BackupManage => Access::Yes,
            Action::DevicesView => Access::Yes,
            Action::DevicesApprove => Access::Yes,
            Action::DevicesManage => Access::Yes,
            Action::AlertsView => Access::Yes,
            Action::LicenseView => Access::Yes,
            Action::AccountChangeOwnPassword => Access::Yes,
            Action::AccountSetOwnLanguage => Access::Yes,
        },
        Role::Accountant => match action {
            Action::StudentsView => Access::Yes,
            Action::StudentsViewFees => Access::Yes,
            Action::StudentsViewContact => Access::Yes,
            Action::StudentsAdd => Access::Yes,
            Action::StudentsEdit => Access::Yes,
            Action::StudentsSetConcession => Access::No,
            Action::StudentsMarkLeft => Access::Yes,
            Action::StudentsImport => Access::Yes,
            Action::StudentsExport => Access::Yes,
            Action::StudentsIssueTc => Access::No,
            Action::AttendanceView => Access::No,
            Action::AttendanceMarkToday => Access::No,
            Action::AttendanceEditPast => Access::No,
            Action::AttendancePrintRegister => Access::No,
            Action::MarksView => Access::No,
            Action::MarksEnter => Access::No,
            Action::ReportcardView => Access::No,
            Action::ReportcardPrint => Access::No,
            Action::FeesView => Access::Yes,
            Action::FeesCollect => Access::Yes,
            Action::FeesCancelReceipt => Access::No,
            Action::FeesDaybook => Access::Yes,
            Action::FeesExport => Access::Yes,
            Action::ReportsView => Access::No,
            Action::ReportsExport => Access::No,
            Action::UsersView => Access::No,
            Action::UsersManage => Access::No,
            Action::ActivityView => Access::No,
            Action::SettingsView => Access::No,
            Action::SettingsEdit => Access::No,
            Action::SessionChange => Access::No,
            Action::BackupManage => Access::No,
            Action::DevicesView => Access::No,
            Action::DevicesApprove => Access::No,
            Action::DevicesManage => Access::No,
            Action::AlertsView => Access::Yes,
            Action::LicenseView => Access::No,
            Action::AccountChangeOwnPassword => Access::Yes,
            Action::AccountSetOwnLanguage => Access::Yes,
        },
        Role::Teacher => match action {
            Action::StudentsView => Access::Own,
            Action::StudentsViewFees => Access::No,
            Action::StudentsViewContact => Access::Own,
            Action::StudentsAdd => Access::No,
            Action::StudentsEdit => Access::No,
            Action::StudentsSetConcession => Access::No,
            Action::StudentsMarkLeft => Access::No,
            Action::StudentsImport => Access::No,
            Action::StudentsExport => Access::No,
            Action::StudentsIssueTc => Access::No,
            Action::AttendanceView => Access::Own,
            Action::AttendanceMarkToday => Access::Own,
            Action::AttendanceEditPast => Access::No,
            Action::AttendancePrintRegister => Access::Own,
            Action::MarksView => Access::Own,
            Action::MarksEnter => Access::Own,
            Action::ReportcardView => Access::Own,
            Action::ReportcardPrint => Access::Own,
            Action::FeesView => Access::No,
            Action::FeesCollect => Access::No,
            Action::FeesCancelReceipt => Access::No,
            Action::FeesDaybook => Access::No,
            Action::FeesExport => Access::No,
            Action::ReportsView => Access::No,
            Action::ReportsExport => Access::No,
            Action::UsersView => Access::No,
            Action::UsersManage => Access::No,
            Action::ActivityView => Access::No,
            Action::SettingsView => Access::No,
            Action::SettingsEdit => Access::No,
            Action::SessionChange => Access::No,
            Action::BackupManage => Access::No,
            Action::DevicesView => Access::No,
            Action::DevicesApprove => Access::No,
            Action::DevicesManage => Access::No,
            Action::AlertsView => Access::No,
            Action::LicenseView => Access::No,
            Action::AccountChangeOwnPassword => Access::Yes,
            Action::AccountSetOwnLanguage => Access::Yes,
        },
    }
}

/// The row scope a service must apply to a permitted action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    All,
    Sections(BTreeSet<String>),
}

fn denied() -> DomainError {
    DomainError::new(ErrorKind::Permission, "permission.denied")
}

fn no_sections() -> DomainError {
    DomainError::new(ErrorKind::Permission, "permission.no_sections")
}

/// Returns whether an actor may act and the row scope services must enforce.
pub fn scope(actor: &Actor, action: Action) -> Result<Scope, DomainError> {
    match access(actor.role, action) {
        Access::Yes => Ok(Scope::All),
        Access::No => Err(denied()),
        Access::Own if actor.section_ids.is_empty() => Err(no_sections()),
        Access::Own => Ok(Scope::Sections(actor.section_ids.clone())),
    }
}

/// Authorizes an action against a specific section.
pub fn authorize_section(actor: &Actor, action: Action, section_id: &str) -> Result<(), DomainError> {
    match access(actor.role, action) {
        Access::Yes => Ok(()),
        Access::No => Err(denied()),
        Access::Own if actor.section_ids.is_empty() => Err(no_sections()),
        Access::Own if actor.section_ids.contains(section_id) => Ok(()),
        Access::Own => Err(denied()),
    }
}

/// Authorizes an action that does not require a section target.
pub fn authorize(actor: &Actor, action: Action) -> Result<(), DomainError> {
    match access(actor.role, action) {
        Access::Yes => Ok(()),
        Access::No => Err(denied()),
        Access::Own => {
            let error = DomainError::new(ErrorKind::Internal, "permission.needs_section");
            debug_assert_eq!(error.kind, ErrorKind::Internal);
            Err(error)
        }
    }
}

/// Lists every permission name visible to a role's UI.
pub fn permission_names(role: Role) -> Vec<&'static str> {
    Action::ALL
        .iter()
        .copied()
        .filter(|action| access(role, *action) != Access::No)
        .map(Action::as_str)
        .collect()
}
