use std::{collections::BTreeSet, str::FromStr};

use vidya_core::{
    error::ErrorKind,
    permissions::{access, authorize, authorize_section, permission_names, scope, Access, Action, Scope},
    roles::{Actor, Lang, Role},
};

const PERMISSIONS_DOC: &str = include_str!("../../../docs/PERMISSIONS.md");

fn document_rows() -> Vec<(String, Access, Access, Access)> {
    PERMISSIONS_DOC
        .lines()
        .filter(|line| line.starts_with("| "))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.len() != 4 || !cells[0].contains('.') {
                return None;
            }
            Some((
                cells[0].to_owned(),
                parse_access(cells[1]),
                parse_access(cells[2]),
                parse_access(cells[3]),
            ))
        })
        .collect()
}

fn parse_access(value: &str) -> Access {
    match value {
        "yes" => Access::Yes,
        "no" => Access::No,
        "own" => Access::Own,
        other => panic!("unknown access value in PERMISSIONS.md: {other}"),
    }
}

#[test]
fn permission_code_equals_the_document() {
    let rows = document_rows();
    let document_names = rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>();
    let code_names = Action::ALL
        .iter()
        .map(|action| action.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        document_names, code_names,
        "action list/order differs between docs/PERMISSIONS.md and Action::ALL"
    );

    let mut differences = Vec::new();
    for (name, principal, accountant, teacher) in rows {
        let action = Action::from_str(&name).unwrap_or_else(|error| panic!("{name}: {error}"));
        for (role, documented) in [
            (Role::Principal, principal),
            (Role::Accountant, accountant),
            (Role::Teacher, teacher),
        ] {
            let coded = access(role, action);
            if documented != coded {
                differences.push(format!(
                    "action={name}, role={role:?}, document={documented:?}, code={coded:?}"
                ));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "permission matrix mismatch:\n{}",
        differences.join("\n")
    );
}

fn actor(role: Role, sections: &[&str]) -> Actor {
    Actor {
        user_id: "user-1".into(),
        role,
        section_ids: sections.iter().map(|section| (*section).to_owned()).collect(),
        device_id: "device-1".into(),
        lang: Lang::En,
    }
}

#[test]
fn teacher_is_limited_to_assigned_sections() {
    let teacher = actor(Role::Teacher, &["V-A"]);
    assert!(authorize_section(&teacher, Action::MarksEnter, "V-A").is_ok());
    let denied = authorize_section(&teacher, Action::MarksEnter, "V-B").unwrap_err();
    assert_eq!(denied.kind, ErrorKind::Permission);
    assert_eq!(denied.message_key, "permission.denied");
    assert_eq!(
        scope(&teacher, Action::StudentsView),
        Ok(Scope::Sections(BTreeSet::from(["V-A".to_owned()])))
    );
    assert_eq!(
        authorize(&teacher, Action::FeesCollect).unwrap_err().kind,
        ErrorKind::Permission
    );
}

#[test]
fn teacher_without_sections_gets_specific_error() {
    let teacher = actor(Role::Teacher, &[]);
    let error = scope(&teacher, Action::StudentsView).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Permission);
    assert_eq!(error.message_key, "permission.no_sections");
}

#[test]
fn own_action_requires_section_authorizer() {
    let teacher = actor(Role::Teacher, &["V-A"]);
    let error = authorize(&teacher, Action::MarksEnter).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Internal);
    assert_eq!(error.message_key, "permission.needs_section");
}

#[test]
fn accountant_permissions_match_required_behaviour() {
    let accountant = actor(Role::Accountant, &[]);
    assert_eq!(scope(&accountant, Action::StudentsView), Ok(Scope::All));
    assert_eq!(
        authorize(&accountant, Action::FeesCancelReceipt)
            .unwrap_err()
            .kind,
        ErrorKind::Permission
    );
    assert_eq!(
        authorize(&accountant, Action::StudentsSetConcession)
            .unwrap_err()
            .kind,
        ErrorKind::Permission
    );
}

#[test]
fn principal_has_all_actions_with_all_scope() {
    let principal = actor(Role::Principal, &[]);
    for action in Action::ALL {
        assert_eq!(scope(&principal, *action), Ok(Scope::All), "{}", action.as_str());
    }
}

#[test]
fn teacher_ui_has_no_forbidden_permission_groups() {
    let permissions = permission_names(Role::Teacher);
    for prefix in ["fees.", "users.", "settings.", "backup.", "reports."] {
        assert!(
            permissions
                .iter()
                .all(|permission| !permission.starts_with(prefix)),
            "teacher permission unexpectedly starts with {prefix}: {permissions:?}"
        );
    }
}

#[test]
fn every_action_round_trips_through_from_str() {
    for action in Action::ALL {
        assert_eq!(Action::from_str(action.as_str()), Ok(*action));
    }
    let error = Action::from_str("not.an.action").unwrap_err();
    assert_eq!(error.kind, ErrorKind::Internal);
    assert_eq!(error.message_key, "permission.unknown_action");
}
