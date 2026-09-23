//! Money is integer **paise** (i64) everywhere in vidya-core. No floats. Rupee
//! formatting is the UI's job (prompts/P02 Step 2 money.rs).

use crate::errors::CoreError;
use serde::{Deserialize, Serialize};

/// A signed amount in paise (100 paise = ₹1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Paise(pub i64);

impl Paise {
    pub const ZERO: Paise = Paise(0);

    #[inline]
    pub fn get(self) -> i64 {
        self.0
    }

    #[inline]
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Checked add; overflow → `INTERNAL{money.overflow}`.
    pub fn checked_add(self, other: Paise) -> Result<Paise, CoreError> {
        self.0
            .checked_add(other.0)
            .map(Paise)
            .ok_or_else(|| CoreError::Internal { id: "money.overflow".into() })
    }

    /// Checked sub; overflow → `INTERNAL{money.overflow}`.
    pub fn checked_sub(self, other: Paise) -> Result<Paise, CoreError> {
        self.0
            .checked_sub(other.0)
            .map(Paise)
            .ok_or_else(|| CoreError::Internal { id: "money.overflow".into() })
    }

    /// Saturating-at-zero subtraction (for "balance after" style displays).
    pub fn sub_floor_zero(self, other: Paise) -> Paise {
        Paise((self.0 - other.0).max(0))
    }
}

impl std::iter::Sum for Paise {
    fn sum<I: Iterator<Item = Paise>>(iter: I) -> Paise {
        Paise(iter.map(|p| p.0).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_sub() {
        assert_eq!(Paise(600000).checked_add(Paise(50000)).unwrap(), Paise(650000));
        assert_eq!(Paise(310000).checked_sub(Paise(100000)).unwrap(), Paise(210000));
    }

    #[test]
    fn overflow_errors() {
        assert!(Paise(i64::MAX).checked_add(Paise(1)).is_err());
        assert!(Paise(i64::MIN).checked_sub(Paise(1)).is_err());
    }

    #[test]
    fn sub_floor_zero_never_negative() {
        assert_eq!(Paise(1000).sub_floor_zero(Paise(3100)), Paise::ZERO);
        assert_eq!(Paise(3100).sub_floor_zero(Paise(1000)), Paise(2100));
    }

    #[test]
    fn sum_of_paise() {
        let total: Paise = vec![Paise(600000), Paise(50000), Paise(20000)].into_iter().sum();
        assert_eq!(total, Paise(670000));
    }
}
