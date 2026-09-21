//! UserService behaviour tests (P3.1 Task 4).

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_db::repo::users::UserRow;
use vidya_db::{repo, Db};
use vidya_services::auth::service::SignInResult;
use vidya_services::auth::{AuthService, SessionStore};
use vidya_services::env::{FixedClock, SeededRandom, SeqIds};
use vidya_services::services::users::{CreateUserInput, UserService};
use vidya_services::{Mode, Services};
use vidya_testkit::SampleSchool;

struct Fx {
    services: Services,
    sessions: SessionStore,
}

impl Fx {
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
        SampleSchool::build(&services, 1).expect("sample");
        Self {
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

    fn actor(&self, username: &str, role: Role, origin: Origin) -> Actor {
        let id = self
            .services
            .db
            .read(|conn| repo::users::get_by_username(conn, username))
            .unwrap()
            .expect("user")
            .id;
        Actor {
            user_id: id,
            role,
            section_ids: BTreeSet::new(),
            device_id: "VD-DEMO-DEMO-DEMO".to_owned(),
            lang: Lang::En,
            origin,
        }
    }

    fn section(&self, class: &str, sec: &str) -> String {
        self.services
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT s.id FROM sections s JOIN classes c ON c.id = s.class_id WHERE c.name = ?1 AND s.name = ?2",
                    rusqlite::params![class, sec],
                    |row| row.get::<_, String>(0),
                )?)
            })
            .unwrap()
    }
}

#[test]
fn duplicate_first_name_gets_a_number() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    // "sierra" already exists in the sample school.
    let slip = fx
        .users()
        .create(
            &principal,
            CreateUserInput {
                name: "Sierra Khan".to_owned(),
                role: "teacher".to_owned(),
                mobile: String::new(),
                sections: vec![fx.section("III", "A")],
                username: None,
            },
        )
        .expect("create");
    assert_eq!(slip.username, "sierra2");
}

#[test]
fn creating_a_principal_is_refused() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    let err = fx
        .users()
        .create(
            &principal,
            CreateUserInput {
                name: "Another Boss".to_owned(),
                role: "principal".to_owned(),
                mobile: String::new(),
                sections: Vec::new(),
                username: None,
            },
        )
        .expect_err("refused");
    assert_eq!(err.kind, ErrorKind::Permission);
    assert_eq!(err.message_key, "users.error.principal_once");
}

#[test]
fn active_user_limit_is_enforced() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    // Fill up to the licence max (40) with repo inserts (fast, no hashing).
    let existing = fx.services.db.read(repo::users::count_active).unwrap();
    fx.services
        .db
        .write(|tx| {
            for i in existing..40 {
                repo::users::insert(
                    tx,
                    &UserRow {
                        id: format!("filler-{i}"),
                        username: format!("filler{i}"),
                        name: format!("Filler {i}"),
                        role: "accountant".to_owned(),
                        mobile: String::new(),
                        password_hash: "$argon2id$x".to_owned(),
                        must_change: false,
                        failed_count: 0,
                        locked: false,
                        locked_until: None,
                        active: true,
                        language: "en".to_owned(),
                        created_at: "2026-09-20T10:00:00Z".to_owned(),
                        last_login_at: None,
                        password_changed_at: None,
                    },
                    "hlc",
                )?;
            }
            Ok(())
        })
        .unwrap();
    let err = fx
        .users()
        .create(
            &principal,
            CreateUserInput {
                name: "One Too Many".to_owned(),
                role: "accountant".to_owned(),
                mobile: String::new(),
                sections: Vec::new(),
                username: None,
            },
        )
        .expect_err("limit");
    assert_eq!(err.message_key, "users.error.limit");
    assert_eq!(err.params.get("max").map(String::as_str), Some("40"));
}

#[test]
fn reset_password_replaces_the_old_one() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    let slip1 = fx
        .users()
        .create(
            &principal,
            CreateUserInput {
                name: "Meera Joshi".to_owned(),
                role: "teacher".to_owned(),
                mobile: String::new(),
                sections: vec![fx.section("III", "A")],
                username: None,
            },
        )
        .expect("create");
    let meera_id = fx
        .services
        .db
        .read(|conn| repo::users::get_by_username(conn, &slip1.username))
        .unwrap()
        .unwrap()
        .id;
    let slip2 = fx.users().reset_password(&principal, &meera_id).expect("reset");
    assert_ne!(slip1.temp_password, slip2.temp_password);

    // Old temporary password no longer signs in; the new one does (must change).
    assert!(fx
        .auth()
        .sign_in(&slip1.username, &slip1.temp_password, "d", Origin::OfficeComputer)
        .is_err());
    assert!(matches!(
        fx.auth()
            .sign_in(&slip1.username, &slip2.temp_password, "d", Origin::OfficeComputer)
            .expect("sign in"),
        SignInResult::MustChangePassword { .. }
    ));
}

#[test]
fn switching_off_ends_the_session() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    let token = match fx
        .auth()
        .sign_in("sierra", "vidya123", "d", Origin::OfficeComputer)
        .expect("sign in")
    {
        SignInResult::Ok { token, .. } => token,
        SignInResult::MustChangePassword { .. } => panic!("unexpected"),
    };
    assert!(fx.auth().actor_for_token(&token).is_ok());

    let sierra_id = fx
        .services
        .db
        .read(|conn| repo::users::get_by_username(conn, "sierra"))
        .unwrap()
        .unwrap()
        .id;
    fx.users()
        .set_active(&principal, &sierra_id, false)
        .expect("switch off");
    assert!(fx.auth().actor_for_token(&token).is_err());
}

#[test]
fn temp_password_never_reaches_the_change_log() {
    let fx = Fx::new();
    let principal = fx.actor("sunita", Role::Principal, Origin::OfficeComputer);
    let slip = fx
        .users()
        .create(
            &principal,
            CreateUserInput {
                name: "Meera Joshi".to_owned(),
                role: "teacher".to_owned(),
                mobile: String::new(),
                sections: vec![fx.section("III", "A")],
                username: None,
            },
        )
        .expect("create");
    let logs: Vec<String> = fx
        .services
        .db
        .read(|conn| {
            let mut stmt = conn.prepare("SELECT payload_json || params_json FROM change_log")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(vidya_db::DbError::from)
        })
        .unwrap();
    for entry in logs {
        assert!(!entry.contains(&slip.temp_password));
    }
}

#[test]
fn principal_phone_cannot_create() {
    let fx = Fx::new();
    let principal_phone = fx.actor("sunita", Role::Principal, Origin::Phone);
    let err = fx
        .users()
        .create(
            &principal_phone,
            CreateUserInput {
                name: "New Account".to_owned(),
                role: "accountant".to_owned(),
                mobile: String::new(),
                sections: Vec::new(),
                username: None,
            },
        )
        .expect_err("office only");
    assert_eq!(err.message_key, "permission.office_computer_only");
}
