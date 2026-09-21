//! Fee-account maths: turns an enrollment plus the session's plan into due,
//! paid, balance, state and the one-term amounts the collect screen offers.
//! Pure reads (no permission checks); the service authorizes before calling.

use rusqlite::Connection;
use vidya_core::fees::{self, FeeInputs, FeePlan, FeeState};
use vidya_db::repo::sessions::SessionRow;
use vidya_db::repo::students::EnrolledStudent;
use vidya_db::repo::{enrollments, fee_plans, receipts, sessions};
use vidya_db::DbError;

/// One student's fee position for a session.
#[derive(Debug, Clone)]
pub struct FeeAccount {
    pub due: i64,
    pub paid: i64,
    pub balance: i64,
    pub state: FeeState,
    /// One term of class fees, transport excluded.
    pub term_fee: i64,
    /// One term including transport when the student uses the bus.
    pub one_term: i64,
    /// What "one term" fills into the amount box (never above the balance).
    pub one_term_payable: i64,
    pub previous_session_dues: i64,
}

/// The fee plan for a class in a session, or all-zero when none is set.
pub fn plan_for(conn: &Connection, session_id: &str, class_id: &str) -> Result<FeePlan, DbError> {
    Ok(fee_plans::list_for_session(conn, session_id)?
        .into_iter()
        .find(|p| p.class_id == class_id)
        .map_or(
            FeePlan {
                tuition: 0,
                exam: 0,
                other: 0,
            },
            |p| FeePlan {
                tuition: p.tuition,
                exam: p.exam,
                other: p.other,
            },
        ))
}

fn inputs(session: &SessionRow, plan: FeePlan, rte: bool, transport: bool, concession: i64) -> FeeInputs {
    FeeInputs {
        plan,
        terms: session.terms as u32,
        transport_fee_per_term: session.transport_fee_per_term,
        transport,
        rte,
        concession,
    }
}

/// Computes the full fee account for an enrolled student.
pub fn fee_account(
    conn: &Connection,
    session: &SessionRow,
    e: &EnrolledStudent,
) -> Result<FeeAccount, DbError> {
    let plan = plan_for(conn, &session.id, &e.class_id)?;
    let inp = inputs(session, plan, e.rte, e.transport, e.concession);
    let due = fees::session_due(&inp);
    let paid = receipts::sum_paid(conn, &session.id, &e.student.id)?;
    let balance = fees::balance(due, paid);
    let one_term = fees::one_term_amount(&inp);
    Ok(FeeAccount {
        due,
        paid,
        balance,
        state: fees::fee_state(due, paid, e.rte),
        term_fee: fees::term_fee(&plan),
        one_term,
        one_term_payable: balance.min(one_term).max(0),
        previous_session_dues: previous_dues(conn, session, &e.student.id)?,
    })
}

/// The unpaid balance carried from the session before this one, or 0.
fn previous_dues(conn: &Connection, session: &SessionRow, student_id: &str) -> Result<i64, DbError> {
    let Some(prev) = sessions::previous(conn, &session.starts_on)? else {
        return Ok(0);
    };
    let Some(enr) = enrollments::get_for_student(conn, student_id, &prev.id)? else {
        return Ok(0);
    };
    let plan = plan_for(conn, &prev.id, &enr.class_id)?;
    let inp = inputs(&prev, plan, enr.rte, enr.transport, enr.concession);
    let due = fees::session_due(&inp);
    let paid = receipts::sum_paid(conn, &prev.id, student_id)?;
    Ok(fees::balance(due, paid))
}
