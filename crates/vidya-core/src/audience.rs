//! Audience of an op — who may decrypt its Drive bundle (§8.8, prompts/P06 Step 4).
//!
//! A Drive bundle is sealed per **audience** (§8.8). Three audiences exist:
//! `admin` (Principal devices), `finance` (Principal + accountants), and one
//! `class:<class_id>` per class (Principal + teachers assigned to that class).
//! A device receives keys only for its audiences.
//!
//! The audience of an op is decided by its table (and, for class-scoped tables,
//! the record's class). Because the **pushing device must hold the audience key**
//! to seal the bundle, the mapping is constrained: every table a role can push
//! must map to an audience that role holds. That makes three tables genuinely
//! ambiguous under §8.8 and they are deliberately **not** decided here — see
//! [`audience_for`] and the Phase-6 handoff `[OWNER]` note.

use crate::errors::{CoreError, CoreResult};

/// A sealing audience (§8.8). The string form (`as_str`) is what is stored in
/// `outbox.audience`, delivered as an `AudienceKey.audience`, and written into
/// the `.vop` bundle's associated data / filename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    /// Principal devices only.
    Admin,
    /// Principal + accountants.
    Finance,
    /// Principal + teachers assigned to this class. Holds the class id (a UUID).
    Class(String),
}

impl Audience {
    /// Wire/string form: `admin` | `finance` | `class:<class_id>`.
    pub fn as_str(&self) -> String {
        match self {
            Audience::Admin => "admin".to_string(),
            Audience::Finance => "finance".to_string(),
            Audience::Class(id) => format!("class:{id}"),
        }
    }

    /// Parse the string form back into an `Audience`.
    pub fn parse(s: &str) -> CoreResult<Audience> {
        match s {
            "admin" => Ok(Audience::Admin),
            "finance" => Ok(Audience::Finance),
            other => match other.strip_prefix("class:") {
                Some(id) if !id.is_empty() => Ok(Audience::Class(id.to_string())),
                _ => Err(CoreError::Validation {
                    field: "audience".into(),
                    rule: "unknown_audience".into(),
                }),
            },
        }
    }
}

