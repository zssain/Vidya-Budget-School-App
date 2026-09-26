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

/// Validate a new stock level after a purchase / adjustment: it must be ≥ 0.
pub fn validate_new_stock(new_stock: i64) -> CoreResult<()> {
    if new_stock < 0 {
        return Err(CoreError::validation("stock", "negative"));
    }
    Ok(())
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
