//! requests — the correction-request lifecycle (§5, §7, §8.5).
//!
//! Locked data (submitted attendance/marks, student details, payment reversals,
//! access/device changes) is corrected only through a request the Principal
//! approves, rejects or returns (§3 rule 6, §5). This module holds the pure
//! rules; all times and ids are passed in (no clock, no randomness), and there
//! are no floats.
//!
//! Lifecycle:
//! * [`create`] — validate the reason and enforce one pending request per
//!   `(target, type)` (`REQUEST_ALREADY_PENDING`).
//! * [`approve`] — the approver must currently be allowed and the target's
//!   `version` must still equal `base_version` (`REQUEST_STALE`); only a
//!   `Pending` request approves, and only once. Returns the exact [`ChangeSet`].
//! * [`reject`] / [`return_for_edits`] — from `Pending`, with a note ≥ 5 chars.
//! * [`cancel`] — the requester only, while `Pending`.
//! * [`resubmit_returned`] — a `Returned` request spawns a NEW revision row
//!   (`revision + 1`, `parent_request_id = Some(old.id)`); the old row is
//!   immutable.
//! * [`mark_applied`] / [`mark_failed`] — move `apply_state` after apply.

use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};
use crate::types::RequestType;

/// Minimum characters for a reject/return note.
const NOTE_MIN: usize = 5;
/// Reason length bounds (inclusive), counted in Unicode scalar values.
const REASON_MIN: usize = 1;
const REASON_MAX: usize = 300;

/// Decision status of a request row (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Returned,
    Cancelled,
}

/// Whether an approved request's change has been written to the target (§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyState {
    NotApplied,
    Applied,
    Failed,
}

/// Inputs to open a new request (§7 `request` columns). `before_json`/
/// `after_json` are opaque snapshots the caller has already serialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRequest {
    pub request_type: RequestType,
    pub target_table: String,
    pub target_id: String,
    pub base_version: i64,
    pub before_json: String,
    pub after_json: String,
    pub reason: String,
    pub requested_by: String,
}

/// The requester's edits when resubmitting a returned request. Only the payload,
/// reason and base_version may change — the target is fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRequestEdit {
    /// The new id for the resubmitted revision row (id passed in, never generated).
    pub id: String,
    pub base_version: i64,
    pub before_json: String,
    pub after_json: String,
    pub reason: String,
}

/// A request row (§7). One `(target, type)` may have at most one `Pending` row;
/// older revisions are kept immutable and linked by `parent_request_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub request_type: RequestType,
    pub target_table: String,
    pub target_id: String,
    pub base_version: i64,
    pub before_json: String,
    pub after_json: String,
    pub reason: String,
    pub requested_by: String,
    pub revision: i64,
    pub parent_request_id: Option<String>,
    pub status: RequestStatus,
    /// Who decided (approver/rejecter/returner/canceller); `None` while pending.
    pub decided_by: Option<String>,
    /// The note left on reject/return; `None` otherwise.
    pub note: Option<String>,
    pub apply_state: ApplyState,
}

/// The exact change an approved request applies to its target (§8.5). The caller
/// writes `after_json` onto `(target_table, target_id)` in one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub target_table: String,
    pub target_id: String,
    pub after_json: String,
}

/// Validate a reason: 1–300 Unicode scalar values, after trimming surrounding
/// whitespace (an all-whitespace reason is empty).
fn validate_reason(reason: &str) -> CoreResult<()> {
    let len = reason.trim().chars().count();
    if len < REASON_MIN {
        return Err(CoreError::validation("reason", "min_length"));
    }
    if len > REASON_MAX {
        return Err(CoreError::validation("reason", "max_length"));
    }
    Ok(())
}

