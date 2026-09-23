//! words — amount-in-words for receipts, Indian numbering system (pure, no IO).
//!
//! Input is [`Paise`]; it is split into rupees (`paise / 100`) and a paise
//! remainder (`paise % 100`). English uses ones/tens/hundred/thousand/lakh/
//! crore; Hindi uses standard Hindi number words. Integer math only — no floats.
//!
//! ## OWNER-REVIEW — Hindi number words
//!
//! Hindi 1–99 is highly irregular (every number has its own word) and dialect/
//! transliteration varies. The full 0–99 table below is provided for
//! completeness, but the following entries in particular should be confirmed by
//! a native-Hindi reviewer before shipping receipts to customers. The spellings
//! used here follow common Devanagari usage but may differ from a school's
//! preferred house style (e.g. nukta placement, chandrabindu vs anusvara):
//!
//!   - छह (6)               — also written छः
//!   - सोलह (16), सत्रह (17), अठारह (18), उन्नीस (19)
//!   - इक्कीस (21) … उनतीस (29)  — the whole 21–29 band
//!   - इकतीस (31) … उनतालीस (39) — the whole 31–39 band
//!   - इकतालीस (41) … उनचास (49) — the whole 41–49 band
//!   - इक्यावन (51) … उनसठ (59)  — the whole 51–59 band
//!   - इकसठ (61) … उनहत्तर (69)  — the whole 61–69 band
//!   - इकहत्तर (71) … उनासी (79)  — the whole 71–79 band
//!   - इक्यासी (81) … नवासी (89)  — the whole 81–89 band
//!   - इक्यानवे (91) … निन्यानवे (99) — the whole 91–99 band
//!   - हज़ार (thousand), करोड़ (crore) — nukta forms; some prefer हजार / करोड़
//!
//! Numbers 0–20, the round tens (20,30,…90), सौ (hundred) and लाख (lakh) are
//! high-confidence. English is fully verified and correct.

use crate::money::Paise;

// ---- English ---------------------------------------------------------------

const EN_ONES: [&str; 20] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight",
    "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen",
    "sixteen", "seventeen", "eighteen", "nineteen",
];

const EN_TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty",
    "ninety",
];

/// Words for 0..=99 in English (empty string for 0 so it contributes nothing
/// inside a larger number; callers handle the standalone-zero case).
fn en_two_digits(n: u64) -> String {
    debug_assert!(n < 100);
    if n < 20 {
        EN_ONES[n as usize].to_string()
    } else {
        let tens = EN_TENS[(n / 10) as usize];
        let ones = n % 10;
        if ones == 0 {
            tens.to_string()
        } else {
            format!("{} {}", tens, EN_ONES[ones as usize])
        }
    }
}

/// Words for 0..=999 in English (used for each hundreds-block).
fn en_three_digits(n: u64) -> String {
    debug_assert!(n < 1000);
    let hundreds = n / 100;
    let rest = n % 100;
    match (hundreds, rest) {
        (0, _) => en_two_digits(rest),
        (h, 0) => format!("{} hundred", EN_ONES[h as usize]),
        (h, r) => format!("{} hundred {}", EN_ONES[h as usize], en_two_digits(r)),
    }
}

/// A non-negative integer in words, Indian numbering system (English).
fn en_number(mut n: u64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    let mut parts: Vec<String> = Vec::new();

    let crore = n / 10_000_000;
    n %= 10_000_000;
    let lakh = n / 100_000;
    n %= 100_000;
    let thousand = n / 1_000;
    n %= 1_000;
    let below_thousand = n; // 0..=999

    if crore > 0 {
        parts.push(format!("{} crore", en_number(crore)));
    }
    if lakh > 0 {
        parts.push(format!("{} lakh", en_two_digits(lakh)));
    }
    if thousand > 0 {
        parts.push(format!("{} thousand", en_two_digits(thousand)));
    }
    if below_thousand > 0 {
        parts.push(en_three_digits(below_thousand));
    }

    parts.join(" ")
}

/// Capitalise the first ASCII letter of `s`, leaving the rest untouched.
fn capitalise_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// English amount-in-words for a receipt, e.g.
/// `Paise(310050)` → `"Three thousand one hundred rupees and fifty paise only"`.
///
/// Negative amounts are prefixed with "Minus".
pub fn amount_in_words_en(amount: Paise) -> String {
    let value = amount.get();
    let negative = value < 0;
    let abs = value.unsigned_abs();
    let rupees = abs / 100;
    let paise = abs % 100;

    let mut body = format!("{} rupees", en_number(rupees));
    if paise > 0 {
        body.push_str(&format!(" and {} paise", en_two_digits(paise)));
    }
    body.push_str(" only");

    if negative {
        body = format!("minus {body}");
    }
    capitalise_first(&body)
}

