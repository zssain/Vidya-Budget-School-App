//! conflicts — decide how an incoming op meets the current row (§8.5).
//!
//! An `update` whose `base_version` is older than the current version **and**
//! touches a field changed since that base by another device is a conflict:
//! keep the current value and flag it, never silently overwrite (§8.5, rule 8).
//! Different fields of the same record merge. Payments never produce conflict
//! rows — money received is never rejected by sync (§8.5, §8.6).
//!
//! Pure decision logic: no IO, no clock, no randomness. The caller supplies the
//! current version, the fields the op writes, and the fields other devices have
//! already changed since the op's base.

/// The table whose ops never conflict — payments are append-only and money is
/// never lost to sync (§8.6).
const PAYMENT_TABLE: &str = "payment";

/// Metadata an op carries for conflict detection (§8.1 op shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpMeta {
    /// The table the op writes to (e.g. `"student"`, `"payment"`).
    pub table: String,
    /// The row version the op was written against.
    pub base_version: i64,
}

/// The outcome of [`detect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    /// Apply the op as-is (no concurrent change, or a never-conflicting table).
    Apply,
    /// Merge: the op's fields are disjoint from what changed since its base, so
    /// they can be written without clobbering another device's change. Carries
    /// the fields the op writes.
    Merge(Vec<String>),
    /// Conflict: the op's fields overlap fields changed since its base. Carries
    /// the overlapping field names (order preserving `fields_in_op`).
    Conflict(Vec<String>),
}

/// Decide how `op` should meet the current row (§8.5).
///
/// * `op.table == "payment"` → [`Detection::Apply`] always (payments never
///   conflict, even when fields overlap — §8.6).
/// * `op.base_version == current_version` → [`Detection::Apply`] (no concurrent
///   change happened since the op was written).
/// * otherwise, if `fields_in_op` and `fields_changed_since_base` are disjoint
///   → [`Detection::Merge`] of `fields_in_op` (different fields merge).
/// * otherwise → [`Detection::Conflict`] of the overlapping fields.
pub fn detect(
    op: &OpMeta,
    current_version: i64,
    fields_in_op: &[String],
    fields_changed_since_base: &[String],
) -> Detection {
    // Payments never conflict (§8.6) — decided before anything else.
    if op.table == PAYMENT_TABLE {
        return Detection::Apply;
    }

    // No concurrent change since the op's base → apply as-is.
    if op.base_version == current_version {
        return Detection::Apply;
    }

    // Something changed since the base. Find the fields the op writes that were
    // also changed by another device (order-preserving over `fields_in_op`).
    let overlap: Vec<String> = fields_in_op
        .iter()
        .filter(|f| fields_changed_since_base.iter().any(|c| c == *f))
        .cloned()
        .collect();

    if overlap.is_empty() {
        // Disjoint fields → safe to merge (§8.5).
        Detection::Merge(fields_in_op.to_vec())
    } else {
        // Overlapping fields → conflict; keep current, flag for the Principal.
        Detection::Conflict(overlap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn op(table: &str, base_version: i64) -> OpMeta {
        OpMeta { table: table.to_string(), base_version }
    }

    #[test]
    fn same_base_version_applies() {
        // No concurrent change since base → Apply, regardless of fields.
        let got = detect(
            &op("student", 5),
            5,
            &fields(&["address"]),
            &fields(&["address", "guardian_mobile"]),
        );
        assert_eq!(got, Detection::Apply);
    }

    #[test]
    fn disjoint_fields_merge() {
        // Op edits `address`; another device changed `guardian_mobile`. Disjoint.
        let got = detect(
            &op("student", 3),
            4,
            &fields(&["address"]),
            &fields(&["guardian_mobile"]),
        );
        assert_eq!(got, Detection::Merge(fields(&["address"])));
    }

    #[test]
    fn overlapping_field_conflicts() {
        // Both devices changed `address` from the same base → conflict.
        let got = detect(
            &op("student", 3),
            4,
            &fields(&["address"]),
            &fields(&["address"]),
        );
        assert_eq!(got, Detection::Conflict(fields(&["address"])));
    }

    #[test]
    fn conflict_reports_only_overlapping_fields() {
        // Op writes address + guardian_name; only address overlaps.
        let got = detect(
            &op("student", 3),
            7,
            &fields(&["address", "guardian_name"]),
            &fields(&["address", "guardian_mobile"]),
        );
        assert_eq!(got, Detection::Conflict(fields(&["address"])));
    }

    #[test]
    fn conflict_preserves_op_field_order() {
        // Overlap keeps the order of fields_in_op, not fields_changed_since.
        let got = detect(
            &op("student", 3),
            7,
            &fields(&["name", "address"]),
            &fields(&["address", "name"]),
        );
        assert_eq!(got, Detection::Conflict(fields(&["name", "address"])));
    }

    #[test]
    fn payment_applies_even_with_overlap() {
        // Payment op with an older base AND overlapping fields → still Apply.
        let got = detect(
            &op("payment", 1),
            9,
            &fields(&["amount_paise"]),
            &fields(&["amount_paise"]),
        );
        assert_eq!(got, Detection::Apply);
    }

    #[test]
    fn payment_never_conflicts_even_when_stale() {
        let got = detect(
            &op("payment", 0),
            100,
            &fields(&["reference", "amount_paise"]),
            &fields(&["reference", "amount_paise", "mode"]),
        );
        assert_eq!(got, Detection::Apply);
    }

    #[test]
    fn stale_op_with_no_fields_changed_merges() {
        // Base is older but nothing was changed since → nothing to conflict with.
        let got = detect(&op("student", 2), 5, &fields(&["address"]), &fields(&[]));
        assert_eq!(got, Detection::Merge(fields(&["address"])));
    }
}
