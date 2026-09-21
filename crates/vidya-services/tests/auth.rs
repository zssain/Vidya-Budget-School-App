//! AuthService tests: sign-in, lockout, pending first password, idle timeout,
//! actor resolution and origin (P2.6).

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use vidya_core::error::ErrorKind;
use vidya_core::permissions::{self, Action};
use vidya_core::roles::Origin;
use vidya_db::{repo, Db};
use vidya_services::auth::password::hash_password;
use vidya_services::auth::service::{AuthService, SignInResult};
use vidya_services::auth::SessionStore;
use vidya_services::env::{Clock, SeededRandom, SeqIds};
use vidya_services::{Mode, Services};
use vidya_testkit::SampleSchool;

/// A clock tests can advance.
struct TestClock {
    now: Mutex<DateTime<Utc>>,
    today: NaiveDate,
}

impl TestClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(Utc.with_ymd_and_hms(2026, 9, 20, 10, 0, 0).unwrap()),
            today: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        })
    }
    fn advance_minutes(&self, minutes: i64) {
        let mut now = self.now.lock().unwrap();
        *now += Duration::minutes(minutes);
    }
}

impl Clock for TestClock {
    fn now_utc(&self) -> DateTime<Utc> {
        *self.now.lock().unwrap()
    }
    fn today_local(&self) -> NaiveDate {
        self.today
    }
    fn now_ms(&self) -> u64 {
        self.now.lock().unwrap().timestamp_millis().max(0) as u64
    }
}

struct Fixture {
    services: Services,
    sessions: SessionStore,
    clock: Arc<TestClock>,
}

impl Fixture {
    fn new() -> Self {
        let db = Arc::new(Db::open_in_memory_for_tests().expect("db"));
        let clock = TestClock::new();
        let services = Services::new(
            db,
            clock.clone(),
            Arc::new(SeqIds::new("id-")),
            Arc::new(SeededRandom::new(1)),
            Mode::Server,
            "VD-DEMO-DEMO-DEMO".to_owned(),
        )
        .expect("services");
        SampleSchool::build(&services, 1).expect("sample school");
        Self {
            services,
            sessions: SessionStore::new(),
            clock,
        }
    }

    fn auth(&self) -> AuthService<'_> {
        AuthService::new(&self.services, &self.sessions)
    }

    /// Inserts a user with a known password. Returns nothing; sign in by username.
    fn add_user(&self, username: &str, password: &str, role: &str, must_change: bool, active: bool) {
        let hash = hash_password(&*self.services.random, password).expect("hash");
        let id = format!("u-{username}");
        self.services
            .db
            .write(|tx| {
                repo::users::insert(
                    tx,
                    &repo::users::UserRow {
                        id,
                        username: username.to_owned(),
                        name: username.to_owned(),
                        role: role.to_owned(),
                        mobile: String::new(),
                        password_hash: hash,
                        must_change,
                        failed_count: 0,
                        locked: false,
                        locked_until: None,
                        active,
                        language: "en".to_owned(),
                        created_at: "2026-09-20T10:00:00Z".to_owned(),
                        last_login_at: None,
                        password_changed_at: None,
                    },
                    "hlc",
                )
            })
            .expect("insert user");
    }

    fn set_inactive(&self, username: &str) {
        self.services
            .db
            .write(|tx| repo::users::set_active(tx, &format!("u-{username}"), false, "hlc"))
            .expect("set inactive");
    }
}

fn ok_token(result: SignInResult) -> String {
    match result {
        SignInResult::Ok { token, .. } => token,
        SignInResult::MustChangePassword { .. } => panic!("expected Ok, got MustChangePassword"),
    }
}

#[test]
fn principal_signs_in_with_office_permissions() {
    let fx = Fixture::new();
    let result = fx
        .auth()
        .sign_in("sunita", "vidya123", "VD-DEMO-DEMO-DEMO", Origin::OfficeComputer)
        .expect("sign in");
    match result {
        SignInResult::Ok { user, .. } => {
            assert_eq!(user.username, "sunita");
            assert!(user.permissions.iter().any(|p| p == "backup.manage"));
        }
        SignInResult::MustChangePassword { .. } => panic!("unexpected must-change"),
    }
}

#[test]
fn school_suffix_is_checked() {
    let fx = Fixture::new();
    assert!(fx
        .auth()
        .sign_in("sunita@vaani", "vidya123", "d", Origin::OfficeComputer)
        .is_ok());
    let err = fx
        .auth()
        .sign_in("sunita@other", "vidya123", "d", Origin::OfficeComputer)
        .expect_err("wrong school");
    assert_eq!(err.message_key, "auth.error.wrong_school");
}

#[test]
fn unknown_and_inactive_look_the_same() {
    let fx = Fixture::new();
    fx.add_user("gone", "vidya123", "teacher", false, false);
    for username in ["nobody", "gone"] {
        let err = fx
            .auth()
            .sign_in(username, "vidya123", "d", Origin::OfficeComputer)
            .expect_err("no login");
        assert_eq!(err.message_key, "auth.error.no_login");
    }
}

