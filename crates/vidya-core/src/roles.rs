use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// A supported user role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Principal,
    Accountant,
    Teacher,
}

/// A supported interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Hi,
}

/// Trusted server-side identity and scope passed into services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: String,
    pub role: Role,
    pub section_ids: BTreeSet<String>,
    pub device_id: String,
    pub lang: Lang,
}
