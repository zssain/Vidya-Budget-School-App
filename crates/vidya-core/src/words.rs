const ONES: [&str; 20] = [
    "Zero",
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "Six",
    "Seven",
    "Eight",
    "Nine",
    "Ten",
    "Eleven",
    "Twelve",
    "Thirteen",
    "Fourteen",
    "Fifteen",
    "Sixteen",
    "Seventeen",
    "Eighteen",
    "Nineteen",
];
const TENS: [&str; 10] = [
    "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
];

fn below_hundred(n: u64) -> String {
    if n < 20 {
        ONES[n as usize].to_owned()
    } else if n.is_multiple_of(10) {
        TENS[(n / 10) as usize].to_owned()
    } else {
        format!("{} {}", TENS[(n / 10) as usize], ONES[(n % 10) as usize])
    }
}

fn below_thousand(n: u64) -> String {
    if n < 100 {
        below_hundred(n)
    } else if n.is_multiple_of(100) {
        format!("{} Hundred", ONES[(n / 100) as usize])
    } else {
        format!("{} Hundred {}", ONES[(n / 100) as usize], below_hundred(n % 100))
    }
}

/// Converts a non-negative amount to English words using Indian units.
pub fn amount_in_words_en(n: u64) -> String {
    if n == 0 {
        return "Zero".to_owned();
    }
    let units = [(10_000_000, "Crore"), (100_000, "Lakh"), (1_000, "Thousand")];
    let mut remaining = n;
    let mut parts = Vec::new();
    for (value, name) in units {
        if remaining >= value {
            parts.push(format!("{} {name}", amount_in_words_en(remaining / value)));
            remaining %= value;
        }
    }
    if remaining > 0 {
        parts.push(below_thousand(remaining));
    }
    parts.join(" ")
}

/// Converts an amount to the wording printed on an English receipt.
pub fn receipt_words_en(n: u64) -> String {
    format!("Rupees {} Only", amount_in_words_en(n))
}
