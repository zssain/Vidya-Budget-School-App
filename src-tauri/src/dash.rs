//! Dashboard aggregate queries (prompts/P03 Step 8, docs §earlier spec). Pure DB
//! reads that return raw numbers; the frontend formats them (₹ lakh grouping,
//! percentages) with the tested `format.ts`, so the mock pixels come from one
//! formatter. `today` is passed in (RFC-3339 midnight handling / testability).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;

use vidya_core::attendance::percent_present;

/// Latest pending approval row (Principal Home + Approvals list).
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalRow {
    pub id: String,
    /// Request type (`marks_correction`, `payment_reversal`, …) → badge + label.
    pub kind: String,
    /// The one-line "what" summary (composed at seed/creation time).
    pub what: String,
    /// "Anita Rao, Teacher".
    pub who: String,
    /// RFC-3339 created_at; the frontend renders "2 hours ago".
    pub created_at: String,
}

/// One "Attendance by class" row (pct None when the sheet is still pending).
#[derive(Debug, Clone, Serialize)]
pub struct ClassPct {
    pub name: String,
    pub pct: Option<i64>,
}

/// A class whose attendance sheet is still a draft today — its display name and
/// the class teacher's name (None if the class has no class teacher). Drives the
/// Home "Needs attention" line "Class teacher <name> · not submitted yet today".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingClass {
    pub class: String,
    pub teacher: Option<String>,
}

/// One "Fee collection" day column (value in paise).
#[derive(Debug, Clone, Serialize)]
pub struct FeeDay {
    pub day: String,
    pub value_paise: i64,
}

/// Everything the Principal Home dashboard needs (raw numbers; UI formats).
#[derive(Debug, Clone, Serialize)]
pub struct PrincipalDashboard {
    // Stat 1 — attendance today.
    pub attendance_pct_tenths: u32,
    pub attendance_marked: i64,
    pub attendance_total: i64,
    pub attendance_pending: Vec<PendingClass>,
    // Stat 2 — collected today.
    pub collected_today_paise: i64,
    pub receipts_today: i64,
    pub waiting_paise: i64,
    // Stat 3 — outstanding.
    pub outstanding_paise: i64,
    pub students_with_dues: i64,
    // Stat 4 — students.
    pub active_students: i64,
    pub admissions_this_week: i64,
    // Lists.
    pub approvals: Vec<ApprovalRow>,
    pub approvals_total: i64,
    pub classes: Vec<ClassPct>,
    pub fee_days: Vec<FeeDay>,
    pub fee_total_paise: i64,
    // Narrative context (drives the greeting/date/needs/backup — real, not fixture).
    /// Current term name by `today` (None outside any term range).
    pub term_label: Option<String>,
    /// Open edit conflicts awaiting the Principal's review.
    pub open_conflicts: i64,
    /// The most recent completed backup run, or None if none has run.
    pub last_backup: Option<LastBackupInfo>,
    /// Home "Needs attention": no school **sync** account is connected yet (P12
    /// Step 1.2) — "Connect the school sync account".
    pub needs_sync_account: bool,
}

/// The latest completed backup, for the "Last backup" line (honest status §3.13).
#[derive(Debug, Clone, Serialize)]
pub struct LastBackupInfo {
    /// RFC-3339 finished_at; the frontend renders it relatively.
    pub at: String,
    pub status: String,
    pub destination: Option<String>,
}

/// Current term name for `today` (YYYY-MM-DD) in the current session, if any.
pub fn current_term(conn: &Connection, today: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT t.name FROM term t JOIN academic_session s ON s.id = t.session_id \
         WHERE s.is_current = 1 AND date(?1) BETWEEN date(t.starts_on) AND date(t.ends_on) \
         ORDER BY t.starts_on LIMIT 1",
        params![today],
        |r| r.get::<_, String>(0),
    )
    .optional()
}

