//! Currency presentation data — the Rust-side mirror of the frontend's
//! `src/lib/currency.ts` (`CURRENCIES`). Static reference data, not user
//! data, so it lives as a constant here rather than a table
//! (database-schema.md has no `currencies`); the frontend keeps its own copy
//! because it formats money for the screen, this one exists because the PDF
//! is composed in Rust and must not depend on anything the webview computed.
//!
//! **Keep the two lists in sync** — adding a currency in one place without
//! the other means the screen and the PDF disagree about a symbol or a
//! decimal count.
//!
//! Only India's tax *model* is implemented (see `domain::calculation`); this
//! module is purely about how an amount is *displayed*, and is deliberately
//! country-agnostic so multi-country support later needs no change here.

/// `(ISO 4217 code, symbol, minor-unit exponent)`.
const CURRENCIES: &[(&str, &str, u32)] = &[
    ("INR", "₹", 2),
    ("USD", "$", 2),
    ("EUR", "€", 2),
    ("GBP", "£", 2),
    ("AUD", "A$", 2),
    ("CAD", "C$", 2),
    ("NZD", "NZ$", 2),
    ("SGD", "S$", 2),
    ("HKD", "HK$", 2),
    ("AED", "د.إ", 2),
    ("SAR", "﷼", 2),
    ("QAR", "ر.ق", 2),
    ("BHD", ".د.ب", 3),
    ("KWD", "د.ك", 3),
    ("OMR", "ر.ع.", 3),
    ("JOD", "د.ا", 3),
    ("TND", "د.ت", 3),
    ("JPY", "¥", 0),
    ("KRW", "₩", 0),
    ("VND", "₫", 0),
    ("IDR", "Rp", 0),
    ("CLP", "$", 0),
    ("ISK", "kr", 0),
    ("UGX", "USh", 0),
    ("PYG", "₲", 0),
    ("CNY", "¥", 2),
    ("CHF", "CHF", 2),
    ("SEK", "kr", 2),
    ("NOK", "kr", 2),
    ("DKK", "kr", 2),
    ("PLN", "zł", 2),
    ("CZK", "Kč", 2),
    ("HUF", "Ft", 2),
    ("RON", "lei", 2),
    ("TRY", "₺", 2),
    ("RUB", "₽", 2),
    ("ZAR", "R", 2),
    ("NGN", "₦", 2),
    ("EGP", "£", 2),
    ("KES", "KSh", 2),
    ("GHS", "₵", 2),
    ("PKR", "₨", 2),
    ("BDT", "৳", 2),
    ("LKR", "₨", 2),
    ("NPR", "₨", 2),
    ("MMK", "K", 2),
    ("THB", "฿", 2),
    ("MYR", "RM", 2),
    ("PHP", "₱", 2),
    ("MXN", "$", 2),
    ("BRL", "R$", 2),
    ("ARS", "$", 2),
    ("COP", "$", 2),
    ("PEN", "S/", 2),
    ("ILS", "₪", 2),
];