#[test]
fn four_wrong_then_correct_resets_count() {
    let fx = Fixture::new();
    for _ in 0..4 {
        assert!(fx
            .auth()
            .sign_in("sierra", "nope", "d", Origin::OfficeComputer)
            .is_err());
    }
    assert!(fx
        .auth()
        .sign_in("sierra", "vidya123", "d", Origin::OfficeComputer)
        .is_ok());
    // Count was reset; a fresh wrong attempt reports 4 tries left.
    let err = fx
        .auth()
        .sign_in("sierra", "nope", "d", Origin::OfficeComputer)
        .expect_err("wrong");
    assert_eq!(err.params.get("left").map(String::as_str), Some("4"));
}

#[test]
fn five_wrong_locks_staff() {
    let fx = Fixture::new();
    for _ in 0..5 {
        let _ = fx.auth().sign_in("sierra", "nope", "d", Origin::OfficeComputer);
    }
    let err = fx
        .auth()
        .sign_in("sierra", "vidya123", "d", Origin::OfficeComputer)
        .expect_err("locked");
    assert_eq!(err.message_key, "auth.error.locked");
    assert_eq!(err.kind, ErrorKind::Locked);
}

#[test]
fn five_wrong_pauses_principal_then_clears() {
    let fx = Fixture::new();
    for _ in 0..5 {
        let _ = fx.auth().sign_in("sunita", "nope", "d", Origin::OfficeComputer);
    }
    let err = fx
        .auth()
        .sign_in("sunita", "vidya123", "d", Origin::OfficeComputer)
        .expect_err("paused");
    assert_eq!(err.message_key, "auth.error.paused");
    fx.clock.advance_minutes(6);
    assert!(fx
        .auth()
        .sign_in("sunita", "vidya123", "d", Origin::OfficeComputer)
        .is_ok());
}

#[test]
fn must_change_flow() {
    let fx = Fixture::new();
    fx.add_user("meera", "temp1234", "teacher", true, true);
    let pending = match fx
        .auth()
        .sign_in("meera", "temp1234", "d", Origin::OfficeComputer)
        .expect("sign in")
    {
        SignInResult::MustChangePassword { pending_token } => pending_token,
        SignInResult::Ok { .. } => panic!("expected must-change"),
    };
    // A pending token is not a usable session.
    assert!(fx.auth().actor_for_token(&pending).is_err());
    // A new password containing the username is rejected.
    assert!(fx.auth().set_first_password(&pending, "meera-secret").is_err());
    // A valid new password signs the user in.
    let token = ok_token(
        fx.auth()
            .set_first_password(&pending, "brandnew99")
            .expect("set pw"),
    );
    assert!(fx.auth().actor_for_token(&token).is_ok());
}

#[test]
fn idle_session_expires() {
    let fx = Fixture::new();
    let token = ok_token(
        fx.auth()
            .sign_in("sunita", "vidya123", "d", Origin::OfficeComputer)
            .expect("sign in"),
    );
    assert!(fx.auth().actor_for_token(&token).is_ok());
    fx.clock.advance_minutes(31);
    let err = fx.auth().actor_for_token(&token).expect_err("expired");
    assert_eq!(err.message_key, "auth.error.session_expired");
}

#[test]
fn deactivating_user_ends_sessions() {
    let fx = Fixture::new();
    fx.add_user("temp", "vidya123", "teacher", false, true);
    let token = ok_token(
        fx.auth()
            .sign_in("temp", "vidya123", "d", Origin::OfficeComputer)
            .expect("sign in"),
    );
    assert!(fx.auth().actor_for_token(&token).is_ok());
    fx.set_inactive("temp");
    let err = fx.auth().actor_for_token(&token).expect_err("signed out");
    assert_eq!(err.message_key, "auth.error.signed_out");
}

#[test]
fn change_log_never_contains_secrets() {
    let fx = Fixture::new();
    let _ = fx
        .auth()
        .sign_in("sunita", "vidya123", "d", Origin::OfficeComputer);
    let payloads: Vec<String> = fx
        .services
        .db
        .read(|conn| {
            let mut stmt = conn.prepare("SELECT payload_json || params_json FROM change_log")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(vidya_db::DbError::from)
        })
        .expect("read change log");
    for payload in payloads {
        assert!(!payload.to_lowercase().contains("password"));
        assert!(!payload.contains("$argon2"));
        assert!(!payload.contains("vidya123"));
    }
}

#[test]
fn phone_origin_cannot_run_office_only_actions() {
    let fx = Fixture::new();
    let token = ok_token(
        fx.auth()
            .sign_in("sunita", "vidya123", "VD-DEMO-DEMO-DEMO", Origin::Phone)
            .expect("sign in"),
    );
    let actor = fx.auth().actor_for_token(&token).expect("actor");
    assert_eq!(actor.origin, Origin::Phone);
    let err = permissions::authorize(&actor, Action::BackupRun).expect_err("office only");
    assert_eq!(err.message_key, "permission.office_computer_only");
}
