//! The reusable permission-matrix harness. Every service method is exercised
//! against each role/origin and checked against `access()` and the
//! office-computer-only rule. Later prompts add their actions' cases and remove
//! them from `NOT_YET_IMPLEMENTED`.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::permissions::{access, Access, Action};
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::Db;
use vidya_services::auth::{AuthService, SessionStore};
use vidya_services::env::{FixedClock, SeededRandom, SeqIds};
use vidya_services::error::ServiceError;
use vidya_services::services::users::{CreateUserInput, UpdateUserInput, UserService};
use vidya_services::{Mode, Services};
use vidya_testkit::SampleSchool;

/// Actions whose permission case is added by a later prompt (with its id).
const NOT_YET_IMPLEMENTED: &[Action] = &[
    // P3.2 students
    Action::StudentsView,
    Action::StudentsViewFees,
    Action::StudentsViewContact,
    Action::StudentsAdd,
    Action::StudentsEdit,
    Action::StudentsSetConcession,
    Action::StudentsMarkLeft,
    // P3.3 fees / alerts
    Action::FeesView,
    Action::FeesCollect,
    Action::FeesCancelReceipt,
    Action::FeesDaybook,
    Action::AlertsView,
    // P3.4 attendance
    Action::AttendanceView,
    Action::AttendanceMarkToday,
    Action::AttendanceEditPast,
    Action::AttendancePrintRegister,
    // P3.5 marks / report cards
    Action::MarksView,
    Action::MarksEnter,
    Action::ReportcardView,
    Action::ReportcardPrint,
    // P3.6 settings
    Action::SettingsView,
    Action::SettingsEdit,
    // P3.7 reports / activity / exports
    Action::ReportsView,
    Action::ReportsExport,
    Action::ActivityView,
    Action::StudentsExport,
    Action::FeesExport,
    // P4.2 license
    Action::LicenseView,
    // P4.5 import
    Action::StudentsImport,
    // P5.2 transfer certificate
    Action::StudentsIssueTc,
    // P6.1 backups
    Action::BackupManage,
    Action::BackupRun,
    // P6.3 session change
    Action::SessionChange,
    // P7.3 devices
    Action::DevicesView,
    Action::DevicesApprove,
    Action::DevicesManage,
];

struct Harness {
    services: Services,
    sessions: SessionStore,
    principal: Actor,
    accountant: Actor,
    teacher_own: Actor,
    teacher_other: Actor,
    principal_phone: Actor,
    accountant_phone: Actor,
    va_section: String,
}

fn actor(user_id: &str, role: Role, sections: Vec<String>, origin: Origin) -> Actor {
    Actor {
        user_id: user_id.to_owned(),
        role,
        section_ids: sections.into_iter().collect(),
        device_id: "VD-DEMO-DEMO-DEMO".to_owned(),
        lang: Lang::En,
        origin,
    }
}

impl Harness {
    fn new() -> Self {
        let db = Arc::new(Db::open_in_memory_for_tests().expect("db"));
        let clock = Arc::new(FixedClock {
            now: Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap(),
            today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        });
        let services = Services::new(
            db,
            clock,
            Arc::new(SeqIds::new("id-")),
            Arc::new(SeededRandom::new(1)),
            Mode::Server,
            "VD-DEMO-DEMO-DEMO".to_owned(),
        )
        .expect("services");
        SampleSchool::build(&services, 1).expect("sample school");

        let uid = |username: &str| -> String {
            services
                .db
                .read(|conn| vidya_db::repo::users::get_by_username(conn, username))
                .unwrap()
                .expect("user")
                .id
        };
        let section = |class: &str, sec: &str| -> String {
            services
                .db
                .read(|conn| {
                    Ok(conn.query_row(
                        "SELECT s.id FROM sections s JOIN classes c ON c.id = s.class_id WHERE c.name = ?1 AND s.name = ?2",
                        rusqlite::params![class, sec],
                        |row| row.get::<_, String>(0),
                    )?)
                })
                .unwrap()
        };

        let sunita = uid("sunita");
        let anita = uid("anita");
        let sierra = uid("sierra");
        let va = section("V", "A");
        let via = section("VI", "A");

        Self {
            principal: actor(&sunita, Role::Principal, vec![], Origin::OfficeComputer),
            accountant: actor(&anita, Role::Accountant, vec![], Origin::OfficeComputer),
            teacher_own: actor(&sierra, Role::Teacher, vec![va.clone()], Origin::OfficeComputer),
            teacher_other: actor(&sierra, Role::Teacher, vec![via], Origin::OfficeComputer),
            principal_phone: actor(&sunita, Role::Principal, vec![], Origin::Phone),
            accountant_phone: actor(&anita, Role::Accountant, vec![], Origin::Phone),
            va_section: va,
            services,
            sessions: SessionStore::new(),
        }
    }

