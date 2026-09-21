//! Fees: register, fee account, collection with append-only receipts,
//! per-device receipt series, cancellation by the principal, and the day book.
//! Every write authorizes, validates with `vidya-core`, runs one transaction
//! and appends a change-log entry. Receipts are never updated or deleted.

pub mod account;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use vidya_core::dates::parse_date;
use vidya_core::error::ErrorKind;
use vidya_core::fees::{
    self, balance as fee_balance, receipt_number, validate_payment, FeeState, PayMode, PaymentCheck,
};
use vidya_core::permissions::{self, Action};
use vidya_core::roles::{Actor, Role};
use vidya_core::validation::validate_reason;
use vidya_core::words::receipt_words_en;
use vidya_db::repo::alerts::AlertRow;
use vidya_db::repo::receipts::{ReceiptDetail, ReceiptRow};
use vidya_db::repo::sessions::SessionRow;
use vidya_db::repo::students::EnrolledStudent;
use vidya_db::repo::{alerts, cancellations, meta, receipts, school, sessions, students};
use vidya_db::DbError;

use self::account::fee_account;
use crate::change_log::{write_entry, ChangeRecord, Op};
use crate::error::ServiceError;
use crate::{Mode, Services};

// ---------- DTOs ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeTotalsDto {
    pub due: i64,
    pub paid: i64,
    pub pending: i64,
    pub unpaid: i64,
    pub part: i64,
    pub paying_count: i64,
    pub pct_collected: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeRegisterRowDto {
    pub id: String,
    pub adm: String,
    pub name: String,
    pub ck: String,
    pub father: String,
    pub mobile: String,
    pub rte: bool,
    pub due: i64,
    pub paid: i64,
    pub balance: i64,
    pub fee_state: FeeState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeRegisterDto {
    pub totals: FeeTotalsDto,
    pub collected_today: i64,
    pub receipts_today: i64,
    pub session: String,
    pub terms: i64,
    pub device_prefix: String,
    pub list: Vec<FeeRegisterRowDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptSummaryDto {
    pub id: String,
    pub no: String,
    pub date: String,
    pub amount: i64,
    pub mode: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeAccountDto {
    pub student_id: String,
    pub name: String,
    pub ck: String,
    pub roll: i64,
    pub rte: bool,
    pub status: String,
    pub due: i64,
    pub paid: i64,
    pub balance: i64,
    pub term_fee: i64,
    pub terms: i64,
    pub transport: bool,
    pub transport_fee: i64,
    pub concession: i64,
    pub one_term: i64,
    pub one_term_payable: i64,
    pub previous_session_dues: i64,
    pub receipts: Vec<ReceiptSummaryDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolHeaderLite {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelInfoDto {
    pub by_name: String,
    pub at: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptDto {
    pub no: String,
    pub id: String,
    pub adm: String,
    pub student_name: String,
    pub ck: String,
    pub roll: i64,
    pub father: String,
    pub amount: i64,
    pub mode: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub note: String,
    pub date: String,
    pub at: String,
    pub by_name: String,
    pub device: String,
    pub balance_after: i64,
    pub amount_words: String,
    pub school: SchoolHeaderLite,
    pub cancelled: Option<CancelInfoDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayModeDto {
    pub mode: String,
    pub total: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayBookDto {
    pub date: String,
    pub modes: Vec<DayModeDto>,
    pub total: i64,
    pub count: i64,
    pub cancelled_count: i64,
    pub receipts: Vec<ReceiptDto>,
    pub school: SchoolHeaderLite,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertDto {
    pub id: String,
    pub kind: String,
    pub message_key: String,
    pub params: serde_json::Value,
    pub entity: String,
    pub entity_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeFilter {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectInput {
    pub student_id: String,
    pub amount: i64,
    pub mode: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub note: String,
}

// ---------- Service ----------

pub struct FeeService<'a> {
    services: &'a Services,
}

/// Computed fee position for one enrollment: `(due, paid, balance, state)`.
type FeeCalc = (i64, i64, i64, FeeState);

fn ck(e: &EnrolledStudent) -> String {
    format!("{}-{}", e.class_name, e.section_name)
}

fn parse_mode(mode: &str) -> Result<PayMode, ServiceError> {
    match mode {
        "Cash" => Ok(PayMode::Cash),
        "UPI" => Ok(PayMode::Upi),
        "Cheque" => Ok(PayMode::Cheque),
        _ => Err(ServiceError::validation("fees.error.mode").field("mode")),
    }
}

fn mode_str(mode: PayMode) -> &'static str {
    match mode {
        PayMode::Cash => "Cash",
        PayMode::Upi => "UPI",
        PayMode::Cheque => "Cheque",
    }
}

fn receipt_dto(detail: ReceiptDetail, school_name: String) -> ReceiptDto {
    let amount_words = receipt_words_en(detail.amount.max(0) as u64);
    ReceiptDto {
        no: detail.receipt_no,
        id: detail.id,
        adm: detail.adm_no,
        student_name: detail.student_name,
        ck: format!("{}-{}", detail.class_name, detail.section_name),
        roll: detail.roll,
        father: detail.father,
        amount: detail.amount,
        mode: detail.mode,
        reference: detail.reference,
        note: detail.note,
        date: detail.paid_on,
        at: detail.created_at,
        by_name: detail.received_by_name,
        device: detail.device_code,
        balance_after: detail.balance_after,
        amount_words,
        school: SchoolHeaderLite { name: school_name },
        cancelled: detail.cancelled.map(|c| CancelInfoDto {
            by_name: c.by_name,
            at: c.at,
            reason: c.reason,
        }),
    }
}

impl<'a> FeeService<'a> {
    pub fn new(services: &'a Services) -> Self {
        Self { services }
    }

    fn session(&self) -> Result<SessionRow, ServiceError> {
        self.services
            .db
            .read(sessions::current)?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "session.error.none"))
    }

    fn school_name(&self) -> Result<String, ServiceError> {
        Ok(self
            .services
            .db
            .read(school::get)?
            .map(|s| s.name)
            .unwrap_or_default())
    }

    /// The receipt-series prefix for this device (office computer reads
    /// `meta.device_code`, defaulting to `PC`).
    fn device_code(&self) -> Result<String, ServiceError> {
        Ok(self
            .services
            .db
            .read(|c| meta::get(c, "device_code"))?
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "PC".to_owned()))
    }

    pub fn register(&self, actor: &Actor, filter: FeeFilter) -> Result<FeeRegisterDto, ServiceError> {
        permissions::authorize(actor, Action::FeesView)?;
        let session = self.session()?;
        let session_id = session.id.clone();
        let today = self.services.clock.today_local().to_string();
        // Fee position for every active student (totals ignore the filters).
        let (computed, collected) = self.services.db.read(|c| {
            let rows = students::list_enrolled(c, &session_id, "active")?;
            let map = Self::plan_inputs(c, &session, &rows)?;
            let collected = receipts::collected_on(c, &today)?;
            let computed: Vec<(EnrolledStudent, i64, i64, i64, FeeState)> = rows
                .into_iter()
                .map(|e| {
                    let (due, paid, bal, state) = map[&e.enrollment_id];
                    (e, due, paid, bal, state)
                })
                .collect();
            Ok((computed, collected))
        })?;

        let totals = totals_of(&computed);

        let q = filter
            .q
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);
        let state_filter = filter.state.as_deref().unwrap_or("dues");
        let mut list: Vec<FeeRegisterRowDto> = computed
            .into_iter()
            .filter(|(_, _, _, _, st)| match state_filter {
                "paid" => *st == FeeState::Paid,
                "rte" => *st == FeeState::Rte,
                "all" => true,
                _ => matches!(st, FeeState::Due | FeeState::Part),
            })
            .filter(|(e, _, _, _, _)| filter.section_id.as_ref().is_none_or(|s| &e.section_id == s))
            .filter(|(e, _, _, _, _)| match &q {
                None => true,
                Some(q) => {
                    e.student.name.to_lowercase().contains(q)
                        || e.student.adm_no.to_lowercase().contains(q)
                        || e.student.father.to_lowercase().contains(q)
                        || e.student.mobile.contains(q)
                }
            })
            .map(|(e, due, paid, balance, state)| FeeRegisterRowDto {
                id: e.student.id.clone(),
                adm: e.student.adm_no.clone(),
                name: e.student.name.clone(),
                ck: ck(&e),
                father: e.student.father.clone(),
                mobile: e.student.mobile.clone(),
                rte: e.rte,
                due,
                paid,
                balance,
                fee_state: state,
            })
            .collect();
        list.sort_by(|a, b| a.ck.cmp(&b.ck).then(a.name.cmp(&b.name)));

        Ok(FeeRegisterDto {
            totals,
            collected_today: collected.0,
            receipts_today: collected.1,
            session: session.name.clone(),
            terms: session.terms,
            device_prefix: self.device_code()?,
            list,
        })
    }

    /// Due/paid/balance/state for each enrolled row, keyed by enrollment id.
    fn plan_inputs(
        conn: &Connection,
        session: &SessionRow,
        rows: &[EnrolledStudent],
    ) -> Result<std::collections::HashMap<String, FeeCalc>, DbError> {
        let paid_map = receipts::paid_by_student(conn, &session.id)?;
        let mut out = std::collections::HashMap::with_capacity(rows.len());
        for e in rows {
            let plan = account::plan_for(conn, &session.id, &e.class_id)?;
            let inputs = fees::FeeInputs {
                plan,
                terms: session.terms as u32,
                transport_fee_per_term: session.transport_fee_per_term,
                transport: e.transport,
                rte: e.rte,
                concession: e.concession,
            };
            let due = fees::session_due(&inputs);
            let paid = *paid_map.get(&e.student.id).unwrap_or(&0);
            let balance = fee_balance(due, paid);
            let state = fees::fee_state(due, paid, e.rte);
            out.insert(e.enrollment_id.clone(), (due, paid, balance, state));
        }
        Ok(out)
    }

    pub fn account(&self, actor: &Actor, student_id: &str) -> Result<FeeAccountDto, ServiceError> {
        permissions::authorize(actor, Action::FeesView)?;
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        let (acct, receipt_rows) = self.services.db.read(|c| {
            let acct = fee_account(c, &session, &enrolled)?;
            let receipt_rows = receipts::list_for_student(c, &session_id, student_id)?;
            Ok((acct, receipt_rows))
        })?;
        Ok(FeeAccountDto {
            student_id: enrolled.student.id.clone(),
            name: enrolled.student.name.clone(),
            ck: ck(&enrolled),
            roll: enrolled.roll,
            rte: enrolled.rte,
            status: enrolled.status.clone(),
            due: acct.due,
            paid: acct.paid,
            balance: acct.balance,
            term_fee: acct.term_fee,
            terms: session.terms,
            transport: enrolled.transport,
            transport_fee: session.transport_fee_per_term,
            concession: enrolled.concession,
            one_term: acct.one_term,
            one_term_payable: acct.one_term_payable,
            previous_session_dues: acct.previous_session_dues,
            receipts: receipt_rows
                .into_iter()
                .map(|r| ReceiptSummaryDto {
                    id: r.id,
                    no: r.receipt_no,
                    date: r.paid_on,
                    amount: r.amount,
                    mode: r.mode,
                    cancelled: r.cancelled,
                })
                .collect(),
        })
    }

    pub fn collect(&self, actor: &Actor, input: CollectInput) -> Result<ReceiptDto, ServiceError> {
        permissions::authorize(actor, Action::FeesCollect)?;
        let session = self.session()?;
        let session_id = session.id.clone();
        let enrolled = self
            .services
            .db
            .read(|c| students::enrolled_by_id(c, &session_id, &input.student_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "students.error.not_found"))?;
        let acct = self.services.db.read(|c| fee_account(c, &session, &enrolled))?;

        let mode = parse_mode(&input.mode)?;
        let reference = input.reference.trim().to_owned();
        let upi_used = if mode == PayMode::Upi {
            self.services
                .db
                .read(|c| receipts::upi_reference_active(c, &reference))?
        } else {
            false
        };
        validate_payment(&PaymentCheck {
            amount: input.amount,
            balance: acct.balance,
            mode,
            reference: &reference,
            upi_reference_already_used: upi_used,
            rte: enrolled.rte,
            student_active: enrolled.status == "active",
        })?;

        let device_code = self.device_code()?;
        let stored_ref = if mode == PayMode::Cash {
            String::new()
        } else {
            reference
        };
        let note = input.note.trim().to_owned();
        let now = self.services.clock.now_utc().to_rfc3339();
        let today = self.services.clock.today_local().to_string();
        let hlc = self.services.next_hlc().to_text();
        let id = self.services.ids.new_id();
        let balance_after = acct.balance - input.amount;
        let paid_after = acct.paid + input.amount;
        let student_name = enrolled.student.name.clone();
        let student_id = input.student_id.clone();
        let is_server = self.services.mode == Mode::Server;

        self.services.db.write(|tx| {
            let counter = meta::next_counter(tx, &format!("receipt:{device_code}"))?;
            let receipt_no = receipt_number(&device_code, counter);
            receipts::insert(
                tx,
                &ReceiptRow {
                    id: id.clone(),
                    receipt_no: receipt_no.clone(),
                    session_id: session_id.clone(),
                    student_id: student_id.clone(),
                    amount: input.amount,
                    mode: mode_str(mode).to_owned(),
                    reference: stored_ref.clone(),
                    note: note.clone(),
                    paid_on: today.clone(),
                    created_by: actor.user_id.clone(),
                    device_code: device_code.clone(),
                    balance_after,
                    hlc: hlc.clone(),
                },
                &now,
            )?;
            // Overpayment can arise after a concession change or a sync merge.
            if is_server
                && paid_after > acct.due
                && !alerts::unresolved_exists(tx, "overpayment", "student", &student_id)?
            {
                alerts::insert(
                    tx,
                    &AlertRow {
                        id: self.services.ids.new_id(),
                        kind: "overpayment".to_owned(),
                        entity: "student".to_owned(),
                        entity_id: student_id.clone(),
                        message_key: "alerts.overpayment".to_owned(),
                        params_json: json!({ "name": student_name, "amount": paid_after - acct.due })
                            .to_string(),
                        created_at: now.clone(),
                        resolved_at: None,
                    },
                )?;
            }
            write_entry(
                self.services,
                tx,
                Some(actor),
                ChangeRecord {
                    kind: "fee",
                    entity: "receipt",
                    entity_id: &id,
                    op: Op::Append,
                    summary_key: "fees.log.collected",
                    params: json!({
                        "receiptNo": receipt_no,
                        "amount": input.amount,
                        "mode": mode_str(mode),
                        "name": student_name,
                    }),
                    payload: json!({}),
                },
            )?;
            Ok(())
        })?;
        self.get_receipt(actor, &id)
    }

    pub fn get_receipt(&self, actor: &Actor, receipt_id: &str) -> Result<ReceiptDto, ServiceError> {
        permissions::authorize(actor, Action::FeesView)?;
        let detail = self
            .services
            .db
            .read(|c| receipts::get_detail(c, receipt_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "fees.error.receipt_not_found"))?;
        Ok(receipt_dto(detail, self.school_name()?))
    }

    pub fn cancel(&self, actor: &Actor, receipt_id: &str, reason: &str) -> Result<ReceiptDto, ServiceError> {
        permissions::authorize(actor, Action::FeesCancelReceipt)?;
        let reason = validate_reason(reason, 4)
            .map_err(|_| ServiceError::validation("fees.error.cancel_reason").field("reason"))?;
        let detail = self
            .services
            .db
            .read(|c| receipts::get_detail(c, receipt_id))?
            .ok_or_else(|| ServiceError::new(ErrorKind::NotFound, "fees.error.receipt_not_found"))?;
        if detail.cancelled.is_some() {
            return Err(ServiceError::new(
                ErrorKind::Conflict,
                "fees.error.already_cancelled",
            ));
        }
        let now = self.services.clock.now_utc().to_rfc3339();
        let hlc = self.services.next_hlc().to_text();
        let rid = receipt_id.to_owned();
        let name = detail.student_name.clone();
        let receipt_no = detail.receipt_no.clone();
        let amount = detail.amount;
        let reason_owned = reason.clone();
        self.services.db.write(|tx| {
            cancellations::insert(tx, &rid, &actor.user_id, &now, &reason_owned, &hlc)?;
            write_entry(
                self.services,
                tx,
                Some(actor),
                ChangeRecord {
                    kind: "fee",
                    entity: "receipt",
                    entity_id: &rid,
                    op: Op::Append,
                    summary_key: "fees.log.cancelled",
                    params: json!({
                        "receiptNo": receipt_no,
                        "amount": amount,
                        "name": name,
                        "reason": reason_owned,
                    }),
                    payload: json!({}),
                },
            )?;
            Ok(())
        })?;
        self.get_receipt(actor, receipt_id)
    }

    pub fn day_book(&self, actor: &Actor, date: &str) -> Result<DayBookDto, ServiceError> {
        permissions::authorize(actor, Action::FeesDaybook)?;
        let day = parse_date(date)?;
        if day > self.services.clock.today_local() {
            return Err(ServiceError::validation("fees.error.future_date").field("date"));
        }
        let details = self
            .services
            .db
            .read(|c| receipts::list_details_for_date(c, date))?;
        let school_name = self.school_name()?;

        let mut modes = Vec::new();
        for m in ["Cash", "UPI", "Cheque"] {
            let live: Vec<&ReceiptDetail> = details
                .iter()
                .filter(|d| d.cancelled.is_none() && d.mode == m)
                .collect();
            modes.push(DayModeDto {
                mode: m.to_owned(),
                total: live.iter().map(|d| d.amount).sum(),
                count: live.len() as i64,
            });
        }
        let live: Vec<&ReceiptDetail> = details.iter().filter(|d| d.cancelled.is_none()).collect();
        let total = live.iter().map(|d| d.amount).sum();
        let count = live.len() as i64;
        let cancelled_count = details.iter().filter(|d| d.cancelled.is_some()).count() as i64;
        let receipts = details
            .into_iter()
            .map(|d| receipt_dto(d, school_name.clone()))
            .collect();
        Ok(DayBookDto {
            date: date.to_owned(),
            modes,
            total,
            count,
            cancelled_count,
            receipts,
            school: SchoolHeaderLite { name: school_name },
        })
    }

    pub fn list_alerts(&self, actor: &Actor) -> Result<Vec<AlertDto>, ServiceError> {
        permissions::authorize(actor, Action::AlertsView)?;
        let kinds: &[&str] = if actor.role == Role::Accountant {
            &["overpayment"]
        } else {
            &[]
        };
        let rows = self.services.db.read(|c| alerts::list_unresolved(c, kinds))?;
        Ok(rows
            .into_iter()
            .map(|a| AlertDto {
                params: serde_json::from_str(&a.params_json).unwrap_or(serde_json::Value::Null),
                id: a.id,
                kind: a.kind,
                message_key: a.message_key,
                entity: a.entity,
                entity_id: a.entity_id,
                created_at: a.created_at,
            })
            .collect())
    }

    pub fn resolve_alert(&self, actor: &Actor, alert_id: &str) -> Result<(), ServiceError> {
        permissions::authorize(actor, Action::AlertsView)?;
        let now = self.services.clock.now_utc().to_rfc3339();
        let aid = alert_id.to_owned();
        let user_id = actor.user_id.clone();
        let changed = self
            .services
            .db
            .write(|tx| alerts::resolve(tx, &aid, &user_id, &now))?;
        if changed == 0 {
            return Err(ServiceError::new(ErrorKind::NotFound, "alerts.error.not_found"));
        }
        Ok(())
    }
}

fn totals_of(computed: &[(EnrolledStudent, i64, i64, i64, FeeState)]) -> FeeTotalsDto {
    let mut due = 0;
    let mut paid = 0;
    let mut unpaid = 0;
    let mut part = 0;
    let mut paying = 0;
    for (_, d, p, _, state) in computed {
        if *state == FeeState::Rte {
            continue;
        }
        paying += 1;
        due += *d;
        paid += (*p).min(*d);
        match state {
            FeeState::Due => unpaid += 1,
            FeeState::Part => part += 1,
            _ => {}
        }
    }
    FeeTotalsDto {
        pending: (due - paid).max(0),
        pct_collected: if due > 0 { (paid * 100) / due } else { 0 },
        due,
        paid,
        unpaid,
        part,
        paying_count: paying,
    }
}
