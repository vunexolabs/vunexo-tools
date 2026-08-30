//! CSV and JSON export shapes. ui-ux.md §6.
//!
//! Pure — building the *content* of an export is a domain concern (which
//! columns, in which order, formatted how); writing it to a path the user
//! picked is `infrastructure::filesystem`.
//!
//! Money is exported as a **plain decimal with no grouping and no currency
//! symbol** (`1234.50`), deliberately unlike the PDF: an export is read by a
//! spreadsheet or an accountant's importer, and `₹12,34,567.89` is data loss
//! dressed up as presentation. The currency code travels in its own column.

use crate::domain::currency::currency_meta;
use crate::domain::customer::Customer;
use crate::domain::invoice::{Invoice, InvoiceStatus};
use crate::domain::invoice_line_item::InvoiceLineItem;
use crate::domain::invoice_pdf::format_basis_points;
use crate::domain::payment::Payment;
use crate::domain::product::Product;
use crate::domain::tax_rate::TaxRate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportEntity {
    Customers,
    Products,
    Invoices,
    /// Every table, as JSON — a different operation from the CSVs, not a
    /// format switch on them (ui-ux.md §6).
    All,
}

impl ExportEntity {
    /// The name the save dialog offers, with the extension this entity
    /// actually exports as.
    pub fn suggested_file_name(self) -> &'static str {
        match self {
            ExportEntity::Customers => "vunexo-customers.csv",
            ExportEntity::Products => "vunexo-products.csv",
            ExportEntity::Invoices => "vunexo-invoices.csv",
            ExportEntity::All => "vunexo-all-data.json",
        }
    }
}

/// Renders one CSV record, RFC 4180 style: a field is quoted when it contains
/// a comma, a quote, or a line break, and embedded quotes are doubled.
///
/// Worth doing precisely rather than "join with commas" — a customer called
/// `Smith, Jones & Co` or an address with a newline in it would otherwise
/// silently shift every later column in that row.
pub fn csv_record(fields: &[String]) -> String {
    let mut out = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&csv_field(field));
    }
    // CRLF per RFC 4180 — Excel on Windows is the least forgiving consumer.
    out.push_str("\r\n");
    out
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Minor units to a bare decimal string: `123456` at 2 decimals -> `1234.56`.
/// No grouping separators — see the module note.
pub fn export_amount(minor: i64, currency_code: &str) -> String {
    let decimals = currency_meta(currency_code).decimals;
    let negative = minor < 0;
    let abs = minor.unsigned_abs();
    let scale = 10u64.pow(decimals);
    let sign = if negative { "-" } else { "" };
    if decimals == 0 {
        return format!("{sign}{}", abs / scale);
    }
    format!(
        "{sign}{}.{:0width$}",
        abs / scale,
        abs % scale,
        width = decimals as usize
    )
}

fn optional(value: Option<&str>) -> String {
    value.unwrap_or("").to_string()
}

pub fn customers_csv(customers: &[Customer]) -> String {
    let mut out = csv_record(&header(&[
        "name", "phone", "email", "address", "gstin", "status",
    ]));
    for customer in customers {
        out.push_str(&csv_record(&[
            customer.name.clone(),
            optional(customer.phone.as_deref()),
            optional(customer.email.as_deref()),
            optional(customer.address.as_deref()),
            optional(customer.gstin.as_deref()),
            format!("{:?}", customer.status).to_uppercase(),
        ]));
    }
    out
}

/// `tax_rates` resolves each product's `tax_rate_id` to a readable percentage
/// — an export full of foreign keys would be useless in a spreadsheet.
pub fn products_csv(products: &[Product], tax_rates: &[TaxRate], currency_code: &str) -> String {
    let mut out = csv_record(&header(&[
        "name",
        "sku",
        "description",
        "unit",
        "price",
        "currency",
        "hsn_sac_code",
        "tax_rate",
        "tax_rate_percent",
        "status",
    ]));
    for product in products {
        let rate = product
            .tax_rate_id
            .and_then(|id| tax_rates.iter().find(|r| r.id == id));
        out.push_str(&csv_record(&[
            product.name.clone(),
            optional(product.sku.as_deref()),
            optional(product.description.as_deref()),
            product.unit.clone(),
            export_amount(product.price_minor, currency_code),
            currency_code.to_string(),
            optional(product.hsn_sac_code.as_deref()),
            rate.map(|r| r.name.clone()).unwrap_or_default(),
            rate.map(|r| format_basis_points(r.rate_basis_points))
                .unwrap_or_default(),
            format!("{:?}", product.status).to_uppercase(),
        ]));
    }
    out
}

