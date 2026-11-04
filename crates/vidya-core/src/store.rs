//! store — school-store rules (P15 Step 6, §10.3, optional module `store`). Pure.
//!
//! Stock can never go negative (a sale beyond stock is blocked with a message);
//! the price is snapshotted onto the sale line (later price changes don't rewrite
//! past sales). Money is integer **paise** (`i64`).

use crate::errors::{CoreError, CoreResult};

/// One line of a store sale, with the item's current stock for the block check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleLine {
    pub item_id: String,
    pub qty: i64,
    /// Price snapshot at sale time (paise).
    pub price_paise: i64,
    /// The item's current stock (for the can't-go-negative check).
    pub stock: i64,
}

/// Validate a store sale and return the total (paise). §10.3:
/// * non-empty;
/// * every `qty > 0`;
/// * `qty <= stock` for each line, else `Validation{stock, insufficient}` (the
///   sale is blocked — stock never goes negative);
/// * total = Σ price × qty must be > 0.
pub fn validate_sale(lines: &[SaleLine]) -> CoreResult<i64> {
    if lines.is_empty() {
        return Err(CoreError::validation("items", "empty"));
    }
    let mut total: i64 = 0;
    for l in lines {
        if l.qty <= 0 {
            return Err(CoreError::validation("qty", "positive"));
        }
        if l.price_paise < 0 {
            return Err(CoreError::validation("price", "negative"));
        }
        if l.qty > l.stock {
            return Err(CoreError::validation("stock", "insufficient"));
        }
        total += l.price_paise * l.qty;
    }
    if total <= 0 {
        return Err(CoreError::validation("total", "positive"));
    }
    Ok(total)
}

/// The school PC's view of a store sale replayed from another device (W1, §8.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayedSale {
    /// Total of the money actually taken (Σ device price × qty), paise.
    pub total_paise: i64,
    /// `(item_id, stock after the sale)` for every line that took the item's stock
    /// below zero (an oversell the Principal must review — never a refusal).
    pub oversold: Vec<(String, i64)>,
}

/// Validate a sale that ANOTHER device already made (money already taken, §8.6):
/// the shape must be a real sale (non-empty, `qty > 0`, `price ≥ 0`, total > 0) —
/// but stock is **not** a reason to refuse. Lines whose quantity exceeds the school
/// PC's current stock are reported in `oversold` (stock may go negative) so the
/// caller raises a review flag instead.
pub fn replay_sale(lines: &[SaleLine]) -> CoreResult<ReplayedSale> {
    if lines.is_empty() {
        return Err(CoreError::validation("items", "empty"));
    }
    let mut total: i64 = 0;
    let mut oversold = Vec::new();
    // The same item may appear on several lines: track its running stock.
    let mut running: Vec<(String, i64)> = Vec::new();
    for l in lines {
        if l.qty <= 0 {
            return Err(CoreError::validation("qty", "positive"));
        }
        if l.price_paise < 0 {
            return Err(CoreError::validation("price", "negative"));
        }
        let after = match running.iter_mut().find(|(id, _)| *id == l.item_id) {
            Some((_, s)) => {
                *s -= l.qty;
                *s
            }
            None => {
                running.push((l.item_id.clone(), l.stock - l.qty));
                l.stock - l.qty
            }
        };
        if after < 0 {
            oversold.retain(|(id, _)| id != &l.item_id);
            oversold.push((l.item_id.clone(), after));
        }
        total = total
            .checked_add(l.price_paise.checked_mul(l.qty).ok_or_else(|| CoreError::validation("total", "overflow"))?)
            .ok_or_else(|| CoreError::validation("total", "overflow"))?;
    }
    if total <= 0 {
        return Err(CoreError::validation("total", "positive"));
    }
    Ok(ReplayedSale { total_paise: total, oversold })
}

/// Validate a new stock level after a purchase / adjustment: it must be ≥ 0.
pub fn validate_new_stock(new_stock: i64) -> CoreResult<()> {
    if new_stock < 0 {
        return Err(CoreError::validation("stock", "negative"));
    }
    Ok(())
}