/// Count of open edit conflicts (Principal Home "Needs attention").
pub fn open_conflicts(conn: &Connection) -> rusqlite::Result<i64> {
    count(conn, "SELECT COUNT(*) FROM conflict WHERE status='open'", &[])
}

/// The most recent completed backup run.
pub fn last_backup(conn: &Connection) -> rusqlite::Result<Option<LastBackupInfo>> {
    conn.query_row(
        "SELECT finished_at, status, destination FROM backup_run \
         WHERE finished_at IS NOT NULL ORDER BY finished_at DESC LIMIT 1",
        [],
        |r| Ok(LastBackupInfo { at: r.get(0)?, status: r.get(1)?, destination: r.get(2)? }),
    )
    .optional()
}

fn count(conn: &Connection, sql: &str, p: &[&dyn rusqlite::ToSql]) -> rusqlite::Result<i64> {
    conn.query_row(sql, p, |r| r.get::<_, i64>(0))
}

pub fn active_students(conn: &Connection) -> rusqlite::Result<i64> {
    count(conn, "SELECT COUNT(*) FROM student WHERE status='active'", &[])
}

/// Students admitted in the 7 days up to and including `today` (YYYY-MM-DD).
pub fn admissions_this_week(conn: &Connection, today: &str) -> rusqlite::Result<i64> {
    count(
        conn,
        "SELECT COUNT(*) FROM student WHERE status='active' \
         AND date(created_at) > date(?1, '-7 days') AND date(created_at) <= date(?1)",
        &[&today],
    )
}

/// Confirmed collections for `today`: (sum_paise, receipt_count).
pub fn collected_today(conn: &Connection, today: &str) -> rusqlite::Result<(i64, i64)> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_paise),0), COUNT(*) FROM payment \
         WHERE date(collected_at)=date(?1) AND sync_state='confirmed'",
        params![today],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )
}

/// Payments recorded but not yet confirmed by the server ("waiting for server").
pub fn waiting_paise(conn: &Connection) -> rusqlite::Result<i64> {
    count(
        conn,
        "SELECT COALESCE(SUM(amount_paise),0) FROM payment WHERE sync_state <> 'confirmed'",
        &[],
    )
}

/// Outstanding across all uncancelled dues: total dues − total 'due' allocations.
pub fn outstanding(conn: &Connection) -> rusqlite::Result<i64> {
    let dues: i64 = count(
        conn,
        "SELECT COALESCE(SUM(amount_paise),0) FROM fee_due WHERE cancelled_at IS NULL",
        &[],
    )?;
    let alloc: i64 = count(
        conn,
        "SELECT COALESCE(SUM(pa.amount_paise),0) FROM payment_allocation pa \
         JOIN fee_due d ON d.id = pa.fee_due_id WHERE pa.kind='due' AND d.cancelled_at IS NULL",
        &[],
    )?;
    Ok(dues - alloc)
}

/// Number of students whose uncancelled dues still have a positive balance.
pub fn students_with_dues(conn: &Connection) -> rusqlite::Result<i64> {
    count(
        conn,
        "SELECT COUNT(*) FROM ( \
           SELECT d.student_id, \
             SUM(d.amount_paise) - COALESCE(SUM(CASE WHEN pa.kind='due' THEN pa.amount_paise END),0) AS bal \
           FROM fee_due d \
           LEFT JOIN payment_allocation pa ON pa.fee_due_id = d.id AND pa.kind='due' \
           WHERE d.cancelled_at IS NULL \
           GROUP BY d.student_id HAVING bal > 0 )",
        &[],
    )
}

/// Attendance totals for a day plus the per-class breakdown.
#[derive(Debug, Clone)]
pub struct AttendanceToday {
    pub present: u32,
    pub absent: u32,
    pub leave: u32,
    pub marked: i64,
    pub classes: Vec<ClassPct>,
}

