//! The invoice as a *printable document* — every string the PDF shows,
//! already formatted, with nothing left for the renderer to decide except
//! where to put it.
//!
//! This exists so that `infrastructure::pdf` stays a pure layout engine: it
//! never formats a date, never divides a money value, never decides whether
//! a tax line is CGST or IGST. Those are business-rule decisions
//! (calculation-engine.md §5, database-schema.md §6), so they belong here in
//! `domain`, where they are testable without producing a single byte of PDF
//! — and where architecture.md rule 3 keeps them free of any PDF-library
//! dependency.
//!
//! One deliberate exception: money amounts are formatted as *numbers only*
//! (`1,234.00`), with no symbol or code attached. Whether an amount can be
//! printed as `₹1,234.00` or has to degrade to `INR 1,234.00` depends on
//! which glyphs the embedded font actually has, which only the renderer
//! knows. `currency` below carries both options; the renderer picks one and
//! prefixes it uniformly.

use chrono::NaiveDate;

use super::business::Business;
use super::calculation::split_gst;
use super::currency::{currency_meta, format_minor};
use super::customer::Customer;
use super::invoice::{DiscountType, Invoice, InvoiceStatus};
use super::invoice_line_item::InvoiceLineItem;
use super::settings::Settings;

/// The tax model whose vocabulary the totals block uses. Only India's GST is
/// implemented (`.ai/product.md` V1 scope, and the "known gaps" note in
/// `.ai/progress/CURRENT.md`); everywhere else a single neutral "Tax" line is
/// printed rather than inventing a regime this app does not actually compute.
const INDIA_COUNTRY_CODE: &str = "IN";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyDisplay {
    /// e.g. `₹` — `None` when this app's currency table has no symbol for the code.
    pub symbol: Option<String>,
    /// e.g. `INR` — always available, and the fallback when the symbol has no glyph.
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfParty {
    pub name: String,
    /// Address split on newlines, blank lines dropped — the renderer draws
    /// one line per entry and never has to parse anything.
    pub address_lines: Vec<String>,
    /// Pre-labelled one-per-line contact rows, e.g. `("GSTIN", "22AAAAA0000A1Z5")`.
    pub details: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfLineItem {
    pub position: usize,
    pub description: String,
    /// `2` or `2.500` — trailing zeros dropped, mirroring the editor's
    /// `formatThousandthsAsQuantity`.
    pub quantity: String,
    pub unit: String,
    /// Money, number-only (see the module note).
    pub rate: String,
    /// `18%`, `18.5%`, or empty when the line carries no tax.
    pub tax_rate: String,
    /// Money, number-only.
    pub amount: String,
    /// `Less 10%` / `Less 100.00` when this line has its own discount.
    pub discount_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotalWeight {
    /// A component of the total — subtotal, discount, each tax line.
    Normal,
    /// The invoice total, and the balance due: drawn heavier, above a rule.
    Strong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfTotalRow {
    pub label: String,
    /// Money, number-only.
    pub amount: String,
    pub weight: TotalWeight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoicePdfDocument {
    /// `TAX INVOICE` where a GSTIN is on the document, else `INVOICE`.
    pub title: String,
    /// Diagonal stamp for a document that is not a live demand for payment.
    pub watermark: Option<String>,
    pub currency: CurrencyDisplay,
    pub business: PdfParty,
    pub logo_path: Option<String>,
    pub customer: Option<PdfParty>,
    /// `Invoice No.` / `Date` / `Due Date`, in print order.
    pub meta: Vec<(String, String)>,
    pub line_items: Vec<PdfLineItem>,
    pub totals: Vec<PdfTotalRow>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    /// Bank details / UPI, printed in the payment-instructions block.
    pub payment_details: Vec<(String, String)>,
    /// Shown under the totals when an edit dropped the total below what was
    /// already paid (user-flows.md's "Editing an issued invoice" rule) — the
    /// PDF states it rather than silently showing a negative balance.
    pub overpayment_note: Option<String>,
}

/// Everything the document is assembled from. `live_business`/`live_customer`
/// are the *unissued* fallbacks: a `DRAFT` has no snapshot yet
/// (user-flows.md §5 — the snapshot is taken at Issue), so previewing one has
/// to read the current records. An issued invoice ignores them entirely and
/// prints its own frozen snapshot, which is the whole point of taking one.
pub struct InvoicePdfInput<'a> {
    pub invoice: &'a Invoice,
    pub line_items: &'a [InvoiceLineItem],
    pub settings: &'a Settings,
    pub live_business: Option<&'a Business>,
    pub live_customer: Option<&'a Customer>,
    pub amount_paid_minor: i64,
}

pub fn build_invoice_pdf_document(input: InvoicePdfInput<'_>) -> InvoicePdfDocument {
    let InvoicePdfInput {
        invoice,
        line_items,
        settings,
        live_business,
        live_customer,
        amount_paid_minor,
    } = input;

    let meta_currency = currency_meta(&settings.currency_code);
    let money =
        |minor: i64| format_minor(minor, meta_currency.decimals, meta_currency.indian_grouping);

    let business = build_business_party(invoice, live_business);
    let customer = build_customer_party(invoice, live_customer);

    let has_gstin = business.details.iter().any(|(label, _)| label == "GSTIN");
    let title = if has_gstin { "TAX INVOICE" } else { "INVOICE" };

    InvoicePdfDocument {
        title: title.to_string(),
        watermark: watermark_for(invoice.status),
        currency: CurrencyDisplay {
            symbol: meta_currency.symbol.map(str::to_string),
            code: settings.currency_code.clone(),
        },
        logo_path: logo_path(invoice, live_business),
        business,
        customer,
        meta: build_meta(invoice, settings),
        line_items: build_line_items(line_items, &money),
        totals: build_totals(invoice, settings, amount_paid_minor, &money),
        notes: non_empty(invoice.notes.as_deref()),
        terms: non_empty(invoice.terms.as_deref()),
        payment_details: build_payment_details(invoice, live_business),
        overpayment_note: build_overpayment_note(invoice, amount_paid_minor, &money),
    }
}

fn watermark_for(status: InvoiceStatus) -> Option<String> {
    match status {
        InvoiceStatus::Draft => Some("DRAFT".to_string()),
        InvoiceStatus::Cancelled => Some("CANCELLED".to_string()),
        InvoiceStatus::Issued | InvoiceStatus::PartiallyPaid | InvoiceStatus::Paid => None,
    }
}

/// Snapshot first, live record second — an issued invoice must print what it
/// froze, never what the master record says today.
fn snapshot_or_live<'a>(snapshot: Option<&'a str>, live: Option<&'a str>) -> Option<&'a str> {
    snapshot.or(live).filter(|s| !s.trim().is_empty())
}

fn logo_path(invoice: &Invoice, live_business: Option<&Business>) -> Option<String> {
    snapshot_or_live(
        invoice.business_snapshot_logo_path.as_deref(),
        live_business.and_then(|b| b.logo_path.as_deref()),
    )
    .map(str::to_string)
}

fn build_business_party(invoice: &Invoice, live: Option<&Business>) -> PdfParty {
    let name = snapshot_or_live(
        invoice.business_snapshot_name.as_deref(),
        live.map(|b| b.name.as_str()),
    )
    .unwrap_or("")
    .to_string();

    let address = snapshot_or_live(
        invoice.business_snapshot_address.as_deref(),
        live.and_then(|b| b.address.as_deref()),
    );

    let mut details = Vec::new();
    if let Some(gstin) = snapshot_or_live(
        invoice.business_snapshot_gstin.as_deref(),
        live.and_then(|b| b.gstin.as_deref()),
    ) {
        details.push(("GSTIN".to_string(), gstin.to_string()));
    }
    if let Some(phone) = snapshot_or_live(
        invoice.business_snapshot_phone.as_deref(),
        live.and_then(|b| b.phone.as_deref()),
    ) {
        details.push(("Phone".to_string(), phone.to_string()));
    }
    if let Some(email) = snapshot_or_live(
        invoice.business_snapshot_email.as_deref(),
        live.and_then(|b| b.email.as_deref()),
    ) {
        details.push(("Email".to_string(), email.to_string()));
    }

    PdfParty {
        name,
        address_lines: split_address(address),
        details,
    }
}

fn build_customer_party(invoice: &Invoice, live: Option<&Customer>) -> Option<PdfParty> {
    let name = snapshot_or_live(
        invoice.customer_snapshot_name.as_deref(),
        live.map(|c| c.name.as_str()),
    )?;

    let address = snapshot_or_live(
        invoice.customer_snapshot_address.as_deref(),
        live.and_then(|c| c.address.as_deref()),
    );

    let mut details = Vec::new();
    if let Some(gstin) = snapshot_or_live(
        invoice.customer_snapshot_gstin.as_deref(),
        live.and_then(|c| c.gstin.as_deref()),
    ) {
        details.push(("GSTIN".to_string(), gstin.to_string()));
    }
    if let Some(phone) = snapshot_or_live(
        invoice.customer_snapshot_phone.as_deref(),
        live.and_then(|c| c.phone.as_deref()),
    ) {
        details.push(("Phone".to_string(), phone.to_string()));
    }
    if let Some(email) = snapshot_or_live(
        invoice.customer_snapshot_email.as_deref(),
        live.and_then(|c| c.email.as_deref()),
    ) {
        details.push(("Email".to_string(), email.to_string()));
    }

    Some(PdfParty {
        name: name.to_string(),
        address_lines: split_address(address),
        details,
    })
}

fn build_payment_details(invoice: &Invoice, live: Option<&Business>) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if let Some(bank) = snapshot_or_live(
        invoice.business_snapshot_bank_details.as_deref(),
        live.and_then(|b| b.bank_details.as_deref()),
    ) {
        rows.push(("Bank Details".to_string(), bank.to_string()));
    }
    if let Some(upi) = snapshot_or_live(
        invoice.business_snapshot_upi_id.as_deref(),
        live.and_then(|b| b.upi_id.as_deref()),
    ) {
        rows.push(("UPI ID".to_string(), upi.to_string()));
    }
    rows
}

fn build_meta(invoice: &Invoice, settings: &Settings) -> Vec<(String, String)> {
    let mut meta = Vec::new();
    // A draft has no number yet — printing an empty row would look like a
    // missing number rather than an unissued document, so say so.
    let number = invoice
        .invoice_number
        .clone()
        .unwrap_or_else(|| "(not yet issued)".to_string());
    meta.push(("Invoice No.".to_string(), number));
    meta.push((
        "Date".to_string(),
        format_date(invoice.invoice_date, &settings.date_format),
    ));
    if let Some(due) = invoice.due_date {
        meta.push((
            "Due Date".to_string(),
            format_date(due, &settings.date_format),
        ));
    }
    meta
}

fn build_line_items(
    line_items: &[InvoiceLineItem],
    money: &dyn Fn(i64) -> String,
) -> Vec<PdfLineItem> {
    line_items
        .iter()
        .enumerate()
        .map(|(index, li)| PdfLineItem {
            position: index + 1,
            description: li.description.clone(),
            quantity: format_quantity(li.quantity_thousandths),
            unit: li.unit.clone(),
            rate: money(li.unit_price_minor),
            tax_rate: if li.tax_rate_basis_points == 0 {
                String::new()
            } else {
                format_basis_points(li.tax_rate_basis_points)
            },
            amount: money(li.line_total_minor),
            discount_note: line_discount_note(li, money),
        })
        .collect()
}

fn line_discount_note(li: &InvoiceLineItem, money: &dyn Fn(i64) -> String) -> Option<String> {
    let value = li.line_discount_value?;
    if value == 0 {
        return None;
    }
    match li.line_discount_type? {
        DiscountType::Percentage => Some(format!("Less {}", format_basis_points(value))),
        DiscountType::Amount => Some(format!("Less {}", money(value))),
    }
}

fn build_totals(
    invoice: &Invoice,
    settings: &Settings,
    amount_paid_minor: i64,
    money: &dyn Fn(i64) -> String,
) -> Vec<PdfTotalRow> {
    let mut rows = vec![PdfTotalRow {
        label: "Subtotal".to_string(),
        amount: money(invoice.subtotal_minor),
        weight: TotalWeight::Normal,
    }];

    if invoice.discount_amount_minor != 0 {
        let label = match (invoice.discount_type, invoice.discount_value) {
            (Some(DiscountType::Percentage), Some(value)) => {
                format!("Discount ({})", format_basis_points(value))
            }
            _ => "Discount".to_string(),
        };
        rows.push(PdfTotalRow {
            label,
            // Negative so it reads as a deduction without the renderer
            // having to know which rows subtract.
            amount: money(-invoice.discount_amount_minor),
            weight: TotalWeight::Normal,
        });
    }

    rows.extend(tax_rows(invoice, settings, money));

    rows.push(PdfTotalRow {
        label: "Total".to_string(),
        amount: money(invoice.total_minor),
        weight: TotalWeight::Strong,
    });

    if amount_paid_minor != 0 {
        rows.push(PdfTotalRow {
            label: "Amount Paid".to_string(),
            amount: money(-amount_paid_minor),
            weight: TotalWeight::Normal,
        });
        rows.push(PdfTotalRow {
            label: "Balance Due".to_string(),
            // Clamped at zero: an overpayment is called out in its own note
            // rather than printed as a negative amount due.
            amount: money((invoice.total_minor - amount_paid_minor).max(0)),
            weight: TotalWeight::Strong,
        });
    }

    rows
}

/// calculation-engine.md §5 — one blended split of the already-final
/// `tax_amount_minor`, at the invoice level. Outside India the same total is
/// printed under a single neutral label, because this app does not compute
/// any other country's tax breakdown (see `INDIA_COUNTRY_CODE`).
fn tax_rows(
    invoice: &Invoice,
    settings: &Settings,
    money: &dyn Fn(i64) -> String,
) -> Vec<PdfTotalRow> {
    if invoice.tax_amount_minor == 0 {
        return Vec::new();
    }
    if settings.country_code != INDIA_COUNTRY_CODE {
        return vec![PdfTotalRow {
            label: "Tax".to_string(),
            amount: money(invoice.tax_amount_minor),
            weight: TotalWeight::Normal,
        }];
    }

    let split = split_gst(invoice.tax_amount_minor, invoice.is_interstate);
    if invoice.is_interstate {
        vec![PdfTotalRow {
            label: "IGST".to_string(),
            amount: money(split.igst),
            weight: TotalWeight::Normal,
        }]
    } else {
        vec![
            PdfTotalRow {
                label: "CGST".to_string(),
                amount: money(split.cgst),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "SGST".to_string(),
                amount: money(split.sgst),
                weight: TotalWeight::Normal,
            },
        ]
    }
}

fn build_overpayment_note(
    invoice: &Invoice,
    amount_paid_minor: i64,
    money: &dyn Fn(i64) -> String,
) -> Option<String> {
    let overpaid = amount_paid_minor - invoice.total_minor;
    (overpaid > 0).then(|| format!("Overpaid by {}", money(overpaid)))
}

fn split_address(address: Option<&str>) -> Vec<String> {
    address
        .map(|a| {
            a.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Mirrors the editor's `formatThousandthsAsQuantity` — `2500` prints as
/// `2.5`, not `2.500`, so whole quantities stay readable.
pub fn format_quantity(thousandths: i64) -> String {
    let negative = thousandths < 0;
    let abs = thousandths.unsigned_abs();
    let whole = abs / 1000;
    let frac = abs % 1000;
    let sign = if negative { "-" } else { "" };
    if frac == 0 {
        return format!("{sign}{whole}");
    }
    let mut frac_digits = format!("{frac:03}");
    while frac_digits.ends_with('0') {
        frac_digits.pop();
    }
    format!("{sign}{whole}.{frac_digits}")
}

/// `1800` -> `18%`, `1825` -> `18.25%`. Basis points are how every rate is
/// stored (database-schema.md §6); this is the only place they become text.
pub fn format_basis_points(basis_points: i64) -> String {
    let whole = basis_points / 100;
    let frac = basis_points % 100;
    if frac == 0 {
        return format!("{whole}%");
    }
    let mut frac_digits = format!("{:02}", frac.abs());
    while frac_digits.ends_with('0') {
        frac_digits.pop();
    }
    format!("{whole}.{frac_digits}%")
}

/// Renders `date` per the user's `settings.date_format` pattern. Supports the
/// tokens the Settings field actually offers — `YYYY`, `YY`, `MMM`, `MM`,
/// `DD` — and leaves anything else in the pattern untouched, so a separator
/// the user typed (`/`, `-`, `.`, a space) comes through as written.
pub fn format_date(date: NaiveDate, pattern: &str) -> String {
    use chrono::Datelike;

    const MONTH_ABBREVIATIONS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let mut out = String::with_capacity(pattern.len() + 4);
    let mut rest = pattern;
    while !rest.is_empty() {
        // Longest token first: `YYYY` before `YY`, `MMM` before `MM`.
        if let Some(tail) = rest.strip_prefix("YYYY") {
            out.push_str(&format!("{:04}", date.year()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("YY") {
            out.push_str(&format!("{:02}", date.year().rem_euclid(100)));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("MMM") {
            out.push_str(MONTH_ABBREVIATIONS[(date.month0()) as usize]);
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("MM") {
            out.push_str(&format!("{:02}", date.month()));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("DD") {
            out.push_str(&format!("{:02}", date.day()));
            rest = tail;
        } else {
            let ch = rest.chars().next().expect("rest is non-empty");
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::customer::CustomerStatus;

    fn settings(country: &str, currency: &str) -> Settings {
        Settings {
            country_code: country.to_string(),
            currency_code: currency.to_string(),
            date_format: "DD/MM/YYYY".to_string(),
            invoice_number_format: "INV-{YYYY}-{####}".to_string(),
            default_due_days: 15,
            default_tax_rate_id: None,
        }
    }

    fn issued_invoice() -> Invoice {
        Invoice {
            id: 7,
            invoice_number: Some("INV-2026-0007".to_string()),
            invoice_number_is_custom: false,
            status: InvoiceStatus::Issued,
            customer_id: Some(3),
            customer_snapshot_name: Some("Frozen Customer".to_string()),
            customer_snapshot_phone: None,
            customer_snapshot_email: None,
            customer_snapshot_address: Some("12 Old Street\n\nBengaluru 560001".to_string()),
            customer_snapshot_gstin: Some("29AAAAA0000A1Z5".to_string()),
            business_snapshot_name: Some("Frozen Business".to_string()),
            business_snapshot_address: Some("1 Mill Road".to_string()),
            business_snapshot_gstin: Some("29BBBBB1111B1Z5".to_string()),
            business_snapshot_phone: Some("+91 99999 00000".to_string()),
            business_snapshot_email: None,
            business_snapshot_bank_details: Some("Acme Bank • 0001".to_string()),
            business_snapshot_upi_id: Some("acme@upi".to_string()),
            business_snapshot_logo_path: Some("/snapshot/logo.png".to_string()),
            is_interstate: false,
            invoice_date: NaiveDate::from_ymd_opt(2026, 3, 9).unwrap(),
            due_date: Some(NaiveDate::from_ymd_opt(2026, 3, 24).unwrap()),
            notes: Some("  ".to_string()),
            terms: Some("Payment within 15 days.".to_string()),
            discount_type: None,
            discount_value: None,
            subtotal_minor: 200_000,
            discount_amount_minor: 0,
            tax_amount_minor: 36_000,
            total_minor: 236_000,
            issued_at: None,
            cancelled_at: None,
            cancel_reason: None,
        }
    }

    fn draft_invoice() -> Invoice {
        Invoice {
            id: 12,
            invoice_number: None,
            status: InvoiceStatus::Draft,
            customer_snapshot_name: None,
            customer_snapshot_address: None,
            customer_snapshot_gstin: None,
            business_snapshot_name: None,
            business_snapshot_address: None,
            business_snapshot_gstin: None,
            business_snapshot_phone: None,
            business_snapshot_bank_details: None,
            business_snapshot_upi_id: None,
            business_snapshot_logo_path: None,
            due_date: None,
            terms: None,
            ..issued_invoice()
        }
    }

    fn live_business() -> Business {
        Business {
            name: "Live Business".to_string(),
            logo_path: Some("/live/logo.png".to_string()),
            address: Some("2 New Road".to_string()),
            phone: None,
            email: Some("hi@live.test".to_string()),
            gstin: Some("29CCCCC2222C1Z5".to_string()),
            bank_details: Some("Live Bank • 0002".to_string()),
            upi_id: None,
        }
    }

    fn live_customer() -> Customer {
        Customer {
            id: 3,
            name: "Live Customer".to_string(),
            phone: Some("+91 88888 11111".to_string()),
            email: None,
            address: Some("9 Current Lane".to_string()),
            gstin: None,
            status: CustomerStatus::Active,
        }
    }

    fn line_item(description: &str, tax_basis_points: i64) -> InvoiceLineItem {
        InvoiceLineItem {
            id: 1,
            product_id: None,
            description: description.to_string(),
            unit: "pcs".to_string(),
            quantity_thousandths: 2_000,
            unit_price_minor: 100_000,
            line_discount_type: None,
            line_discount_value: None,
            tax_rate_id: None,
            tax_rate_basis_points: tax_basis_points,
            line_subtotal_minor: 200_000,
            line_discount_amount_minor: 0,
            invoice_discount_amount_minor: 0,
            taxable_amount_minor: 200_000,
            line_tax_minor: 36_000,
            line_total_minor: 236_000,
            sort_order: 0,
        }
    }

    fn build(
        invoice: &Invoice,
        line_items: &[InvoiceLineItem],
        settings: &Settings,
        business: Option<&Business>,
        customer: Option<&Customer>,
        amount_paid_minor: i64,
    ) -> InvoicePdfDocument {
        build_invoice_pdf_document(InvoicePdfInput {
            invoice,
            line_items,
            settings,
            live_business: business,
            live_customer: customer,
            amount_paid_minor,
        })
    }

    fn total_labelled<'a>(doc: &'a InvoicePdfDocument, label: &str) -> Option<&'a PdfTotalRow> {
        doc.totals.iter().find(|row| row.label == label)
    }

    #[test]
    fn an_issued_invoice_prints_its_snapshot_not_the_live_records() {
        let invoice = issued_invoice();
        let business = live_business();
        let customer = live_customer();
        let doc = build(
            &invoice,
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            Some(&business),
            Some(&customer),
            0,
        );

        assert_eq!(doc.business.name, "Frozen Business");
        assert_eq!(doc.customer.as_ref().unwrap().name, "Frozen Customer");
        assert_eq!(doc.logo_path.as_deref(), Some("/snapshot/logo.png"));
        assert_eq!(
            doc.payment_details,
            vec![
                ("Bank Details".to_string(), "Acme Bank • 0001".to_string()),
                ("UPI ID".to_string(), "acme@upi".to_string()),
            ]
        );
    }

    #[test]
    fn a_draft_falls_back_to_the_live_records_because_it_has_no_snapshot_yet() {
        let invoice = draft_invoice();
        let business = live_business();
        let customer = live_customer();
        let doc = build(
            &invoice,
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            Some(&business),
            Some(&customer),
            0,
        );

        assert_eq!(doc.business.name, "Live Business");
        assert_eq!(doc.customer.as_ref().unwrap().name, "Live Customer");
        assert_eq!(doc.logo_path.as_deref(), Some("/live/logo.png"));
        assert_eq!(doc.watermark.as_deref(), Some("DRAFT"));
        assert_eq!(doc.meta[0].1, "(not yet issued)");
        // A draft has no due date here, so the row must be absent rather than blank.
        assert!(doc.meta.iter().all(|(label, _)| label != "Due Date"));
    }

    #[test]
    fn a_cancelled_invoice_is_stamped_and_a_live_one_is_not() {
        let mut invoice = issued_invoice();
        invoice.status = InvoiceStatus::Cancelled;
        let doc = build(&invoice, &[], &settings("IN", "INR"), None, None, 0);
        assert_eq!(doc.watermark.as_deref(), Some("CANCELLED"));

        let doc = build(
            &issued_invoice(),
            &[],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(doc.watermark, None);
    }

    #[test]
    fn intrastate_tax_splits_into_cgst_and_sgst() {
        let doc = build(
            &issued_invoice(),
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(total_labelled(&doc, "CGST").unwrap().amount, "180.00");
        assert_eq!(total_labelled(&doc, "SGST").unwrap().amount, "180.00");
        assert!(total_labelled(&doc, "IGST").is_none());
    }

    #[test]
    fn interstate_tax_is_a_single_igst_line() {
        let mut invoice = issued_invoice();
        invoice.is_interstate = true;
        let doc = build(
            &invoice,
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(total_labelled(&doc, "IGST").unwrap().amount, "360.00");
        assert!(total_labelled(&doc, "CGST").is_none());
    }

    #[test]
    fn outside_india_the_same_tax_total_prints_under_a_neutral_label() {
        // Only India's GST model is implemented — the PDF must not invent a
        // CGST/SGST breakdown for a country this app doesn't compute tax for.
        let doc = build(
            &issued_invoice(),
            &[line_item("Widget", 1800)],
            &settings("US", "USD"),
            None,
            None,
            0,
        );
        assert_eq!(total_labelled(&doc, "Tax").unwrap().amount, "360.00");
        assert!(total_labelled(&doc, "CGST").is_none());
        assert!(total_labelled(&doc, "IGST").is_none());
    }

    #[test]
    fn a_zero_tax_invoice_prints_no_tax_line_at_all() {
        let mut invoice = issued_invoice();
        invoice.tax_amount_minor = 0;
        invoice.total_minor = 200_000;
        let doc = build(
            &invoice,
            &[line_item("Widget", 0)],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert!(total_labelled(&doc, "CGST").is_none());
        assert!(total_labelled(&doc, "Tax").is_none());
        assert_eq!(doc.line_items[0].tax_rate, "");
    }

    #[test]
    fn a_percentage_discount_is_labelled_with_its_rate_and_shown_as_a_deduction() {
        let mut invoice = issued_invoice();
        invoice.discount_type = Some(DiscountType::Percentage);
        invoice.discount_value = Some(1000);
        invoice.discount_amount_minor = 20_000;
        let doc = build(&invoice, &[], &settings("IN", "INR"), None, None, 0);
        let row = total_labelled(&doc, "Discount (10%)").unwrap();
        assert_eq!(row.amount, "-200.00");
    }

    #[test]
    fn payments_add_paid_and_balance_rows() {
        let doc = build(
            &issued_invoice(),
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            None,
            None,
            100_000,
        );
        assert_eq!(
            total_labelled(&doc, "Amount Paid").unwrap().amount,
            "-1,000.00"
        );
        let balance = total_labelled(&doc, "Balance Due").unwrap();
        assert_eq!(balance.amount, "1,360.00");
        assert_eq!(balance.weight, TotalWeight::Strong);
    }

    #[test]
    fn an_overpayment_is_stated_rather_than_shown_as_a_negative_balance() {
        // user-flows.md: editing an issued invoice below what was already
        // paid surfaces the difference; it never silently adjusts a payment.
        let doc = build(
            &issued_invoice(),
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            None,
            None,
            300_000,
        );
        assert_eq!(total_labelled(&doc, "Balance Due").unwrap().amount, "0.00");
        assert_eq!(doc.overpayment_note.as_deref(), Some("Overpaid by 640.00"));
    }

    #[test]
    fn the_title_says_tax_invoice_only_when_a_gstin_is_printed() {
        let doc = build(
            &issued_invoice(),
            &[],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(doc.title, "TAX INVOICE");

        let mut invoice = issued_invoice();
        invoice.business_snapshot_gstin = None;
        let doc = build(&invoice, &[], &settings("IN", "INR"), None, None, 0);
        assert_eq!(doc.title, "INVOICE");
    }

    #[test]
    fn currency_carries_both_a_symbol_and_a_code_and_amounts_carry_neither() {
        let doc = build(
            &issued_invoice(),
            &[line_item("Widget", 1800)],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(doc.currency.symbol.as_deref(), Some("₹"));
        assert_eq!(doc.currency.code, "INR");
        assert_eq!(total_labelled(&doc, "Total").unwrap().amount, "2,360.00");
    }

    #[test]
    fn a_zero_decimal_currency_formats_without_a_decimal_point() {
        let doc = build(
            &issued_invoice(),
            &[],
            &settings("JP", "JPY"),
            None,
            None,
            0,
        );
        assert_eq!(total_labelled(&doc, "Total").unwrap().amount, "236,000");
    }

    #[test]
    fn blank_address_lines_and_whitespace_only_notes_are_dropped() {
        let doc = build(
            &issued_invoice(),
            &[],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(
            doc.customer.as_ref().unwrap().address_lines,
            vec!["12 Old Street".to_string(), "Bengaluru 560001".to_string()]
        );
        assert_eq!(doc.notes, None);
        assert_eq!(doc.terms.as_deref(), Some("Payment within 15 days."));
    }

    #[test]
    fn a_line_discount_is_noted_under_the_description() {
        let mut item = line_item("Widget", 1800);
        item.line_discount_type = Some(DiscountType::Percentage);
        item.line_discount_value = Some(1250);
        let doc = build(
            &issued_invoice(),
            &[item],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(
            doc.line_items[0].discount_note.as_deref(),
            Some("Less 12.5%")
        );

        let mut item = line_item("Widget", 1800);
        item.line_discount_type = Some(DiscountType::Amount);
        item.line_discount_value = Some(5_000);
        let doc = build(
            &issued_invoice(),
            &[item],
            &settings("IN", "INR"),
            None,
            None,
            0,
        );
        assert_eq!(
            doc.line_items[0].discount_note.as_deref(),
            Some("Less 50.00")
        );
    }

    #[test]
    fn quantities_drop_trailing_zeros() {
        assert_eq!(format_quantity(2_000), "2");
        assert_eq!(format_quantity(2_500), "2.5");
        assert_eq!(format_quantity(2_050), "2.05");
        assert_eq!(format_quantity(1), "0.001");
        assert_eq!(format_quantity(0), "0");
    }

    #[test]
    fn basis_points_render_as_percentages() {
        assert_eq!(format_basis_points(1800), "18%");
        assert_eq!(format_basis_points(1825), "18.25%");
        assert_eq!(format_basis_points(1850), "18.5%");
        assert_eq!(format_basis_points(0), "0%");
    }

    #[test]
    fn dates_follow_the_users_configured_pattern() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        assert_eq!(format_date(date, "DD/MM/YYYY"), "09/03/2026");
        assert_eq!(format_date(date, "MM/DD/YYYY"), "03/09/2026");
        assert_eq!(format_date(date, "YYYY-MM-DD"), "2026-03-09");
        assert_eq!(format_date(date, "DD MMM YYYY"), "09 Mar 2026");
        assert_eq!(format_date(date, "DD.MM.YY"), "09.03.26");
        // An unrecognised pattern comes through as written rather than
        // silently becoming some default.
        assert_eq!(format_date(date, "on DD"), "on 09");
    }
}