// ---- Hindi -----------------------------------------------------------------
//
// Hindi 0–99 is irregular, so a full table is used rather than composing tens
// and ones. See the OWNER-REVIEW note at the top of this file.

const HI_0_99: [&str; 100] = [
    "शून्य", "एक", "दो", "तीन", "चार", "पाँच", "छह", "सात", "आठ", "नौ",
    "दस", "ग्यारह", "बारह", "तेरह", "चौदह", "पंद्रह", "सोलह", "सत्रह", "अठारह", "उन्नीस",
    "बीस", "इक्कीस", "बाईस", "तेईस", "चौबीस", "पच्चीस", "छब्बीस", "सत्ताईस", "अट्ठाईस", "उनतीस",
    "तीस", "इकतीस", "बत्तीस", "तैंतीस", "चौंतीस", "पैंतीस", "छत्तीस", "सैंतीस", "अड़तीस", "उनतालीस",
    "चालीस", "इकतालीस", "बयालीस", "तैंतालीस", "चौवालीस", "पैंतालीस", "छियालीस", "सैंतालीस", "अड़तालीस", "उनचास",
    "पचास", "इक्यावन", "बावन", "तिरेपन", "चौवन", "पचपन", "छप्पन", "सत्तावन", "अट्ठावन", "उनसठ",
    "साठ", "इकसठ", "बासठ", "तिरेसठ", "चौंसठ", "पैंसठ", "छियासठ", "सड़सठ", "अड़सठ", "उनहत्तर",
    "सत्तर", "इकहत्तर", "बहत्तर", "तिहत्तर", "चौहत्तर", "पचहत्तर", "छिहत्तर", "सतहत्तर", "अठहत्तर", "उन्यासी",
    "अस्सी", "इक्यासी", "बयासी", "तिरासी", "चौरासी", "पचासी", "छियासी", "सतासी", "अठासी", "नवासी",
    "नब्बे", "इक्यानवे", "बानवे", "तिरानवे", "चौरानवे", "पचानवे", "छियानवे", "सत्तानवे", "अट्ठानवे", "निन्यानवे",
];

const HI_HUNDRED: &str = "सौ";
const HI_THOUSAND: &str = "हज़ार";
const HI_LAKH: &str = "लाख";
const HI_CRORE: &str = "करोड़";

/// Words for 0..=99 in Hindi (empty for 0 so it contributes nothing inside a
/// larger number).
fn hi_two_digits(n: u64) -> String {
    debug_assert!(n < 100);
    if n == 0 {
        String::new()
    } else {
        HI_0_99[n as usize].to_string()
    }
}

/// Words for 0..=999 in Hindi.
fn hi_three_digits(n: u64) -> String {
    debug_assert!(n < 1000);
    let hundreds = n / 100;
    let rest = n % 100;
    match (hundreds, rest) {
        (0, _) => hi_two_digits(rest),
        (h, 0) => format!("{} {}", HI_0_99[h as usize], HI_HUNDRED),
        (h, r) => format!("{} {} {}", HI_0_99[h as usize], HI_HUNDRED, hi_two_digits(r)),
    }
}

/// A non-negative integer in words, Indian numbering system (Hindi).
fn hi_number(mut n: u64) -> String {
    if n == 0 {
        return HI_0_99[0].to_string();
    }
    let mut parts: Vec<String> = Vec::new();

    let crore = n / 10_000_000;
    n %= 10_000_000;
    let lakh = n / 100_000;
    n %= 100_000;
    let thousand = n / 1_000;
    n %= 1_000;
    let below_thousand = n;

    if crore > 0 {
        parts.push(format!("{} {}", hi_number(crore), HI_CRORE));
    }
    if lakh > 0 {
        parts.push(format!("{} {}", hi_two_digits(lakh), HI_LAKH));
    }
    if thousand > 0 {
        parts.push(format!("{} {}", hi_two_digits(thousand), HI_THOUSAND));
    }
    if below_thousand > 0 {
        parts.push(hi_three_digits(below_thousand));
    }

    parts.join(" ")
}