/// Attendance for `today`: overall present/absent/leave/marked + per-class rows.
/// Per-class pct is `None` when that class's sheet is still `draft` (pending).
pub fn attendance_today(conn: &Connection, today: &str) -> rusqlite::Result<AttendanceToday> {
    let mut stmt = conn.prepare(
        "SELECT c.display, c.sort_order, s.status, m.mark \
         FROM attendance_sheet s \
         JOIN class c ON c.id = s.class_id \
         LEFT JOIN attendance_mark m ON m.sheet_id = s.id \
         WHERE date(s.date)=date(?1)",
    )?;
    // Per-class accumulator keyed by class display name.
    #[derive(Default)]
    struct Acc {
        sort_order: i64,
        status: String,
        p: u32,
        a: u32,
        l: u32,
    }
    let mut per: BTreeMap<String, Acc> = BTreeMap::new();
    let mut rows = stmt.query(params![today])?;
    let (mut tp, mut ta, mut tl) = (0u32, 0u32, 0u32);
    while let Some(row) = rows.next()? {
        let display: String = row.get(0)?;
        let sort_order: i64 = row.get(1)?;
        let status: String = row.get(2)?;
        let mark: Option<String> = row.get(3)?;
        let e = per.entry(display).or_default();
        e.sort_order = sort_order;
        e.status = status;
        match mark.as_deref() {
            Some("P") => { e.p += 1; tp += 1; }
            Some("A") => { e.a += 1; ta += 1; }
            Some("L") => { e.l += 1; tl += 1; }
            _ => {}
        }
    }
    // Order classes by sort_order for display.
    let mut ordered: Vec<(String, Acc)> = per.into_iter().collect();
    ordered.sort_by_key(|(_, v)| v.sort_order);
    let classes = ordered
        .into_iter()
        .map(|(name, acc)| {
            let marked = acc.p + acc.a + acc.l;
            let pct = if acc.status == "draft" || marked == 0 {
                None
            } else {
                // Class row shows an integer %: round(tenths/10) half-up.
                Some(((percent_present(acc.p, acc.a, acc.l) + 5) / 10) as i64)
            };
            ClassPct { name, pct }
        })
        .collect();
    let marked = (tp + ta + tl) as i64;
    Ok(AttendanceToday { present: tp, absent: ta, leave: tl, marked, classes })
}

/// Classes with a `draft` (pending) attendance sheet today, each with its class
/// teacher's name (§5: only the class teacher takes attendance for a class).
pub fn attendance_pending(conn: &Connection, today: &str) -> rusqlite::Result<Vec<PendingClass>> {
    // No attendance is expected on a non-working day (P13 calendar backbone): a
    // weekly off or a holiday means dashboards list nothing as "not submitted".
    if !crate::calendar::is_working_day(conn, today)? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT c.display, st.name \
         FROM attendance_sheet s JOIN class c ON c.id=s.class_id \
         LEFT JOIN staff st ON st.id=c.class_teacher_id \
         WHERE date(s.date)=date(?1) AND s.status='draft' ORDER BY c.sort_order",
    )?;
    let rows = stmt.query_map(params![today], |r| {
        Ok(PendingClass { class: r.get(0)?, teacher: r.get(1)? })
    })?;
    rows.collect()
}

/// The six *school* dates up to and including `today` (Sundays skipped),
/// oldest → newest. Shared by the dashboard query and the demo seed.
pub fn fee_collection_days_dates(today: time::Date) -> Vec<time::Date> {
    use time::Weekday;
    let mut days: Vec<time::Date> = Vec::new();
    let mut d = today;
    while days.len() < 6 {
        if d.weekday() != Weekday::Sunday {
            days.push(d);
        }
        match d.previous_day() {
            Some(prev) => d = prev,
            None => break,
        }
    }
    days.reverse(); // oldest → newest
    days
}