/// Currencies conventionally grouped Indian-style (`12,34,567.89`) rather
/// than in uniform thousands (`1,234,567.89`). Keyed off the currency rather
/// than the country because the currency is what the amount is denominated
/// in — an INR figure reads as lakhs/crores wherever it is printed.
const INDIAN_GROUPED: &[&str] = &["INR", "PKR", "LKR", "NPR", "BDT"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyMeta {
    /// `None` for a code this table doesn't know — callers fall back to
    /// printing the ISO code itself, matching the frontend's `currencyMeta`.
    pub symbol: Option<&'static str>,
    pub decimals: u32,
    /// Whether amounts in this currency read as lakhs/crores.
    pub indian_grouping: bool,
}

/// Unknown codes degrade to "no symbol, 2 decimals" rather than failing, so a
/// currency added to the frontend list but not yet here still renders as
/// `XYZ 1,234.00` instead of breaking the invoice.
pub fn currency_meta(code: &str) -> CurrencyMeta {
    let indian_grouping = INDIAN_GROUPED.contains(&code);
    match CURRENCIES.iter().find(|(c, _, _)| *c == code) {
        Some((_, symbol, decimals)) => CurrencyMeta {
            symbol: Some(symbol),
            decimals: *decimals,
            indian_grouping,
        },
        None => CurrencyMeta {
            symbol: None,
            decimals: 2,
            indian_grouping,
        },
    }
}

/// Formats a minor-unit amount as a grouped decimal string **without any
/// currency symbol or code** — the caller decides which of those to prefix,
/// because in the PDF that choice depends on whether the embedded font can
/// actually draw the symbol (`infrastructure::pdf`).
///
/// Negative amounts get a leading `-`; grouping is applied to the integer
/// part only. Pure integer arithmetic throughout, per calculation-engine.md
/// §1 — no floats touch a money value anywhere in this crate.
pub fn format_minor(minor: i64, decimals: u32, indian_grouping: bool) -> String {
    let negative = minor < 0;
    let abs = minor.unsigned_abs();
    let scale = 10u64.pow(decimals);
    let whole = abs / scale;
    let frac = abs % scale;

    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&group_digits(whole, indian_grouping));
    if decimals > 0 {
        out.push('.');
        out.push_str(&format!("{frac:0width$}", width = decimals as usize));
    }
    out
}

fn group_digits(value: u64, indian_grouping: bool) -> String {
    let digits = value.to_string();
    if digits.len() <= 3 {
        return digits;
    }
    let bytes = digits.as_bytes();
    // Walk right-to-left. Western grouping inserts a separator every 3
    // digits; Indian grouping does so after the first 3, then every 2.
    let mut out: Vec<u8> = Vec::with_capacity(digits.len() + digits.len() / 2);
    let mut since_separator = 0usize;
    let mut group_size = 3usize;
    for (i, byte) in bytes.iter().enumerate().rev() {
        out.push(*byte);
        since_separator += 1;
        let more_digits_remain = i > 0;
        if since_separator == group_size && more_digits_remain {
            out.push(b',');
            since_separator = 0;
            if indian_grouping {
                group_size = 2;
            }
        }
    }
    out.reverse();
    String::from_utf8(out).expect("grouping only ever appends ASCII digits and commas")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_currency_metadata_matches_the_frontend_table() {
        assert_eq!(currency_meta("INR").symbol, Some("₹"));
        assert_eq!(currency_meta("INR").decimals, 2);
        assert_eq!(currency_meta("JPY").decimals, 0);
        assert_eq!(currency_meta("KWD").decimals, 3);
    }

    #[test]
    fn unknown_currency_falls_back_to_two_decimals_and_no_symbol() {
        let meta = currency_meta("ZZZ");
        assert_eq!(meta.symbol, None);
        assert_eq!(meta.decimals, 2);
    }

    #[test]
    fn formats_two_decimal_currency_with_western_grouping() {
        assert_eq!(format_minor(123_456_789, 2, false), "1,234,567.89");
        assert_eq!(format_minor(0, 2, false), "0.00");
        assert_eq!(format_minor(99, 2, false), "0.99");
        assert_eq!(format_minor(100_000, 2, false), "1,000.00");
    }

    #[test]
    fn formats_with_indian_grouping() {
        assert_eq!(format_minor(123_456_789, 2, true), "12,34,567.89");
        assert_eq!(format_minor(100_000, 2, true), "1,000.00");
        assert_eq!(format_minor(1_000_000, 2, true), "10,000.00");
        assert_eq!(format_minor(10_000_000_000, 2, true), "10,00,00,000.00");
    }

    #[test]
    fn formats_zero_and_three_decimal_currencies() {
        assert_eq!(format_minor(1_234_567, 0, false), "1,234,567");
        assert_eq!(format_minor(1_234_567, 3, false), "1,234.567");
    }

    #[test]
    fn formats_negative_amounts() {
        // Overpayment/credit lines are the only place a negative reaches the PDF.
        assert_eq!(format_minor(-150_000, 2, false), "-1,500.00");
    }

    #[test]
    fn grouping_style_follows_the_currency() {
        assert!(currency_meta("INR").indian_grouping);
        assert!(!currency_meta("USD").indian_grouping);
    }
}