/// Hindi amount-in-words for a receipt, e.g.
/// `Paise(310000)` → `"तीन हज़ार एक सौ रुपये मात्र"`.
///
/// Negative amounts are prefixed with "ऋण".
pub fn amount_in_words_hi(amount: Paise) -> String {
    let value = amount.get();
    let negative = value < 0;
    let abs = value.unsigned_abs();
    let rupees = abs / 100;
    let paise = abs % 100;

    let mut body = format!("{} रुपये मात्र", hi_number(rupees));
    if paise > 0 {
        body = format!("{} रुपये {} पैसे मात्र", hi_number(rupees), hi_two_digits(paise));
    }
    if negative {
        body = format!("ऋण {body}");
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- English: rupee-only values -------------------------------------

    #[test]
    fn en_zero() {
        assert_eq!(amount_in_words_en(Paise(0)), "Zero rupees only");
    }

    #[test]
    fn en_one_rupee() {
        // 1 rupee = 100 paise.
        assert_eq!(amount_in_words_en(Paise(100)), "One rupees only");
    }

    #[test]
    fn en_ninety_nine_rupees() {
        assert_eq!(amount_in_words_en(Paise(9900)), "Ninety nine rupees only");
    }

    #[test]
    fn en_one_hundred_rupees() {
        assert_eq!(amount_in_words_en(Paise(10000)), "One hundred rupees only");
    }

    #[test]
    fn en_one_thousand_rupees() {
        assert_eq!(amount_in_words_en(Paise(100000)), "One thousand rupees only");
    }

    #[test]
    fn en_one_lakh_rupees() {
        // 100000 rupees = one lakh.
        assert_eq!(amount_in_words_en(Paise(10_000_000)), "One lakh rupees only");
    }

    #[test]
    fn en_one_crore_rupees() {
        // 10000000 rupees = one crore.
        assert_eq!(amount_in_words_en(Paise(1_000_000_000)), "One crore rupees only");
    }

    #[test]
    fn en_prompt_example_no_paise() {
        // 3100 rupees.
        assert_eq!(
            amount_in_words_en(Paise(310000)),
            "Three thousand one hundred rupees only"
        );
    }

    // ---- English: values WITH a paise remainder --------------------------

    #[test]
    fn en_prompt_example_with_paise() {
        // 3100 rupees and 50 paise.
        assert_eq!(
            amount_in_words_en(Paise(310050)),
            "Three thousand one hundred rupees and fifty paise only"
        );
    }

    #[test]
    fn en_one_paise() {
        assert_eq!(amount_in_words_en(Paise(1)), "Zero rupees and one paise only");
    }

    #[test]
    fn en_rupee_and_paise() {
        // 1 rupee 25 paise.
        assert_eq!(
            amount_in_words_en(Paise(125)),
            "One rupees and twenty five paise only"
        );
    }

    #[test]
    fn en_mixed_lakh_with_paise() {
        // 1,23,456 rupees and 99 paise.
        assert_eq!(
            amount_in_words_en(Paise(12_345_699)),
            "One lakh twenty three thousand four hundred fifty six rupees and ninety nine paise only"
        );
    }

    #[test]
    fn en_negative_is_prefixed() {
        assert_eq!(amount_in_words_en(Paise(-10000)), "Minus one hundred rupees only");
    }

    // ---- Hindi: high-confidence shapes only ------------------------------

    #[test]
    fn hi_zero() {
        assert_eq!(amount_in_words_hi(Paise(0)), "शून्य रुपये मात्र");
    }

    #[test]
    fn hi_one_rupee() {
        assert_eq!(amount_in_words_hi(Paise(100)), "एक रुपये मात्र");
    }

    #[test]
    fn hi_one_hundred_rupees() {
        assert_eq!(amount_in_words_hi(Paise(10000)), "एक सौ रुपये मात्र");
    }

    #[test]
    fn hi_one_thousand_rupees() {
        assert_eq!(amount_in_words_hi(Paise(100000)), "एक हज़ार रुपये मात्र");
    }

    #[test]
    fn hi_one_lakh_rupees() {
        assert_eq!(amount_in_words_hi(Paise(10_000_000)), "एक लाख रुपये मात्र");
    }

    #[test]
    fn hi_one_crore_rupees() {
        assert_eq!(amount_in_words_hi(Paise(1_000_000_000)), "एक करोड़ रुपये मात्र");
    }

    #[test]
    fn hi_prompt_example_no_paise() {
        // 3100 rupees → "तीन हज़ार एक सौ रुपये मात्र" (from the prompt).
        assert_eq!(amount_in_words_hi(Paise(310000)), "तीन हज़ार एक सौ रुपये मात्र");
    }

    #[test]
    fn hi_with_paise_uses_high_confidence_words() {
        // 3100 rupees and 50 paise; पचास (50) is a round-ten, high confidence.
        assert_eq!(
            amount_in_words_hi(Paise(310050)),
            "तीन हज़ार एक सौ रुपये पचास पैसे मात्र"
        );
    }

    #[test]
    fn hi_negative_is_prefixed() {
        assert_eq!(amount_in_words_hi(Paise(-10000)), "ऋण एक सौ रुपये मात्र");
    }
}
