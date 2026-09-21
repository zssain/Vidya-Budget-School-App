//! Staff login management (principal only). Each write authorizes, validates,
//! runs one transaction and appends a change-log entry (`kind = "user"`). The
//! password hash is never returned; the temporary password is returned only from
//! `create`/`reset_password`.

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use vidya_core::error::ErrorKind;
use vidya_core::permissions::{self, Action};
use vidya_core::roles::{Actor, Role};
use vidya_core::secrets::format_temp_password;
use vidya_core::usernames::{fallback_username_base, unique_username, username_base_from_name};
use vidya_core::validation::{validate_mobile, validate_person_name, validate_username};
use vidya_db::repo::users::UserRow;
use vidya_db::repo::{classes, license, users};
use vidya_db::DbError;

use crate::auth::password::hash_password;
use crate::auth::SessionStore;
use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::Services;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub full_username: String,
    pub role: Role,
    pub mobile: String,
    pub sections: Vec<String>,
    /// One of `active`, `must_change`, `locked`, `switched_off`.
    pub status: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSlipDto {
    pub name: String,
    pub username: String,
    pub full_username: String,
    pub temp_password: String,
    pub role: Role,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub mobile: String,
    #[serde(default)]
    pub sections: Vec<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInput {
    pub user_id: String,
    pub name: String,
    #[serde(default)]
    pub mobile: String,
    #[serde(default)]
    pub sections: Vec<String>,
}

pub struct UserService<'a> {
    services: &'a Services,
    sessions: &'a SessionStore,
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Principal => "principal",
        Role::Accountant => "accountant",
        Role::Teacher => "teacher",
    }
}

fn role_of(text: &str) -> Role {
    match text {
        "principal" => Role::Principal,
        "accountant" => Role::Accountant,
        _ => Role::Teacher,
    }
}

fn status_of(user: &UserRow) -> &'static str {
    if !user.active {
        "switched_off"
    } else if user.locked {
        "locked"
    } else if user.must_change {
        "must_change"
    } else {
        "active"
    }
}

fn user_event<'a>(user_id: &'a str, summary_key: &'a str) -> ChangeRecord<'a> {
    ChangeRecord {
        kind: "user",
        entity: "user",
        entity_id: user_id,
        op: Op::Event,
        summary_key,
        params: serde_json::json!({}),
        payload: serde_json::json!({}),
    }
}

impl<'a> UserService<'a> {
    pub fn new(services: &'a Services, sessions: &'a SessionStore) -> Self {
        Self { services, sessions }
    }

    fn school_code(&self) -> String {
        self.services
            .db
            .read(license::get)
            .ok()
            .flatten()
            .map(|l| l.school_code)
            .unwrap_or_default()
    }

    fn full_username(&self, username: &str) -> String {
        let code = self.school_code();
        if code.is_empty() {
            username.to_owned()
        } else {
            format!("{username}@{code}")
        }
    }

    fn to_dto(&self, conn: &Connection, user: UserRow) -> Result<UserDto, DbError> {
        let mut sections = Vec::new();
        for section_id in users::sections_for(conn, &user.id)? {
            if let Some(label) = classes::section_label(conn, &section_id)? {
                sections.push(label);
            }
        }
        sections.sort();
        Ok(UserDto {
            full_username: self.full_username(&user.username),
            status: status_of(&user).to_owned(),
            role: role_of(&user.role),
            sections,
            id: user.id,
            name: user.name,
            username: user.username,
            mobile: user.mobile,
            last_login_at: user.last_login_at,
        })
    }

    fn dto_by_id(&self, user_id: &str) -> Result<UserDto, ServiceError> {
        self.services
            .db
            .read(|conn| {
                let user =
                    users::get_by_id(conn, user_id)?.ok_or(DbError::Pool("user not found".to_owned()))?;
                self.to_dto(conn, user)
            })
            .map_err(|_| ServiceError::new(ErrorKind::NotFound, "users.error.not_found"))
    }

