use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use sqlx::SqlitePool;

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::invoice_number_sequencer::InvoiceNumberSequencer;
use crate::application::ports::transaction::Transaction;

use super::transaction::sqlite_tx;

pub struct SqliteInvoiceNumberSequencer {
    pool: SqlitePool,
}

impl SqliteInvoiceNumberSequencer {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// `{year}`-based formats reset annually (one counter row per year);
/// anything else shares a single, never-reset counter — database-schema.md §7.
fn scope_key_for(format: &str, at: NaiveDate) -> String {
    if format.contains("{year}") {
        at.year().to_string()
    } else {
        "ALL".to_string()
    }
}

/// Supports exactly the two placeholders `database-schema.md` §7's default
/// format needs: `{year}` and `{seq}` / `{seq:0Nd}` (zero-padded to N digits).
fn format_invoice_number(format: &str, year: i32, seq: i64) -> String {
    let with_year = format.replace("{year}", &year.to_string());
    if let Some(start) = with_year.find("{seq:") {
        if let Some(end_rel) = with_year[start..].find('}') {
            let end = start + end_rel + 1;
            let spec = &with_year[start..end];
            let width: usize = spec
                .trim_start_matches("{seq:")
                .trim_end_matches("d}")
                .parse()
                .unwrap_or(1);
            let seq_str = format!("{seq:0width$}");
            return format!("{}{}{}", &with_year[..start], seq_str, &with_year[end..]);
        }
    }
    with_year.replace("{seq}", &seq.to_string())
}

#[async_trait]
impl InvoiceNumberSequencer for SqliteInvoiceNumberSequencer {
    async fn preview_next(
        &self,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError> {
        let scope_key = scope_key_for(format, at);
        let last_value: Option<i64> = sqlx::query_scalar(
            "SELECT last_value FROM invoice_number_counters WHERE scope_key = ?",
        )
        .bind(&scope_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(format_invoice_number(
            format,
            at.year(),
            last_value.unwrap_or(0) + 1,
        ))
    }

    async fn issue_next(
        &self,
        tx: &mut dyn Transaction,
        format: &str,
        at: NaiveDate,
    ) -> Result<String, InfrastructureError> {
        let scope_key = scope_key_for(format, at);
        let conn = sqlite_tx(tx);
        sqlx::query(
            "INSERT INTO invoice_number_counters (scope_key, last_value) VALUES (?, 1) \
             ON CONFLICT(scope_key) DO UPDATE SET last_value = last_value + 1",
        )
        .bind(&scope_key)
        .execute(&mut **conn)
        .await?;

        let new_value: i64 = sqlx::query_scalar(
            "SELECT last_value FROM invoice_number_counters WHERE scope_key = ?",
        )
        .bind(&scope_key)
        .fetch_one(&mut **conn)
        .await?;
        Ok(format_invoice_number(format, at.year(), new_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_default_style() {
        assert_eq!(
            format_invoice_number("INV-{year}-{seq:04d}", 2026, 7),
            "INV-2026-0007"
        );
    }

    #[test]
    fn formats_plain_seq() {
        assert_eq!(format_invoice_number("BILL-{seq}", 2026, 42), "BILL-42");
    }

    #[test]
    fn scope_key_resets_yearly_only_for_year_based_formats() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 28).unwrap();
        assert_eq!(scope_key_for("INV-{year}-{seq:04d}", date), "2026");
        assert_eq!(scope_key_for("BILL-{seq}", date), "ALL");
    }
}