/// The last six *school* days up to `today` (Sundays skipped): (label, paise).
/// Labels are weekday short names, with the most recent shown as "Today".
pub fn fee_collection_days(conn: &Connection, today: &str) -> rusqlite::Result<Vec<FeeDay>> {
    use time::Date;
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    let today_date = Date::parse(today, fmt).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let days = fee_collection_days_dates(today_date);
    let mut out = Vec::with_capacity(6);
    for date in days {
        let ds = date.format(fmt).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let sum: i64 = count(
            conn,
            "SELECT COALESCE(SUM(amount_paise),0) FROM payment \
             WHERE date(collected_at)=date(?1) AND sync_state='confirmed'",
            &[&ds],
        )?;
        let label = if date == today_date {
            "Today".to_string()
        } else {
            weekday_short(date.weekday()).to_string()
        };
        out.push(FeeDay { day: label, value_paise: sum });
    }
    Ok(out)
}

fn weekday_short(w: time::Weekday) -> &'static str {
    use time::Weekday::*;
    match w {
        Monday => "Mon",
        Tuesday => "Tue",
        Wednesday => "Wed",
        Thursday => "Thu",
        Friday => "Fri",
        Saturday => "Sat",
        Sunday => "Sun",
    }
}

/// Pending approvals: the latest `limit` rows + the total pending count.
pub fn pending_approvals(conn: &Connection, limit: i64) -> rusqlite::Result<(Vec<ApprovalRow>, i64)> {
    let total = count(conn, "SELECT COUNT(*) FROM request WHERE status='pending'", &[])?;
    let mut stmt = conn.prepare(
        "SELECT r.id, r.type, COALESCE(r.after_json,''), s.name, s.role, r.created_at \
         FROM request r JOIN staff s ON s.id = r.requested_by \
         WHERE r.status='pending' ORDER BY r.created_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let kind: String = row.get(1)?;
            let after: String = row.get(2)?;
            let name: String = row.get(3)?;
            let role: String = row.get(4)?;
            // The "what" summary is stored in after_json.summary at creation time.
            let what = serde_json::from_str::<serde_json::Value>(&after)
                .ok()
                .and_then(|v| v.get("summary").and_then(|s| s.as_str()).map(str::to_string))
                .unwrap_or_default();
            Ok(ApprovalRow {
                id: row.get(0)?,
                kind,
                what,
                who: format!("{name}, {}", role_title(&role)),
                created_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((rows, total))
}

fn role_title(role: &str) -> &'static str {
    match role {
        "principal" => "Principal",
        "accountant" => "Accountant",
        _ => "Teacher",
    }
}

/// Assemble the full Principal Home dashboard.
pub fn principal_dashboard(conn: &Connection, today: &str) -> rusqlite::Result<PrincipalDashboard> {
    let att = attendance_today(conn, today)?;
    let (collected, receipts) = collected_today(conn, today)?;
    let (approvals, approvals_total) = pending_approvals(conn, 4)?;
    let fee_days = fee_collection_days(conn, today)?;
    let fee_total_paise = fee_days.iter().map(|d| d.value_paise).sum();
    Ok(PrincipalDashboard {
        attendance_pct_tenths: percent_present(att.present, att.absent, att.leave),
        attendance_marked: att.marked,
        attendance_total: active_students(conn)?,
        attendance_pending: attendance_pending(conn, today)?,
        collected_today_paise: collected,
        receipts_today: receipts,
        waiting_paise: waiting_paise(conn)?,
        outstanding_paise: outstanding(conn)?,
        students_with_dues: students_with_dues(conn)?,
        active_students: active_students(conn)?,
        admissions_this_week: admissions_this_week(conn, today)?,
        approvals,
        approvals_total,
        classes: att.classes,
        fee_days,
        fee_total_paise,
        term_label: current_term(conn, today)?,
        open_conflicts: open_conflicts(conn)?,
        last_backup: last_backup(conn)?,
        needs_sync_account: crate::drive_account::needs_sync_account(conn)?,
    })
}