    pub fn list(&self, actor: &Actor) -> Result<Vec<UserDto>, ServiceError> {
        permissions::authorize(actor, Action::UsersView)?;
        let dtos = self.services.db.read(|conn| {
            users::list(conn)?
                .into_iter()
                .map(|user| self.to_dto(conn, user))
                .collect::<Result<Vec<_>, DbError>>()
        })?;
        Ok(dtos)
    }

    pub fn create(&self, actor: &Actor, input: CreateUserInput) -> Result<CredentialSlipDto, ServiceError> {
        permissions::authorize(actor, Action::UsersManage)?;
        let role = match input.role.as_str() {
            "teacher" => Role::Teacher,
            "accountant" => Role::Accountant,
            "principal" => {
                return Err(ServiceError::new(
                    ErrorKind::Permission,
                    "users.error.principal_once",
                ))
            }
            _ => return Err(ServiceError::validation("users.error.role").field("role")),
        };
        let name = validate_person_name(&input.name, "name")?;
        let mobile = if input.mobile.trim().is_empty() {
            String::new()
        } else {
            validate_mobile(&input.mobile)?
        };

        // A teacher must be assigned at least one existing active section.
        if role == Role::Teacher {
            let sections_ok = self.services.db.read(|conn| {
                if input.sections.is_empty() {
                    return Ok(false);
                }
                for section_id in &input.sections {
                    if !classes::section_is_active(conn, section_id)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            })?;
            if !sections_ok {
                return Err(ServiceError::validation("users.error.no_sections").field("sections"));
            }
        }

        let (max_users, active_count, taken) = self.services.db.read(|conn| {
            let max = license::get(conn)?.map(|l| l.max_users).unwrap_or(i64::MAX);
            let count = users::count_active(conn)?;
            let taken: BTreeSet<String> = users::list(conn)?.into_iter().map(|u| u.username).collect();
            Ok((max, count, taken))
        })?;
        if active_count >= max_users {
            return Err(ServiceError::validation("users.error.limit").param("max", max_users.to_string()));
        }

        let username = match input.username.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(candidate) => {
                let candidate = validate_username(candidate)?;
                if taken.contains(&candidate) {
                    return Err(
                        ServiceError::new(ErrorKind::Conflict, "users.error.username_taken")
                            .field("username"),
                    );
                }
                candidate
            }
            None => {
                let base = username_base_from_name(&name)
                    .unwrap_or_else(|| fallback_username_base(role, active_count as usize + 1));
                unique_username(&base, &taken)
            }
        };

        let mut bytes = [0u8; 8];
        self.services.random.fill(&mut bytes);
        let temp_password = format_temp_password(bytes);
        let hash = hash_password(&*self.services.random, &temp_password)?;

        let id = self.services.ids.new_id();
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        let sections = input.sections.clone();
        self.services.db.write(|tx| {
            users::insert(
                tx,
                &UserRow {
                    id: id.clone(),
                    username: username.clone(),
                    name: name.clone(),
                    role: role_str(role).to_owned(),
                    mobile: mobile.clone(),
                    password_hash: hash.clone(),
                    must_change: true,
                    failed_count: 0,
                    locked: false,
                    locked_until: None,
                    active: true,
                    language: "en".to_owned(),
                    created_at: now.clone(),
                    last_login_at: None,
                    password_changed_at: None,
                },
                &hlc,
            )?;
            if role == Role::Teacher {
                users::set_sections(tx, &id, &sections)?;
            }
            write_entry(
                self.services,
                tx,
                Some(actor),
                user_event(&id, "users.log.created"),
            )?;
            Ok(())
        })?;

        Ok(CredentialSlipDto {
            full_username: self.full_username(&username),
            name,
            username,
            temp_password,
            role,
        })
    }

