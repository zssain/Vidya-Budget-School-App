//! `AuthService`: sign-in with lockout, pending first-password tokens, idle
//! sessions, password/language changes, and actor resolution. Every other
//! service receives the `Actor` this produces.

use chrono::{DateTime, Duration, Local, Utc};
use serde::Serialize;
use serde_json::json;
use vidya_core::error::ErrorKind;
use vidya_core::permissions::{self, permission_names, Action};
use vidya_core::roles::{Actor, Lang, Origin, Role};
use vidya_core::secrets::format_session_token;
use vidya_core::validation::validate_new_password;
use vidya_db::repo;
use vidya_db::repo::users::UserRow;

use crate::auth::password::{dummy_verify, hash_password, verify_password};
use crate::auth::sessions::{SessionKind, SessionStore};
use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::Services;

const MAX_ATTEMPTS: i64 = 5;
const PAUSE_MINUTES: i64 = 5;
const DEFAULT_TIMEOUT_MINUTES: i64 = 30;

/// The signed-in user, as the frontend and LAN API see them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub full_username: String,
    pub role: Role,
    pub sections: Vec<String>,
    pub language: String,
    pub permissions: Vec<String>,
}

#[derive(Debug)]
pub enum SignInResult {
    Ok { token: String, user: CurrentUserDto },
    MustChangePassword { pending_token: String },
}

pub struct AuthService<'a> {
    services: &'a Services,
    sessions: &'a SessionStore,
}

fn role_from_str(role: &str) -> Result<Role, ServiceError> {
    match role {
        "principal" => Ok(Role::Principal),
        "accountant" => Ok(Role::Accountant),
        "teacher" => Ok(Role::Teacher),
        _ => Err(ServiceError::internal("unknown role in database")),
    }
}

fn lang_from_str(language: &str) -> Lang {
    if language == "hi" {
        Lang::Hi
    } else {
        Lang::En
    }
}

fn lang_to_str(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Hi => "hi",
    }
}

/// Formats an RFC3339 instant as a local `2:35 pm` time for the paused message.
fn format_local_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.with_timezone(&Local).format("%-I:%M %p").to_string())
        .unwrap_or_else(|_| rfc3339.to_owned())
}

impl<'a> AuthService<'a> {
    pub fn new(services: &'a Services, sessions: &'a SessionStore) -> Self {
        Self { services, sessions }
    }

    fn now_ms(&self) -> u64 {
        self.services.clock.now_ms()
    }