/// Decide the audience of an op from its `table` (and the record's `class_id`
/// for class-scoped tables).
///
/// * Finance domain (`fee_head`, `fee_due`, `payment`, `payment_allocation`,
///   `reversal`) → [`Audience::Finance`].
/// * Class-scoped academic data (`attendance_sheet`, `attendance_mark`,
///   `marks_sheet`, `mark_entry`, `exam`, `exam_subject`) → [`Audience::Class`]
///   (requires `class_id`; the caller resolves it from the record, as
///   `permissions::can` already does).
/// * Principal-only administrative data (`school`, `academic_session`, `term`,
///   `subject`, `class`, `class_subject`, `staff`, `device`, `invite`,
///   `conflict`, `review_flag`, `notification`, `licence`, `grade_scale`,
///   `grade_band`) → [`Audience::Admin`].
///
/// **Not decided here (returns `VALIDATION`):** `student`, `enrollment`, and
/// `request`. These cross the finance/class boundary — an accountant (holds only
/// `finance`) and a teacher (holds only `class:<own>`) can each push some of
/// them, so no single table→audience rule keeps every pusher able to seal. This
/// is an `[OWNER]` policy decision recorded in the Phase-6 handoff; until it is
/// taken, the command layer sets those ops' audience explicitly.
pub fn audience_for(table: &str, class_id: Option<&str>) -> CoreResult<Audience> {
    match table {
        // ---- finance domain -------------------------------------------------
        "fee_head" | "fee_due" | "payment" | "payment_allocation" | "reversal" => {
            Ok(Audience::Finance)
        }
        // ---- class-scoped academic data ------------------------------------
        "attendance_sheet" | "attendance_mark" | "marks_sheet" | "mark_entry" | "exam"
        | "exam_subject" => match class_id {
            Some(id) if !id.is_empty() => Ok(Audience::Class(id.to_string())),
            _ => Err(CoreError::Validation {
                field: "class_id".into(),
                rule: "required_for_class_audience".into(),
            }),
        },
        // ---- student roster: [OWNER default, P06] = finance ----------------
        // student/enrollment are written by accountants (admissions, transfers,
        // detail edits) and the Principal — both hold `finance`; a teacher only
        // *reads* the roster (via requests / the server snapshot). `finance` is
        // the one audience every pusher of these tables holds, so it is the
        // pusher-consistent default. Consequence: during a server outage a teacher
        // won't see a *new* admission via Drive until the server returns (it
        // reconciles on import). The alternative (`class:<id>`) would let teachers
        // see roster changes but leave accountants unable to seal — worse.
        // guardian/student_guardian belong to the student-roster domain (P13):
        // written at admission by accountants + the Principal (both hold finance),
        // read by teachers via the server snapshot — same pusher-consistent choice
        // as student/enrollment.
        "student" | "enrollment" | "guardian" | "student_guardian" => Ok(Audience::Finance),
        // ---- principal-only administrative data ----------------------------
        // Calendar (P13): Principal-edited school-wide config; every role reads it
        // via the server snapshot (like the student roster). Pusher = Principal →
        // `admin` is the pusher-consistent audience.
        "school" | "academic_session" | "term" | "subject" | "class" | "class_subject"
        | "staff" | "device" | "invite" | "conflict" | "review_flag" | "notification"
        | "licence" | "grade_scale" | "grade_band" | "school_week" | "calendar_event" => {
            Ok(Audience::Admin)
        }
        // ---- request: audience follows the requester's own domain ----------
        // A request is sealed by whoever raises it, so its audience is the
        // requester's own (a teacher's correction under `class:<own>`, an
        // accountant's under `finance`) — record/actor-dependent, not table-only,
        // so it cannot be decided from the table alone. The write path passes it
        // explicitly; the Principal's *approve* op is an `admin` action.
        "request" => Err(CoreError::Validation {
            field: "table".into(),
            rule: "audience_from_requester".into(),
        }),
        // ---- unknown table --------------------------------------------------
        _ => Err(CoreError::Validation {
            field: "table".into(),
            rule: "unknown_table".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finance_tables_map_to_finance() {
        for t in ["fee_head", "fee_due", "payment", "payment_allocation", "reversal"] {
            assert_eq!(audience_for(t, None).unwrap(), Audience::Finance, "{t}");
        }
    }

    #[test]
    fn class_tables_need_a_class_and_map_to_class() {
        for t in [
            "attendance_sheet",
            "attendance_mark",
            "marks_sheet",
            "mark_entry",
            "exam",
            "exam_subject",
        ] {
            assert_eq!(
                audience_for(t, Some("cls-1")).unwrap(),
                Audience::Class("cls-1".into()),
                "{t}"
            );
            // Missing class id is a validation error, not a silent admin bundle.
            assert!(matches!(
                audience_for(t, None),
                Err(CoreError::Validation { .. })
            ));
        }
    }

    #[test]
    fn admin_tables_map_to_admin() {
        for t in [
            "school",
            "academic_session",
            "term",
            "subject",
            "class",
            "class_subject",
            "staff",
            "device",
            "invite",
            "conflict",
            "review_flag",
            "notification",
            "licence",
            "grade_scale",
            "grade_band",
            "school_week",
            "calendar_event",
        ] {
            assert_eq!(audience_for(t, None).unwrap(), Audience::Admin, "{t}");
        }
    }

    #[test]
    fn student_roster_maps_to_finance() {
        // [OWNER default, P06]: pusher-consistent — accountants + Principal hold finance.
        for t in ["student", "enrollment", "guardian", "student_guardian"] {
            assert_eq!(audience_for(t, None).unwrap(), Audience::Finance, "{t}");
        }
    }

    #[test]
    fn request_audience_is_requester_scoped_not_table_only() {
        // request is sealed by whoever raised it → decided at the write site, not here.
        let e = audience_for("request", Some("cls-1")).unwrap_err();
        assert_eq!(e.code(), "VALIDATION");
    }

    #[test]
    fn unknown_table_errors() {
        assert!(matches!(
            audience_for("nope", None),
            Err(CoreError::Validation { .. })
        ));
    }

    #[test]
    fn string_form_round_trips() {
        let cases = [
            (Audience::Admin, "admin"),
            (Audience::Finance, "finance"),
            (Audience::Class("cls-7".into()), "class:cls-7"),
        ];
        for (a, s) in cases {
            assert_eq!(a.as_str(), s);
            assert_eq!(Audience::parse(s).unwrap(), a);
        }
    }

    #[test]
    fn parse_rejects_junk() {
        for s in ["", "class:", "teacher", "class"] {
            assert!(Audience::parse(s).is_err(), "{s}");
        }
    }
}
