use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// A whole-rupee amount. Money is never represented with floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rupees(pub i64);

const MAX_RUPEES: i64 = 1_000_000_000;

/// Formats whole rupees with Indian digit grouping.
pub fn format_inr(amount: Rupees) -> String {
    let negative = amount.0 < 0;
    let digits = amount.0.unsigned_abs().to_string();
    let grouped = if digits.len() <= 3 {
        digits
    } else {
        let split = digits.len() - 3;
        let (head, tail) = digits.split_at(split);
        let mut first_group = head.len() % 2;
        if first_group == 0 {
            first_group = 2;
        }
        let mut groups = vec![head[..first_group].to_owned()];
        let mut index = first_group;
        while index < head.len() {
            groups.push(head[index..index + 2].to_owned());
            index += 2;
        }
        groups.push(tail.to_owned());
        groups.join(",")
    };
    if negative {
        format!("−₹{grouped}")
    } else {
        format!("₹{grouped}")
    }
}

/// Parses a non-negative whole-rupee amount up to ten crore.
pub fn parse_rupees(input: &str) -> Result<Rupees, DomainError> {
    let trimmed = input.trim();
    let without_symbol = trimmed.strip_prefix('₹').unwrap_or(trimmed).trim();
    let digits = without_symbol.replace(',', "");
    let valid = !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
    let amount = valid.then(|| digits.parse::<i64>().ok()).flatten();
    match amount {
        Some(value) if value <= MAX_RUPEES => Ok(Rupees(value)),
        _ => Err(DomainError::validation("money.error.invalid")),
    }
}