/// Open a new request (§7).
///
/// * The reason must be 1–300 characters ([`validate_reason`]).
/// * At most one `Pending` request per `(target, type)`: if `input.request_type`
///   is already pending for this target (present in `pending_for_target`) →
///   [`CoreError::RequestAlreadyPending`].
///
/// `pending_for_target` is the set of request types currently `Pending` for
/// **this** `(target_table, target_id)`; the caller queries it. The `id` is
/// supplied by the caller (UUIDv7), never generated here.
///
/// Returns a fresh [`Request`]: `status = Pending`, `revision = 1`,
/// `parent_request_id = None`, `apply_state = NotApplied`.
pub fn create(
    id: impl Into<String>,
    input: NewRequest,
    pending_for_target: &[RequestType],
) -> CoreResult<Request> {
    validate_reason(&input.reason)?;

    if pending_for_target.contains(&input.request_type) {
        return Err(CoreError::RequestAlreadyPending);
    }

    Ok(Request {
        id: id.into(),
        request_type: input.request_type,
        target_table: input.target_table,
        target_id: input.target_id,
        base_version: input.base_version,
        before_json: input.before_json,
        after_json: input.after_json,
        reason: input.reason,
        requested_by: input.requested_by,
        revision: 1,
        parent_request_id: None,
        status: RequestStatus::Pending,
        decided_by: None,
        note: None,
        apply_state: ApplyState::NotApplied,
    })
}

/// Decide whether `req` may be approved and return the [`ChangeSet`] to apply
/// (§5, §8.5). Does NOT mutate `req`; the caller records the approval and writes
/// the change in one transaction, then calls [`mark_applied`]/[`mark_failed`].
///
/// * `approver_allowed` must be `true` (the approver's CURRENT permission,
///   re-checked at decide time — §5, §8.4). Otherwise [`CoreError::Forbidden`].
/// * `req.status` must be `Pending`; a second approval (status no longer
///   `Pending`) → [`CoreError::Forbidden`], so the change applies exactly once.
/// * `current_target_version` must equal `req.base_version`; if the target has
///   changed → [`CoreError::RequestStale`] (the Principal must reopen and review
///   current values before re-deciding).
pub fn approve(
    req: &Request,
    approver_allowed: bool,
    current_target_version: i64,
) -> CoreResult<ChangeSet> {
    if !approver_allowed {
        return Err(CoreError::Forbidden { reason: "not allowed to approve".into() });
    }
    if req.status != RequestStatus::Pending {
        // Already decided (approved/rejected/returned/cancelled): refuse, so an
        // approved change is never applied twice.
        return Err(CoreError::Forbidden { reason: "request is not pending".into() });
    }
    if current_target_version != req.base_version {
        // Target moved under the request — the review is stale (§8.5).
        return Err(CoreError::RequestStale);
    }

    Ok(ChangeSet {
        target_table: req.target_table.clone(),
        target_id: req.target_id.clone(),
        after_json: req.after_json.clone(),
    })
}

/// Reject `req` with a note (§5). Returns the rejected request; the old fields
/// are preserved and `status`/`decided_by`/`note` are set.
///
/// * `req.status` must be `Pending` → else [`CoreError::Forbidden`].
/// * `note` must be at least 5 characters →
///   `CoreError::validation("note", "min_length")`.
pub fn reject(req: &Request, decided_by: &str, note: &str) -> CoreResult<Request> {
    decide_with_note(req, RequestStatus::Rejected, decided_by, note)
}

/// Return `req` to the requester for edits, with a note (§5). Same rules as
/// [`reject`]; the requester may later [`resubmit_returned`].
pub fn return_for_edits(req: &Request, decided_by: &str, note: &str) -> CoreResult<Request> {
    decide_with_note(req, RequestStatus::Returned, decided_by, note)
}

/// Shared body for reject/return: only from `Pending`, note ≥ 5 chars.
fn decide_with_note(
    req: &Request,
    new_status: RequestStatus,
    decided_by: &str,
    note: &str,
) -> CoreResult<Request> {
    if req.status != RequestStatus::Pending {
        return Err(CoreError::Forbidden { reason: "request is not pending".into() });
    }
    if note.trim().chars().count() < NOTE_MIN {
        return Err(CoreError::validation("note", "min_length"));
    }

    let mut out = req.clone();
    out.status = new_status;
    out.decided_by = Some(decided_by.to_string());
    out.note = Some(note.to_string());
    Ok(out)
}

