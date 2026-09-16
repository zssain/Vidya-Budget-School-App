use std::collections::BTreeSet;

use crate::roles::Role;

/// Builds a lowercase ASCII username base from the first word of a name.
pub fn username_base_from_name(name: &str) -> Option<String> {
    let first = name.split_whitespace().next()?;
    let base = first
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    (!base.is_empty()).then_some(base)
}

/// Appends the first available suffix, beginning with 2, to make a username unique.
pub fn unique_username(base: &str, taken: &BTreeSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_owned();
    }
    for suffix in 2_u64.. {
        let candidate = format!("{base}{suffix}");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("the unbounded numeric suffix must eventually be unique")
}

/// Returns a deterministic fallback base when a name has no ASCII letters or digits.
pub fn fallback_username_base(role: Role, index: usize) -> String {
    let prefix = match role {
        Role::Principal => "principal",
        Role::Accountant => "accountant",
        Role::Teacher => "teacher",
    };
    format!("{prefix}{index}")
}