/// One row per invoice — the figures an accountant reconciles against, not
/// the line items (those are in the JSON export, which is the lossless one).
pub fn invoices_csv(invoices: &[InvoiceExportRow], currency_code: &str) -> String {
    let mut out = csv_record(&header(&[
        "invoice_number",
        "status",
        "customer",
        "invoice_date",
        "due_date",
        "currency",
        "subtotal",
        "discount",
        "tax",
        "total",
        "amount_paid",
        "balance_due",
    ]));
    for row in invoices {
        out.push_str(&csv_record(&[
            optional(row.invoice_number.as_deref()),
            status_label(row.status).to_string(),
            optional(row.customer_name.as_deref()),
            row.invoice_date.clone(),
            optional(row.due_date.as_deref()),
            currency_code.to_string(),
            export_amount(row.subtotal_minor, currency_code),
            export_amount(row.discount_amount_minor, currency_code),
            export_amount(row.tax_amount_minor, currency_code),
            export_amount(row.total_minor, currency_code),
            export_amount(row.amount_paid_minor, currency_code),
            export_amount(row.total_minor - row.amount_paid_minor, currency_code),
        ]));
    }
    out
}

/// What `invoices_csv` needs per invoice, assembled by the use case — the
/// stored invoice plus the payment total, which lives in another table.
pub struct InvoiceExportRow {
    pub invoice_number: Option<String>,
    pub status: InvoiceStatus,
    pub customer_name: Option<String>,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub subtotal_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_minor: i64,
    pub amount_paid_minor: i64,
}

fn status_label(status: InvoiceStatus) -> &'static str {
    status.as_db_str()
}

fn header(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|l| l.to_string()).collect()
}

/// One invoice, with its line items and payments, for the JSON export.
/// Serialized from the *domain* shapes rather than raw DB rows, per
/// ui-ux.md §6's "not a blind SQLite table dump".
#[derive(serde::Serialize)]
pub struct InvoiceExport<'a> {
    #[serde(flatten)]
    pub invoice: &'a Invoice,
    pub line_items: &'a [InvoiceLineItem],
    pub payments: &'a [Payment],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::customer::CustomerStatus;

    fn customer(name: &str, address: Option<&str>) -> Customer {
        Customer {
            id: 1,
            name: name.to_string(),
            phone: None,
            email: None,
            address: address.map(str::to_string),
            gstin: None,
            status: CustomerStatus::Active,
        }
    }

    #[test]
    fn plain_fields_are_not_quoted() {
        assert_eq!(csv_record(&["a".into(), "b".into()]), "a,b\r\n");
    }

    #[test]
    fn a_field_containing_a_comma_is_quoted() {
        assert_eq!(
            csv_record(&["Smith, Jones & Co".into()]),
            "\"Smith, Jones & Co\"\r\n"
        );
    }

    #[test]
    fn embedded_quotes_are_doubled() {
        assert_eq!(
            csv_record(&["He said \"hello\"".into()]),
            "\"He said \"\"hello\"\"\"\r\n"
        );
    }

    #[test]
    fn a_field_containing_a_newline_is_quoted_and_keeps_the_newline() {
        // A multi-line address must stay one field, not become a new row.
        assert_eq!(
            csv_record(&["12 Old Street\nBengaluru".into()]),
            "\"12 Old Street\nBengaluru\"\r\n"
        );
    }

    #[test]
    fn a_multi_line_address_does_not_shift_later_columns() {
        let csv = customers_csv(&[customer("Acme", Some("12 Old Street\nBengaluru 560001"))]);
        // Header + exactly one record: the newline is inside a quoted field.
        assert_eq!(csv.matches("\r\n").count(), 2);
        assert!(csv.contains("\"12 Old Street\nBengaluru 560001\""));
    }

    #[test]
    fn amounts_export_as_plain_decimals_without_symbol_or_grouping() {
        assert_eq!(export_amount(123_456_789, "INR"), "1234567.89");
        assert_eq!(export_amount(0, "USD"), "0.00");
        assert_eq!(export_amount(-150_000, "USD"), "-1500.00");
        // Currency-correct decimal counts still apply.
        assert_eq!(export_amount(1_234, "JPY"), "1234");
        assert_eq!(export_amount(1_234_567, "KWD"), "1234.567");
    }

    #[test]
    fn the_customers_csv_has_a_header_and_one_row_per_customer() {
        let csv = customers_csv(&[customer("Acme", None), customer("Beta", None)]);
        let lines: Vec<&str> = csv.split("\r\n").filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "name,phone,email,address,gstin,status");
        assert!(lines[1].starts_with("Acme,"));
        assert!(lines[1].ends_with(",ACTIVE"));
    }

    #[test]
    fn the_invoices_csv_derives_balance_due_from_total_and_payments() {
        let csv = invoices_csv(
            &[InvoiceExportRow {
                invoice_number: Some("INV-1".into()),
                status: InvoiceStatus::PartiallyPaid,
                customer_name: Some("Acme".into()),
                invoice_date: "2026-03-09".into(),
                due_date: None,
                subtotal_minor: 200_000,
                discount_amount_minor: 0,
                tax_amount_minor: 36_000,
                total_minor: 236_000,
                amount_paid_minor: 100_000,
            }],
            "INR",
        );
        let row = csv.split("\r\n").nth(1).unwrap();
        assert!(row.starts_with("INV-1,PARTIALLY_PAID,Acme,2026-03-09,,INR,"));
        assert!(row.ends_with("2360.00,1000.00,1360.00"));
    }

    #[test]
    fn an_export_with_no_rows_is_still_a_valid_file_with_its_header() {
        let csv = customers_csv(&[]);
        assert_eq!(csv, "name,phone,email,address,gstin,status\r\n");
    }
}