    fn timeout_ms(&self) -> u64 {
        let minutes = self
            .services
            .db
            .read(|conn| repo::app_settings::get(conn, "session_timeout_minutes"))
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_MINUTES)
            .max(1);
        (minutes as u64) * 60_000
    }

    fn new_token(&self) -> String {
        let mut bytes = [0u8; 32];
        self.services.random.fill(&mut bytes);
        format_session_token(bytes)
    }

    fn school_code(&self) -> String {
        self.services
            .db
            .read(repo::license::get)
            .ok()
            .flatten()
            .map(|license| license.school_code)
            .unwrap_or_default()
    }

    fn build_current_user(&self, user: &UserRow, origin: Origin) -> Result<CurrentUserDto, ServiceError> {
        let sections = self
            .services
            .db
            .read(|conn| repo::users::sections_for(conn, &user.id))?;
        let role = role_from_str(&user.role)?;
        let code = self.school_code();
        let full_username = if code.is_empty() {
            user.username.clone()
        } else {
            format!("{}@{}", user.username, code)
        };
        Ok(CurrentUserDto {
            id: user.id.clone(),
            name: user.name.clone(),
            username: user.username.clone(),
            full_username,
            role,
            sections: sections.into_iter().collect(),
            language: user.language.clone(),
            permissions: permission_names(role, origin)
                .into_iter()
                .map(String::from)
                .collect(),
        })
    }

    /// The `CurrentUserDto` for an already-resolved actor (the `current_user`
    /// command). Reads the user fresh so name/sections reflect the database.
    pub fn current_user_dto(&self, actor: &Actor) -> Result<CurrentUserDto, ServiceError> {
        let user = self
            .services
            .db
            .read(|conn| repo::users::get_by_id(conn, &actor.user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::Auth, "auth.error.signed_out"))?;
        self.build_current_user(&user, actor.origin)
    }

    /// Signs a user in. `origin` is trusted from the caller: the local command
    /// layer passes `OfficeComputer`, the LAN layer always passes `Phone`.
    pub fn sign_in(
        &self,
        username_input: &str,
        password: &str,
        device_id: &str,
        origin: Origin,
    ) -> Result<SignInResult, ServiceError> {
        // 1. Normalise and check the optional @school suffix.
        let normalized = username_input.trim().to_lowercase();
        let (username, at_school) = match normalized.split_once('@') {
            Some((user, school)) => (user.to_owned(), Some(school.to_owned())),
            None => (normalized, None),
        };
        if let Some(school) = at_school {
            let code = self.school_code();
            if school != code {
                return Err(ServiceError::new(ErrorKind::Auth, "auth.error.wrong_school").param("code", code));
            }
        }

        // 2. Load the user; unknown or inactive looks identical (and times the same).
        let user = match self
            .services
            .db
            .read(|conn| repo::users::get_by_username(conn, &username))?
        {
            Some(user) if user.active => user,
            _ => {
                dummy_verify(password);
                return Err(ServiceError::new(ErrorKind::Auth, "auth.error.no_login"));
            }
        };

        // 3 & 4. Locked or paused.
        if user.locked {
            return Err(ServiceError::new(ErrorKind::Locked, "auth.error.locked"));
        }
        if let Some(until) = &user.locked_until {
            if DateTime::parse_from_rfc3339(until)
                .map(|dt| dt.with_timezone(&Utc) > self.services.clock.now_utc())
                .unwrap_or(false)
            {
                return Err(ServiceError::new(ErrorKind::Locked, "auth.error.paused")
                    .param("time", format_local_time(until)));
            }
        }

        // 5. Verify the password.
        if !verify_password(&user.password_hash, password) {
            let new_count = user.failed_count + 1;
            let is_principal = user.role == "principal";
            let now = self.services.clock.now_utc();
            let pause_until = (now + Duration::minutes(PAUSE_MINUTES)).to_rfc3339();
            let hlc = self.services.next_hlc().to_text();

            self.services.db.write(|tx| {
                if new_count >= MAX_ATTEMPTS {
                    if is_principal {
                        repo::users::set_login_state(tx, &user.id, 0, false, Some(&pause_until), &hlc)?;
                        write_entry(self.services, tx, None, auth_event(&user.id, "auth.log.paused"))?;
                    } else {
                        repo::users::set_login_state(tx, &user.id, 0, true, None, &hlc)?;
                        write_entry(self.services, tx, None, auth_event(&user.id, "auth.log.locked"))?;
                    }
                } else {
                    repo::users::set_login_state(tx, &user.id, new_count, false, None, &hlc)?;
                    write_entry(
                        self.services,
                        tx,
                        None,
                        auth_event(&user.id, "auth.log.wrong_password"),
                    )?;
                }
                Ok(())
            })?;

            if new_count >= MAX_ATTEMPTS {
                return if is_principal {
                    Err(ServiceError::new(ErrorKind::Locked, "auth.error.paused")
                        .param("time", format_local_time(&pause_until)))
                } else {
                    Err(ServiceError::new(ErrorKind::Locked, "auth.error.locked"))
                };
            }
            return Err(ServiceError::new(ErrorKind::Auth, "auth.error.wrong_password")
                .param("left", (MAX_ATTEMPTS - new_count).to_string()));
        }

        // 6. Correct password but a change is required: pending session, no login stamp.
        let token = self.new_token();
        if user.must_change {
            let hlc = self.services.next_hlc().to_text();
            self.services
                .db
                .write(|tx| repo::users::set_login_state(tx, &user.id, 0, false, None, &hlc))?;
            self.sessions.create_pending(
                &token,
                user.id.clone(),
                device_id.to_owned(),
                origin,
                self.now_ms(),
            );
            return Ok(SignInResult::MustChangePassword { pending_token: token });
        }

        // 7. Full sign-in.
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            repo::users::record_login(tx, &user.id, &now, &hlc)?;
            write_entry(
                self.services,
                tx,
                None,
                auth_event(&user.id, "auth.log.signed_in"),
            )?;
            Ok(())
        })?;
        self.sessions.create_full(
            &token,
            user.id.clone(),
            device_id.to_owned(),
            origin,
            self.now_ms(),
        );
        let dto = self.build_current_user(&user, origin)?;
        Ok(SignInResult::Ok { token, user: dto })
    }

    /// Sets the first password for a pending session and signs the user in.
    pub fn set_first_password(
        &self,
        pending_token: &str,
        new_password: &str,
    ) -> Result<SignInResult, ServiceError> {
        let resolved = self
            .sessions
            .resolve(pending_token, self.now_ms(), self.timeout_ms())
            .filter(|r| r.kind == SessionKind::Pending)
            .ok_or_else(|| ServiceError::new(ErrorKind::Auth, "auth.error.password_change_required"))?;

        let user = self
            .services
            .db
            .read(|conn| repo::users::get_by_id(conn, &resolved.user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::Auth, "auth.error.no_login"))?;

        let same_as_current = verify_password(&user.password_hash, new_password);
        validate_new_password(new_password, &user.username, same_as_current)?;
        let hash = hash_password(&*self.services.random, new_password)?;
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            repo::users::set_password(tx, &user.id, &hash, &now, &hlc)?;
            repo::users::record_login(tx, &user.id, &now, &hlc)?;
            write_entry(
                self.services,
                tx,
                None,
                auth_event(&user.id, "auth.log.first_password"),
            )?;
            Ok(())
        })?;

        self.sessions.end(pending_token);
        let token = self.new_token();
        self.sessions.create_full(
            &token,
            user.id.clone(),
            resolved.device_id.clone(),
            resolved.origin,
            self.now_ms(),
        );
        let dto = self.build_current_user(&user, resolved.origin)?;
        Ok(SignInResult::Ok { token, user: dto })
    }

    /// Changes the signed-in user's password and signs out their other sessions.
    pub fn change_password(
        &self,
        actor: &Actor,
        current: &str,
        new: &str,
        current_token: &str,
    ) -> Result<(), ServiceError> {
        permissions::authorize(actor, Action::AccountChangeOwnPassword)?;
        let user = self
            .services
            .db
            .read(|conn| repo::users::get_by_id(conn, &actor.user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::Auth, "auth.error.no_login"))?;

        if !verify_password(&user.password_hash, current) {
            return Err(ServiceError::new(ErrorKind::Auth, "auth.error.current_wrong").field("current"));
        }
        let same_as_current = verify_password(&user.password_hash, new);
        validate_new_password(new, &user.username, same_as_current)?;
        let hash = hash_password(&*self.services.random, new)?;
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            repo::users::set_password(tx, &user.id, &hash, &now, &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                auth_event(&user.id, "auth.log.password_changed"),
            )?;
            Ok(())
        })?;
        self.sessions.end_others_for_user(&user.id, current_token);
        Ok(())
    }

    /// Sets the signed-in user's interface language.
    pub fn set_language(&self, actor: &Actor, lang: Lang) -> Result<(), ServiceError> {
        permissions::authorize(actor, Action::AccountSetOwnLanguage)?;
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            repo::users::set_language(tx, &actor.user_id, lang_to_str(lang), &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                auth_event(&actor.user_id, "auth.log.language"),
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Ends a session (sign out).
    pub fn sign_out(&self, token: &str) {
        self.sessions.end(token);
    }

    /// Resolves a full session token to a fresh `Actor`, reloading the user from
    /// the database so role/active/lock/sections changes take effect at once.
    pub fn actor_for_token(&self, token: &str) -> Result<Actor, ServiceError> {
        let timeout = self.timeout_ms();
        let resolved = self
            .sessions
            .resolve(token, self.now_ms(), timeout)
            .ok_or_else(|| {
                ServiceError::new(ErrorKind::Auth, "auth.error.session_expired")
                    .param("minutes", (timeout / 60_000).to_string())
            })?;
        if resolved.kind == SessionKind::Pending {
            return Err(ServiceError::new(
                ErrorKind::Auth,
                "auth.error.password_change_required",
            ));
        }

        let user = match self
            .services
            .db
            .read(|conn| repo::users::get_by_id(conn, &resolved.user_id))?
        {
            Some(user) if user.active && !user.locked => user,
            _ => {
                self.sessions.end_all_for_user(&resolved.user_id);
                return Err(ServiceError::new(ErrorKind::Auth, "auth.error.signed_out"));
            }
        };
        let sections = self
            .services
            .db
            .read(|conn| repo::users::sections_for(conn, &user.id))?;
        Ok(Actor {
            user_id: user.id,
            role: role_from_str(&user.role)?,
            section_ids: sections,
            device_id: resolved.device_id,
            lang: lang_from_str(&user.language),
            origin: resolved.origin,
        })
    }
}

/// An auth change-log event with an empty (secret-free) payload.
fn auth_event<'a>(user_id: &'a str, summary_key: &'a str) -> ChangeRecord<'a> {
    ChangeRecord {
        kind: "auth",
        entity: "user",
        entity_id: user_id,
        op: Op::Event,
        summary_key,
        params: json!({}),
        payload: json!({}),
    }
}