/// Cancel `req` (§5). Only the requester, and only while `Pending`.
///
/// * `by_staff_id` must equal `req.requested_by` → else [`CoreError::Forbidden`].
/// * `req.status` must be `Pending` → else [`CoreError::Forbidden`].
pub fn cancel(req: &Request, by_staff_id: &str) -> CoreResult<Request> {
    if by_staff_id != req.requested_by {
        return Err(CoreError::Forbidden { reason: "only the requester can cancel".into() });
    }
    if req.status != RequestStatus::Pending {
        return Err(CoreError::Forbidden { reason: "request is not pending".into() });
    }

    let mut out = req.clone();
    out.status = RequestStatus::Cancelled;
    out.decided_by = Some(by_staff_id.to_string());
    Ok(out)
}

/// Resubmit a `Returned` request as a NEW revision row (§7). The OLD row is
/// immutable — this returns a fresh [`Request`]; the caller does not mutate the
/// returned one.
///
/// The new row: `revision = returned.revision + 1`,
/// `parent_request_id = Some(returned.id)`, `status = Pending`,
/// `apply_state = NotApplied`, carrying the edited payload/reason/base_version.
/// The type, target and original requester are inherited unchanged.
///
/// * `returned.status` must be `Returned` → else [`CoreError::Forbidden`].
/// * the edited `reason` must pass [`validate_reason`].
pub fn resubmit_returned(returned: &Request, edited: NewRequestEdit) -> CoreResult<Request> {
    if returned.status != RequestStatus::Returned {
        return Err(CoreError::Forbidden {
            reason: "only a returned request can be resubmitted".into(),
        });
    }
    validate_reason(&edited.reason)?;

    Ok(Request {
        id: edited.id,
        request_type: returned.request_type,
        target_table: returned.target_table.clone(),
        target_id: returned.target_id.clone(),
        base_version: edited.base_version,
        before_json: edited.before_json,
        after_json: edited.after_json,
        reason: edited.reason,
        requested_by: returned.requested_by.clone(),
        revision: returned.revision + 1,
        parent_request_id: Some(returned.id.clone()),
        status: RequestStatus::Pending,
        decided_by: None,
        note: None,
        apply_state: ApplyState::NotApplied,
    })
}

/// Move an approved request's `apply_state` to [`ApplyState::Applied`] after the
/// change has been written. Only from [`ApplyState::NotApplied`], and only on an
/// `Approved` request → else [`CoreError::Forbidden`].
pub fn mark_applied(req: &Request) -> CoreResult<Request> {
    move_apply_state(req, ApplyState::Applied)
}

/// Move an approved request's `apply_state` to [`ApplyState::Failed`] when the
/// change could not be written. Same preconditions as [`mark_applied`].
pub fn mark_failed(req: &Request) -> CoreResult<Request> {
    move_apply_state(req, ApplyState::Failed)
}