/// Validate a purchase / adjustment of `delta` on `current` stock. The result must
/// be ≥ 0 — except that an INCREASE is always allowed: after a replayed oversell the
/// stock can be negative, and a purchase that only partly covers it must still be
/// recordable.
pub fn validate_stock_change(current: i64, delta: i64) -> CoreResult<i64> {
    let new_stock = current.checked_add(delta).ok_or_else(|| CoreError::validation("stock", "overflow"))?;
    if delta > 0 {
        return Ok(new_stock);
    }
    validate_new_stock(new_stock)?;
    Ok(new_stock)
}

/// Validate a store item's price (≥ 0) and low-stock threshold (≥ 0).
pub fn validate_item(price_paise: i64, low_stock_at: i64) -> CoreResult<()> {
    if price_paise < 0 {
        return Err(CoreError::validation("price", "negative"));
    }
    if low_stock_at < 0 {
        return Err(CoreError::validation("low_stock_at", "negative"));
    }
    Ok(())
}

/// Whether an item is at or below its low-stock threshold.
pub fn is_low_stock(stock: i64, low_stock_at: i64) -> bool {
    stock <= low_stock_at
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(qty: i64, price: i64, stock: i64) -> SaleLine {
        SaleLine { item_id: "i1".into(), qty, price_paise: price, stock }
    }

    #[test]
    fn sale_total_and_stock_block() {
        // 1 × ₹2,450 + 2 × ₹350 = ₹3,150.
        let lines = [
            SaleLine { item_id: "book".into(), qty: 1, price_paise: 245_000, stock: 18 },
            SaleLine { item_id: "shirt".into(), qty: 2, price_paise: 35_000, stock: 42 },
        ];
        assert_eq!(validate_sale(&lines).unwrap(), 245_000 + 70_000);

        // qty beyond stock is blocked (stock never goes negative).
        assert_eq!(validate_sale(&[line(5, 10_000, 4)]), Err(CoreError::validation("stock", "insufficient")));
        // qty must be positive.
        assert_eq!(validate_sale(&[line(0, 10_000, 4)]), Err(CoreError::validation("qty", "positive")));
        // empty cart rejected.
        assert_eq!(validate_sale(&[]), Err(CoreError::validation("items", "empty")));
    }

    #[test]
    fn selling_all_stock_is_allowed() {
        assert!(validate_sale(&[line(4, 10_000, 4)]).is_ok());
    }

    #[test]
    fn a_replayed_sale_is_never_refused_for_stock() {
        // 5 sold against 4 in stock: recorded, oversold reported (stock → -1).
        let r = replay_sale(&[line(5, 10_000, 4)]).unwrap();
        assert_eq!(r.total_paise, 50_000);
        assert_eq!(r.oversold, vec![("i1".to_string(), -1)]);
        // Two lines of the same item share the running stock.
        let r = replay_sale(&[line(3, 100, 4), line(3, 100, 4)]).unwrap();
        assert_eq!(r.oversold, vec![("i1".to_string(), -2)]);
        // Within stock → nothing flagged.
        assert!(replay_sale(&[line(4, 100, 4)]).unwrap().oversold.is_empty());
        // Shape rules still apply.
        assert!(replay_sale(&[]).is_err());
        assert!(replay_sale(&[line(0, 100, 4)]).is_err());
        assert!(replay_sale(&[line(1, -1, 4)]).is_err());
        assert!(replay_sale(&[line(1, 0, 4)]).is_err());
    }

    #[test]
    fn a_purchase_may_leave_negative_stock_less_negative() {
        assert_eq!(validate_stock_change(-3, 2).unwrap(), -1);
        assert_eq!(validate_stock_change(5, -5).unwrap(), 0);
        assert!(validate_stock_change(5, -6).is_err());
        assert!(validate_stock_change(-3, -1).is_err());
    }

    #[test]
    fn stock_and_item_validation() {
        assert!(validate_new_stock(0).is_ok());
        assert_eq!(validate_new_stock(-1), Err(CoreError::validation("stock", "negative")));
        assert!(validate_item(48_000, 5).is_ok());
        assert_eq!(validate_item(-1, 5), Err(CoreError::validation("price", "negative")));
        assert!(is_low_stock(4, 5));
        assert!(is_low_stock(5, 5));
        assert!(!is_low_stock(6, 5));
    }
}
