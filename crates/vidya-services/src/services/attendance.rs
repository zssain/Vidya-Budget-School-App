//! Attendance per section and date. Teachers mark their own sections for today;
//! the principal may correct past days. Every save replaces the whole day's
//! marks in one transaction and appends a `replace_set` change-log entry.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;

use vidya_core::dates::parse_date;
use vidya_core::error::ErrorKind;
use vidya_core::i18n;
use vidya_core::permissions::{self, Access, Action, Scope};
use vidya_core::roles::Actor;
use vidya_db::repo::attendance::AttendanceDayRow;
use vidya_db::repo::sessions::SessionRow;
use vidya_db::repo::students::EnrolledStudent;
use vidya_db::repo::{attendance, classes, sessions, students, users};

use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::Services;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// ---------- DTOs ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceStudentDto {
    pub id: String,
    pub adm: String,
    pub roll: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceSheetDto {
    pub section_id: String,
    pub section_label: String,
    pub date: String,
    pub students: Vec<AttendanceStudentDto>,
    pub marks: HashMap<String, String>,
    pub saved_by: Option<String>,
    pub saved_at: Option<String>,
    pub read_only_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRowDto {
    pub adm: String,
    pub roll: i64,
    pub name: String,
    pub cells: Vec<String>,
    pub present: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDto {
    pub section_id: String,
    pub section_label: String,
    pub year: i32,
    pub month: u32,
    pub month_name: String,
    pub days: u32,
    pub rows: Vec<RegisterRowDto>,
    pub school: SchoolName,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySectionDto {
    pub section_id: String,
    pub section_label: String,
    pub marked: bool,
    pub present: i64,
    pub absent: i64,
    pub leave: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAttendanceInput {
    pub section_id: String,
    pub date: String,
    pub marks: HashMap<String, String>,
}

pub struct AttendanceService<'a> {
    services: &'a Services,
}

fn can_edit_past(actor: &Actor, section_id: &str) -> bool {
    match permissions::access(actor.role, Action::AttendanceEditPast) {
        Access::Yes => true,
        Access::Own => actor.section_ids.contains(section_id),
        Access::No => false,
    }
}

fn valid_status(status: &str) -> bool {
    matches!(status, "P" | "A" | "L")
}

impl<'a> AttendanceService<'a> {
    pub fn new(services: &'a Services) -> Self {
        Self { services }
    }

    fn session(&self) -> Result<SessionRow, ServiceError> {
        self.services
            .db
            .read(sessions::current)?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "session.error.none"))
    }

    fn msg(&self, actor: &Actor, key: &str) -> String {
        i18n::message(actor.lang, key, &BTreeMap::new())
    }

    fn section_label(&self, section_id: &str) -> Result<String, ServiceError> {
        Ok(self
            .services
            .db
            .read(|c| classes::section_label(c, section_id))?
            .unwrap_or_default())
    }

    /// Active students of a section in the current session, ordered by roll.
    fn section_students(
        &self,
        session: &SessionRow,
        section_id: &str,
    ) -> Result<Vec<EnrolledStudent>, ServiceError> {
        let session_id = session.id.clone();
        let mut students: Vec<EnrolledStudent> = self
            .services
            .db
            .read(|c| students::list_enrolled(c, &session_id, "active"))?
            .into_iter()
            .filter(|e| e.section_id == section_id)
            .collect();
        students.sort_by_key(|e| e.roll);
        Ok(students)
    }

    pub fn sheet(
        &self,
        actor: &Actor,
        section_id: &str,
        date: &str,
    ) -> Result<AttendanceSheetDto, ServiceError> {
        permissions::authorize_section(actor, Action::AttendanceView, section_id)?;
        let session = self.session()?;
        let today = self.services.clock.today_local();
        let day = parse_date(date)?;
        let students = self.section_students(&session, section_id)?;

        let section = section_id.to_owned();
        let (marks, saved_by, saved_at) =
            self.services
                .db
                .read(|c| match attendance::get_day(c, &section, date)? {
                    Some(day) => {
                        let marks = attendance::marks_for_day(c, &day.id)?;
                        let name = users::get_by_id(c, &day.saved_by)?.map(|u| u.name);
                        Ok((marks, name, Some(day.saved_at)))
                    }
                    None => Ok((Vec::new(), None, None)),
                })?;

        let read_only_reason = if day > today {
            Some(self.msg(actor, "attendance.error.future"))
        } else if day < today && !can_edit_past(actor, section_id) {
            Some(self.msg(actor, "attendance.error.past_read_only"))
        } else {
            None
        };

        Ok(AttendanceSheetDto {
            section_id: section_id.to_owned(),
            section_label: self.section_label(section_id)?,
            date: date.to_owned(),
            students: students
                .iter()
                .map(|e| AttendanceStudentDto {
                    id: e.student.id.clone(),
                    adm: e.student.adm_no.clone(),
                    roll: e.roll,
                    name: e.student.name.clone(),
                })
                .collect(),
            marks: marks.into_iter().collect(),
            saved_by,
            saved_at,
            read_only_reason,
        })
    }

    pub fn save(
        &self,
        actor: &Actor,
        input: SaveAttendanceInput,
    ) -> Result<AttendanceSheetDto, ServiceError> {
        let session = self.session()?;
        let today = self.services.clock.today_local();
        let day_date = parse_date(&input.date)?;
        if day_date > today {
            return Err(ServiceError::validation("attendance.error.future").field("date"));
        }
        if day_date == today {
            permissions::authorize_section(actor, Action::AttendanceMarkToday, &input.section_id)?;
        } else {
            permissions::authorize_section(actor, Action::AttendanceEditPast, &input.section_id)?;
        }

        let students = self.section_students(&session, &input.section_id)?;
        let active_ids: BTreeSet<&str> = students.iter().map(|e| e.student.id.as_str()).collect();
        for (id, status) in &input.marks {
            if !valid_status(status) {
                return Err(ServiceError::validation("attendance.error.invalid_status").field("marks"));
            }
            if !active_ids.contains(id.as_str()) {
                return Err(ServiceError::validation("attendance.error.not_in_section").field("marks"));
            }
        }
        let missing: Vec<&EnrolledStudent> = students
            .iter()
            .filter(|e| !input.marks.contains_key(&e.student.id))
            .collect();
        if !missing.is_empty() {
            let mut names = missing
                .iter()
                .take(3)
                .map(|e| e.student.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            if missing.len() > 3 {
                names.push('…');
            }
            return Err(ServiceError::validation("attendance.error.not_marked")
                .field("marks")
                .param("count", missing.len().to_string())
                .param("names", names));
        }

        let count = |k: &str| {
            students
                .iter()
                .filter(|e| input.marks.get(&e.student.id).map(String::as_str) == Some(k))
                .count() as i64
        };
        let (present, absent, leave) = (count("P"), count("A"), count("L"));

        let existing = self
            .services
            .db
            .read(|c| attendance::get_day(c, &input.section_id, &input.date))?;
        let existed = existing.is_some();
        let day_id = existing.map_or_else(|| self.services.ids.new_id(), |d| d.id);
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        let section_label = self.section_label(&input.section_id)?;
        let summary_key = if existed {
            "attendance.log.corrected"
        } else {
            "attendance.log.saved"
        };
        let session_id = session.id.clone();

        self.services.db.write(|tx| {
            if existed {
                attendance::update_day(tx, &day_id, &actor.user_id, &now, &hlc)?;
                attendance::delete_marks(tx, &day_id)?;
            } else {
                attendance::insert_day(
                    tx,
                    &AttendanceDayRow {
                        id: day_id.clone(),
                        session_id: session_id.clone(),
                        section_id: input.section_id.clone(),
                        date: input.date.clone(),
                        saved_by: actor.user_id.clone(),
                        saved_at: now.clone(),
                    },
                    &hlc,
                )?;
            }
            for e in &students {
                attendance::insert_mark(tx, &day_id, &e.student.id, &input.marks[&e.student.id])?;
            }
            write_entry(
                self.services,
                tx,
                Some(actor),
                ChangeRecord {
                    kind: "att",
                    entity: "attendance_day",
                    entity_id: &day_id,
                    op: Op::ReplaceSet,
                    summary_key,
                    params: json!({
                        "section": section_label,
                        "date": input.date,
                        "present": present,
                        "absent": absent,
                        "leave": leave,
                    }),
                    payload: json!({}),
                },
            )?;
            Ok(())
        })?;
        self.sheet(actor, &input.section_id, &input.date)
    }

    pub fn register(
        &self,
        actor: &Actor,
        section_id: &str,
        month: &str,
    ) -> Result<RegisterDto, ServiceError> {
        permissions::authorize_section(actor, Action::AttendancePrintRegister, section_id)?;
        let session = self.session()?;
        let (year, m) = parse_month(month)?;
        let days = days_in_month(year, m);
        let students = self.section_students(&session, section_id)?;
        let section = section_id.to_owned();
        let month_owned = month.to_owned();
        let marks = self
            .services
            .db
            .read(|c| attendance::month_marks(c, &section, &month_owned))?;
        // (student_id, date) -> status
        let mut by_cell: HashMap<(String, String), String> = HashMap::new();
        for (date, student_id, status) in marks {
            by_cell.insert((student_id, date), status);
        }
        let school_name = self
            .services
            .db
            .read(vidya_db::repo::school::get)?
            .map(|s| s.name)
            .unwrap_or_default();

        let rows = students
            .iter()
            .map(|e| {
                let mut present = 0;
                let cells = (1..=days)
                    .map(|d| {
                        let date = format!("{year}-{m:02}-{d:02}");
                        let status = by_cell
                            .get(&(e.student.id.clone(), date))
                            .cloned()
                            .unwrap_or_default();
                        if status == "P" {
                            present += 1;
                        }
                        status
                    })
                    .collect();
                RegisterRowDto {
                    adm: e.student.adm_no.clone(),
                    roll: e.roll,
                    name: e.student.name.clone(),
                    cells,
                    present,
                }
            })
            .collect();

        Ok(RegisterDto {
            section_id: section_id.to_owned(),
            section_label: self.section_label(section_id)?,
            year,
            month: m,
            month_name: MONTHS[(m - 1) as usize].to_owned(),
            days,
            rows,
            school: SchoolName { name: school_name },
        })
    }

    /// Per-section attendance state for today, for the home screens.
    pub fn today_summary(&self, actor: &Actor) -> Result<Vec<TodaySectionDto>, ServiceError> {
        let scope = permissions::scope(actor, Action::AttendanceView)?;
        let today = self.services.clock.today_local().to_string();
        let section_ids: Vec<String> = match scope {
            Scope::All => self.services.db.read(|c| {
                let mut ids = Vec::new();
                for class in classes::list_active_classes(c)? {
                    for section in classes::list_active_sections(c, &class.id)? {
                        ids.push(section.id);
                    }
                }
                Ok(ids)
            })?,
            Scope::Sections(set) => set.into_iter().collect(),
        };

        let mut out = Vec::with_capacity(section_ids.len());
        for section_id in section_ids {
            let label = self.section_label(&section_id)?;
            let counts = self
                .services
                .db
                .read(|c| match attendance::get_day(c, &section_id, &today)? {
                    Some(day) => {
                        let marks = attendance::marks_for_day(c, &day.id)?;
                        let count = |k: &str| marks.iter().filter(|(_, s)| s == k).count() as i64;
                        Ok(Some((count("P"), count("A"), count("L"))))
                    }
                    None => Ok(None),
                })?;
            let (marked, present, absent, leave) = match counts {
                Some((p, a, l)) => (true, p, a, l),
                None => (false, 0, 0, 0),
            };
            out.push(TodaySectionDto {
                section_id,
                section_label: label,
                marked,
                present,
                absent,
                leave,
            });
        }
        Ok(out)
    }
}

fn parse_month(month: &str) -> Result<(i32, u32), ServiceError> {
    let bad = || ServiceError::validation("attendance.error.month").field("month");
    let (y, m) = month.split_once('-').ok_or_else(bad)?;
    let year: i32 = y.parse().map_err(|_| bad())?;
    let m: u32 = m.parse().map_err(|_| bad())?;
    if !(1..=12).contains(&m) {
        return Err(bad());
    }
    Ok((year, m))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(31, |d| d.day())
}