fn move_apply_state(req: &Request, to: ApplyState) -> CoreResult<Request> {
    if req.status != RequestStatus::Approved {
        return Err(CoreError::Forbidden { reason: "request is not approved".into() });
    }
    if req.apply_state != ApplyState::NotApplied {
        return Err(CoreError::Forbidden { reason: "apply state already set".into() });
    }

    let mut out = req.clone();
    out.apply_state = to;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_request() -> NewRequest {
        NewRequest {
            request_type: RequestType::StudentDetails,
            target_table: "student".into(),
            target_id: "stu-1".into(),
            base_version: 4,
            before_json: r#"{"address":"old"}"#.into(),
            after_json: r#"{"address":"new"}"#.into(),
            reason: "Guardian moved house; updating the address on record.".into(),
            requested_by: "acc-1".into(),
        }
    }

    /// A pending request as it would come back from `create`.
    fn pending() -> Request {
        create("req-1", new_request(), &[]).unwrap()
    }

    // ---- create ---------------------------------------------------------

    #[test]
    fn create_sets_initial_fields() {
        let r = pending();
        assert_eq!(r.status, RequestStatus::Pending);
        assert_eq!(r.revision, 1);
        assert_eq!(r.parent_request_id, None);
        assert_eq!(r.apply_state, ApplyState::NotApplied);
        assert_eq!(r.decided_by, None);
        assert_eq!(r.note, None);
        assert_eq!(r.base_version, 4);
    }

    #[test]
    fn create_rejects_empty_reason() {
        let mut input = new_request();
        input.reason = "   ".into(); // all whitespace → empty
        assert_eq!(
            create("req-1", input, &[]),
            Err(CoreError::validation("reason", "min_length"))
        );
    }

    #[test]
    fn create_rejects_too_long_reason() {
        let mut input = new_request();
        input.reason = "x".repeat(301);
        assert_eq!(
            create("req-1", input, &[]),
            Err(CoreError::validation("reason", "max_length"))
        );
    }

    #[test]
    fn create_accepts_boundary_reason_lengths() {
        let mut one = new_request();
        one.reason = "x".into();
        assert!(create("req-1", one, &[]).is_ok());

        let mut max = new_request();
        max.reason = "y".repeat(300);
        assert!(create("req-2", max, &[]).is_ok());
    }

    #[test]
    fn create_rejects_second_pending_same_target_and_type() {
        // A StudentDetails request is already pending for this target.
        let pending_types = [RequestType::StudentDetails];
        assert_eq!(
            create("req-2", new_request(), &pending_types),
            Err(CoreError::RequestAlreadyPending)
        );
    }

    #[test]
    fn create_allows_different_type_for_same_target() {
        // A different type pending for the target does not block this one.
        let pending_types = [RequestType::PaymentReversal];
        assert!(create("req-2", new_request(), &pending_types).is_ok());
    }

    // ---- approve --------------------------------------------------------

    #[test]
    fn approve_returns_change_set() {
        let r = pending();
        let cs = approve(&r, true, 4).unwrap();
        assert_eq!(
            cs,
            ChangeSet {
                target_table: "student".into(),
                target_id: "stu-1".into(),
                after_json: r#"{"address":"new"}"#.into(),
            }
        );
    }

    #[test]
    fn approve_requires_allowed_approver() {
        let r = pending();
        assert!(matches!(approve(&r, false, 4), Err(CoreError::Forbidden { .. })));
    }

    #[test]
    fn approve_stale_when_target_changed() {
        let r = pending(); // base_version = 4
        // Target moved to version 5 since the request was written.
        assert_eq!(approve(&r, true, 5), Err(CoreError::RequestStale));
    }

    #[test]
    fn second_approval_errors_and_applies_once() {
        let r = pending();
        // First approval succeeds and the caller records status = Approved.
        let cs = approve(&r, true, 4).unwrap();
        assert_eq!(cs.after_json, r#"{"address":"new"}"#);

        let mut approved = r.clone();
        approved.status = RequestStatus::Approved;
        // A second approve on the now-Approved row is refused.
        assert!(matches!(approve(&approved, true, 4), Err(CoreError::Forbidden { .. })));
    }

    #[test]
    fn approve_refused_after_reject() {
        let r = pending();
        let rejected = reject(&r, "prin-1", "Not needed after all.").unwrap();
        assert!(matches!(approve(&rejected, true, 4), Err(CoreError::Forbidden { .. })));
    }

    // ---- reject / return ------------------------------------------------

    #[test]
    fn reject_requires_five_char_note() {
        let r = pending();
        assert_eq!(
            reject(&r, "prin-1", "no"),
            Err(CoreError::validation("note", "min_length"))
        );
        let ok = reject(&r, "prin-1", "Wrong student selected.").unwrap();
        assert_eq!(ok.status, RequestStatus::Rejected);
        assert_eq!(ok.decided_by.as_deref(), Some("prin-1"));
        assert_eq!(ok.note.as_deref(), Some("Wrong student selected."));
    }

    #[test]
    fn return_requires_five_char_note() {
        let r = pending();
        assert_eq!(
            return_for_edits(&r, "prin-1", "bad"),
            Err(CoreError::validation("note", "min_length"))
        );
        let ok = return_for_edits(&r, "prin-1", "Please add a supporting document.").unwrap();
        assert_eq!(ok.status, RequestStatus::Returned);
    }

    #[test]
    fn reject_only_from_pending() {
        let r = pending();
        let rejected = reject(&r, "prin-1", "First rejection.").unwrap();
        assert!(matches!(
            reject(&rejected, "prin-1", "Second rejection."),
            Err(CoreError::Forbidden { .. })
        ));
    }

    // ---- cancel ---------------------------------------------------------

    #[test]
    fn cancel_only_by_requester() {
        let r = pending(); // requested_by = acc-1
        assert!(matches!(cancel(&r, "acc-2"), Err(CoreError::Forbidden { .. })));
        let cancelled = cancel(&r, "acc-1").unwrap();
        assert_eq!(cancelled.status, RequestStatus::Cancelled);
        assert_eq!(cancelled.decided_by.as_deref(), Some("acc-1"));
    }

    #[test]
    fn cancel_only_while_pending() {
        let r = pending();
        let returned = return_for_edits(&r, "prin-1", "Needs more detail.").unwrap();
        assert!(matches!(cancel(&returned, "acc-1"), Err(CoreError::Forbidden { .. })));
    }

    // ---- resubmit returned ---------------------------------------------

    #[test]
    fn returned_resubmit_creates_new_revision() {
        let r = pending();
        let returned = return_for_edits(&r, "prin-1", "Please correct the DOB too.").unwrap();

        let edit = NewRequestEdit {
            id: "req-2".into(),
            base_version: 5,
            before_json: r#"{"address":"old","dob":"2010-01-01"}"#.into(),
            after_json: r#"{"address":"new","dob":"2010-02-02"}"#.into(),
            reason: "Updated address and corrected date of birth.".into(),
        };
        let resub = resubmit_returned(&returned, edit).unwrap();

        assert_eq!(resub.id, "req-2");
        assert_eq!(resub.revision, 2);
        assert_eq!(resub.parent_request_id.as_deref(), Some("req-1"));
        assert_eq!(resub.status, RequestStatus::Pending);
        assert_eq!(resub.apply_state, ApplyState::NotApplied);
        assert_eq!(resub.decided_by, None);
        assert_eq!(resub.note, None);
        assert_eq!(resub.base_version, 5);
        assert_eq!(resub.requested_by, "acc-1"); // inherited
        assert_eq!(resub.request_type, RequestType::StudentDetails); // inherited

        // The old (returned) revision is unchanged/immutable.
        assert_eq!(returned.revision, 1);
        assert_eq!(returned.status, RequestStatus::Returned);
        assert_eq!(returned.parent_request_id, None);
    }

    #[test]
    fn resubmit_only_from_returned() {
        let r = pending(); // still Pending, not Returned
        let edit = NewRequestEdit {
            id: "req-2".into(),
            base_version: 4,
            before_json: "{}".into(),
            after_json: "{}".into(),
            reason: "Trying to resubmit a pending request.".into(),
        };
        assert!(matches!(
            resubmit_returned(&r, edit),
            Err(CoreError::Forbidden { .. })
        ));
    }

    #[test]
    fn resubmit_validates_reason() {
        let r = pending();
        let returned = return_for_edits(&r, "prin-1", "Reason is too vague.").unwrap();
        let edit = NewRequestEdit {
            id: "req-2".into(),
            base_version: 4,
            before_json: "{}".into(),
            after_json: "{}".into(),
            reason: "  ".into(), // empty
        };
        assert_eq!(
            resubmit_returned(&returned, edit),
            Err(CoreError::validation("reason", "min_length"))
        );
    }

    // ---- apply state ----------------------------------------------------

    #[test]
    fn mark_applied_from_approved() {
        let mut approved = pending();
        approved.status = RequestStatus::Approved;
        let applied = mark_applied(&approved).unwrap();
        assert_eq!(applied.apply_state, ApplyState::Applied);
    }

    #[test]
    fn mark_failed_from_approved() {
        let mut approved = pending();
        approved.status = RequestStatus::Approved;
        let failed = mark_failed(&approved).unwrap();
        assert_eq!(failed.apply_state, ApplyState::Failed);
    }

    #[test]
    fn mark_applied_requires_approved() {
        let r = pending(); // still Pending
        assert!(matches!(mark_applied(&r), Err(CoreError::Forbidden { .. })));
    }

    #[test]
    fn mark_applied_only_once() {
        let mut approved = pending();
        approved.status = RequestStatus::Approved;
        let applied = mark_applied(&approved).unwrap();
        // A second transition is refused.
        assert!(matches!(mark_failed(&applied), Err(CoreError::Forbidden { .. })));
    }
}