    fn users(&self) -> UserService<'_> {
        UserService::new(&self.services, &self.sessions)
    }
    fn auth(&self) -> AuthService<'_> {
        AuthService::new(&self.services, &self.sessions)
    }

    fn labelled(&self) -> [(&'static str, &Actor); 6] {
        [
            ("principal", &self.principal),
            ("accountant", &self.accountant),
            ("teacher_own", &self.teacher_own),
            ("teacher_other", &self.teacher_other),
            ("principal_phone", &self.principal_phone),
            ("accountant_phone", &self.accountant_phone),
        ]
    }

    /// Runs `call` for every actor and asserts the outcome matches the policy.
    /// `section` is the section id the action operates on (for `Own` actions).
    fn check(
        &self,
        action: Action,
        section: Option<&str>,
        call: impl Fn(&Harness, &Actor) -> Result<(), ServiceError>,
    ) {
        for (label, actor) in self.labelled() {
            let result = call(self, actor);
            let office_only_phone = action.office_computer_only() && actor.origin == Origin::Phone;
            let allowed = if office_only_phone {
                false
            } else {
                match access(actor.role, action) {
                    Access::Yes => true,
                    Access::No => false,
                    Access::Own => section.is_some_and(|s| actor.section_ids.contains(s)),
                }
            };
            if allowed {
                if let Err(e) = &result {
                    assert_ne!(
                        e.kind,
                        ErrorKind::Permission,
                        "{label} should be allowed for {} but got Permission ({})",
                        action.as_str(),
                        e.message_key
                    );
                }
            } else {
                let e = result.expect_err(&format!("{label} should be denied for {}", action.as_str()));
                assert_eq!(e.kind, ErrorKind::Permission, "{label} / {}", action.as_str());
                if office_only_phone {
                    assert_eq!(e.message_key, "permission.office_computer_only");
                }
            }
        }
    }
}

fn create_input() -> CreateUserInput {
    CreateUserInput {
        name: "New Account".to_owned(),
        role: "accountant".to_owned(),
        mobile: String::new(),
        sections: Vec::new(),
        username: None,
    }
}

#[test]
fn permission_matrix() {
    let h = Harness::new();
    let sierra = h.teacher_own.user_id.clone();
    let va = h.va_section.clone();
    let mut covered: BTreeSet<&'static str> = BTreeSet::new();

    macro_rules! case {
        ($action:expr, $section:expr, $call:expr) => {{
            covered.insert($action.as_str());
            h.check($action, $section, $call);
        }};
    }

    // P3.1 — user management and account actions.
    case!(Action::UsersView, None, |h, a| h.users().list(a).map(|_| ()));
    case!(Action::UsersManage, None, |h, a| h
        .users()
        .create(a, create_input())
        .map(|_| ()));
    case!(Action::AccountChangeOwnPassword, None, |h, a| h
        .auth()
        .change_password(a, "wrong-current", "brandnew99", "no-token")
        .map(|_| ()));
    case!(Action::AccountSetOwnLanguage, None, |h, a| h
        .auth()
        .set_language(a, Lang::En)
        .map(|_| ()));

    // Extra UsersManage methods (same action, so already counted as covered).
    h.check(Action::UsersManage, None, |h, a| {
        h.users()
            .update(
                a,
                UpdateUserInput {
                    user_id: sierra.clone(),
                    name: "Sierra D'Souza".to_owned(),
                    mobile: String::new(),
                    sections: vec![va.clone()],
                },
            )
            .map(|_| ())
    });
    h.check(Action::UsersManage, None, |h, a| {
        h.users().unlock(a, &sierra).map(|_| ())
    });
    h.check(Action::UsersManage, None, |h, a| {
        h.users().set_active(a, &sierra, true).map(|_| ())
    });

    // Coverage: every action has a case, except those a later prompt adds.
    let pending: BTreeSet<&str> = NOT_YET_IMPLEMENTED.iter().map(|a| a.as_str()).collect();
    for action in Action::ALL {
        let key = action.as_str();
        if pending.contains(key) {
            assert!(
                !covered.contains(key),
                "{key} is covered but still in NOT_YET_IMPLEMENTED"
            );
        } else {
            assert!(covered.contains(key), "no permission case for {key}");
        }
    }
}