    pub fn update(&self, actor: &Actor, input: UpdateUserInput) -> Result<UserDto, ServiceError> {
        permissions::authorize(actor, Action::UsersManage)?;
        let user = self
            .services
            .db
            .read(|conn| users::get_by_id(conn, &input.user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "users.error.not_found"))?;
        let name = validate_person_name(&input.name, "name")?;
        let mobile = if input.mobile.trim().is_empty() {
            String::new()
        } else {
            validate_mobile(&input.mobile)?
        };
        let is_teacher = user.role == "teacher";
        if is_teacher {
            let ok = self.services.db.read(|conn| {
                for section_id in &input.sections {
                    if !classes::section_is_active(conn, section_id)? {
                        return Ok(false);
                    }
                }
                Ok(!input.sections.is_empty())
            })?;
            if !ok {
                return Err(ServiceError::validation("users.error.no_sections").field("sections"));
            }
        }
        let hlc = self.services.next_hlc().to_text();
        let sections = input.sections.clone();
        self.services.db.write(|tx| {
            users::update_profile(tx, &user.id, &name, &mobile, &hlc)?;
            if is_teacher {
                users::set_sections(tx, &user.id, &sections)?;
            }
            write_entry(
                self.services,
                tx,
                Some(actor),
                user_event(&user.id, "users.log.updated"),
            )?;
            Ok(())
        })?;
        self.dto_by_id(&user.id)
    }

    pub fn reset_password(&self, actor: &Actor, user_id: &str) -> Result<CredentialSlipDto, ServiceError> {
        permissions::authorize(actor, Action::UsersManage)?;
        let user = self
            .services
            .db
            .read(|conn| users::get_by_id(conn, user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "users.error.not_found"))?;
        if user.role == "principal" {
            return Err(ServiceError::new(
                ErrorKind::Permission,
                "users.error.principal_self",
            ));
        }
        let mut bytes = [0u8; 8];
        self.services.random.fill(&mut bytes);
        let temp_password = format_temp_password(bytes);
        let hash = hash_password(&*self.services.random, &temp_password)?;
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            users::reset_for_temp_password(tx, &user.id, &hash, &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                user_event(&user.id, "users.log.reset"),
            )?;
            Ok(())
        })?;
        self.sessions.end_all_for_user(&user.id);
        Ok(CredentialSlipDto {
            full_username: self.full_username(&user.username),
            name: user.name,
            username: user.username,
            temp_password,
            role: role_of(&user.role),
        })
    }

    pub fn unlock(&self, actor: &Actor, user_id: &str) -> Result<UserDto, ServiceError> {
        permissions::authorize(actor, Action::UsersManage)?;
        let hlc = self.services.next_hlc().to_text();
        self.services.db.write(|tx| {
            users::unlock(tx, user_id, &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                user_event(user_id, "users.log.unlocked"),
            )?;
            Ok(())
        })?;
        self.dto_by_id(user_id)
    }

    pub fn set_active(&self, actor: &Actor, user_id: &str, active: bool) -> Result<UserDto, ServiceError> {
        permissions::authorize(actor, Action::UsersManage)?;
        let user = self
            .services
            .db
            .read(|conn| users::get_by_id(conn, user_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "users.error.not_found"))?;
        if user.role == "principal" && !active {
            return Err(ServiceError::new(
                ErrorKind::Permission,
                "users.error.principal_off",
            ));
        }
        if active && !user.active {
            let (max, count) = self.services.db.read(|conn| {
                Ok((
                    license::get(conn)?.map(|l| l.max_users).unwrap_or(i64::MAX),
                    users::count_active(conn)?,
                ))
            })?;
            if count >= max {
                return Err(ServiceError::validation("users.error.limit").param("max", max.to_string()));
            }
        }
        let hlc = self.services.next_hlc().to_text();
        let summary = if active {
            "users.log.switched_on"
        } else {
            "users.log.switched_off"
        };
        self.services.db.write(|tx| {
            users::set_active(tx, user_id, active, &hlc)?;
            write_entry(self.services, tx, Some(actor), user_event(user_id, summary))?;
            Ok(())
        })?;
        if !active {
            self.sessions.end_all_for_user(user_id);
        }
        self.dto_by_id(user_id)
    }
}
