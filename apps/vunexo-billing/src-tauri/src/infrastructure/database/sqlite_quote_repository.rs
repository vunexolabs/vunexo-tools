//! Mirrors `sqlite_invoice_repository.rs` closely — see that file for the
//! shape this follows. database-schema-v2.md §3/§4/§9.

use async_trait::async_trait;
use sqlx::{Row, Sqlite, SqlitePool, Transaction as SqlxTransaction};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::quote_repository::QuoteRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::invoice::DiscountType;
use crate::domain::quote::{
    DraftQuoteToSave, IssueQuoteData, Quote, QuoteFilter, QuoteStatus, QuoteSummary,
    QuoteWithLineItems,
};
use crate::domain::quote_line_item::{QuoteLineItem, QuoteLineItemToSave};
use crate::domain::tax_regime::normalize_legacy_snapshot;

use super::transaction::sqlite_tx;

pub struct SqliteQuoteRepository {
    pool: SqlitePool,
}

impl SqliteQuoteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const QUOTE_COLUMNS: &str = "id, quote_number, status, customer_id, \
    customer_snapshot_name, customer_snapshot_phone, customer_snapshot_email, customer_snapshot_address, customer_snapshot_gstin, \
    business_snapshot_name, business_snapshot_address, business_snapshot_gstin, business_snapshot_phone, business_snapshot_email, \
    business_snapshot_bank_details, business_snapshot_upi_id, business_snapshot_logo_path, \
    tax_regime_snapshot, is_interstate, quote_date, valid_until, notes, terms, discount_type, discount_value, \
    subtotal_minor, discount_amount_minor, tax_amount_minor, total_minor, \
    issued_at, accepted_at, declined_at, converted_at, cancelled_at, cancel_reason";

const LINE_ITEM_COLUMNS: &str =
    "id, product_id, description, unit, quantity_thousandths, unit_price_minor, \
    line_discount_type, line_discount_value, tax_rate_id, tax_rate_basis_points, \
    line_subtotal_minor, line_discount_amount_minor, quote_discount_amount_minor, \
    taxable_amount_minor, line_tax_minor, line_total_minor, sort_order";

fn quote_from_row(row: &sqlx::sqlite::SqliteRow) -> Quote {
    Quote {
        id: row.get("id"),
        quote_number: row.get("quote_number"),
        status: QuoteStatus::from_db_str(row.get::<String, _>("status").as_str()),
        customer_id: row.get("customer_id"),
        customer_snapshot_name: row.get("customer_snapshot_name"),
        customer_snapshot_phone: row.get("customer_snapshot_phone"),
        customer_snapshot_email: row.get("customer_snapshot_email"),
        customer_snapshot_address: row.get("customer_snapshot_address"),
        customer_snapshot_gstin: row.get("customer_snapshot_gstin"),
        business_snapshot_name: row.get("business_snapshot_name"),
        business_snapshot_address: row.get("business_snapshot_address"),
        business_snapshot_gstin: row.get("business_snapshot_gstin"),
        business_snapshot_phone: row.get("business_snapshot_phone"),
        business_snapshot_email: row.get("business_snapshot_email"),
        business_snapshot_bank_details: row.get("business_snapshot_bank_details"),
        business_snapshot_upi_id: row.get("business_snapshot_upi_id"),
        business_snapshot_logo_path: row.get("business_snapshot_logo_path"),
        tax_regime_snapshot: {
            let issued_at: Option<chrono::DateTime<chrono::Utc>> = row.get("issued_at");
            let raw: Option<String> = row.get("tax_regime_snapshot");
            issued_at.map(|_| normalize_legacy_snapshot(raw.as_deref()))
        },
        is_interstate: row.get("is_interstate"),
        quote_date: row.get("quote_date"),
        valid_until: row.get("valid_until"),
        notes: row.get("notes"),
        terms: row.get("terms"),
        discount_type: row
            .get::<Option<String>, _>("discount_type")
            .map(|s| DiscountType::from_db_str(&s)),
        discount_value: row.get("discount_value"),
        subtotal_minor: row.get("subtotal_minor"),
        discount_amount_minor: row.get("discount_amount_minor"),
        tax_amount_minor: row.get("tax_amount_minor"),
        total_minor: row.get("total_minor"),
        issued_at: row.get("issued_at"),
        accepted_at: row.get("accepted_at"),
        declined_at: row.get("declined_at"),
        converted_at: row.get("converted_at"),
        cancelled_at: row.get("cancelled_at"),
        cancel_reason: row.get("cancel_reason"),
    }
}

fn line_item_from_row(row: &sqlx::sqlite::SqliteRow) -> QuoteLineItem {
    QuoteLineItem {
        id: row.get("id"),
        product_id: row.get("product_id"),
        description: row.get("description"),
        unit: row.get("unit"),
        quantity_thousandths: row.get("quantity_thousandths"),
        unit_price_minor: row.get("unit_price_minor"),
        line_discount_type: row
            .get::<Option<String>, _>("line_discount_type")
            .map(|s| DiscountType::from_db_str(&s)),
        line_discount_value: row.get("line_discount_value"),
        tax_rate_id: row.get("tax_rate_id"),
        tax_rate_basis_points: row.get("tax_rate_basis_points"),
        line_subtotal_minor: row.get("line_subtotal_minor"),
        line_discount_amount_minor: row.get("line_discount_amount_minor"),
        quote_discount_amount_minor: row.get("quote_discount_amount_minor"),
        taxable_amount_minor: row.get("taxable_amount_minor"),
        line_tax_minor: row.get("line_tax_minor"),
        line_total_minor: row.get("line_total_minor"),
        sort_order: row.get("sort_order"),
    }
}

async fn replace_line_items(
    conn: &mut SqlxTransaction<'static, Sqlite>,
    quote_id: i64,
    line_items: &[QuoteLineItemToSave],
) -> Result<(), InfrastructureError> {
    sqlx::query("DELETE FROM quote_line_items WHERE quote_id = ?")
        .bind(quote_id)
        .execute(&mut **conn)
        .await?;

    for item in line_items {
        sqlx::query(
            "INSERT INTO quote_line_items (\
                quote_id, product_id, description, unit, quantity_thousandths, unit_price_minor, \
                line_discount_type, line_discount_value, tax_rate_id, tax_rate_basis_points, \
                line_subtotal_minor, line_discount_amount_minor, quote_discount_amount_minor, \
                taxable_amount_minor, line_tax_minor, line_total_minor, sort_order \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(quote_id)
        .bind(item.product_id)
        .bind(&item.description)
        .bind(&item.unit)
        .bind(item.quantity_thousandths)
        .bind(item.unit_price_minor)
        .bind(item.line_discount_type.map(DiscountType::as_db_str))
        .bind(item.line_discount_value)
        .bind(item.tax_rate_id)
        .bind(item.tax_rate_basis_points)
        .bind(item.line_subtotal_minor)
        .bind(item.line_discount_amount_minor)
        .bind(item.quote_discount_amount_minor)
        .bind(item.taxable_amount_minor)
        .bind(item.line_tax_minor)
        .bind(item.line_total_minor)
        .bind(item.sort_order)
        .execute(&mut **conn)
        .await?;
    }
    Ok(())
}

async fn fetch_line_items<'e, E>(
    pool_or_conn: E,
    quote_id: i64,
) -> Result<Vec<QuoteLineItem>, InfrastructureError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let rows = sqlx::query(&format!(
        "SELECT {LINE_ITEM_COLUMNS} FROM quote_line_items WHERE quote_id = ? ORDER BY sort_order"
    ))
    .bind(quote_id)
    .fetch_all(pool_or_conn)
    .await?;
    Ok(rows.iter().map(line_item_from_row).collect())
}

async fn fetch_quote_with_line_items(
    conn: &mut SqlxTransaction<'static, Sqlite>,
    id: i64,
) -> Result<QuoteWithLineItems, InfrastructureError> {
    let row = sqlx::query(&format!("SELECT {QUOTE_COLUMNS} FROM quotes WHERE id = ?"))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
    let line_items = fetch_line_items(&mut **conn, id).await?;
    Ok(QuoteWithLineItems {
        quote: quote_from_row(&row),
        line_items,
    })
}

async fn insert_draft_row(
    conn: &mut SqlxTransaction<'static, Sqlite>,
    draft: &DraftQuoteToSave,
) -> Result<i64, InfrastructureError> {
    let id = sqlx::query(
        "INSERT INTO quotes (\
            customer_id, quote_date, valid_until, notes, terms, is_interstate, \
            discount_type, discount_value, subtotal_minor, discount_amount_minor, \
            tax_amount_minor, total_minor \
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(draft.customer_id)
    .bind(draft.quote_date)
    .bind(draft.valid_until)
    .bind(&draft.notes)
    .bind(&draft.terms)
    .bind(draft.is_interstate)
    .bind(draft.discount_type.map(DiscountType::as_db_str))
    .bind(draft.discount_value)
    .bind(draft.subtotal_minor)
    .bind(draft.discount_amount_minor)
    .bind(draft.tax_amount_minor)
    .bind(draft.total_minor)
    .execute(&mut **conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

#[async_trait]
impl QuoteRepository for SqliteQuoteRepository {
    async fn create_draft(
        &self,
        tx: &mut dyn Transaction,
        draft: DraftQuoteToSave,
    ) -> Result<QuoteWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = insert_draft_row(&mut *conn, &draft).await?;
        replace_line_items(&mut *conn, id, &draft.line_items).await?;
        fetch_quote_with_line_items(&mut *conn, id).await
    }

    async fn update_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        draft: DraftQuoteToSave,
    ) -> Result<QuoteWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET customer_id = ?, quote_date = ?, valid_until = ?, notes = ?, terms = ?, \
             is_interstate = ?, discount_type = ?, discount_value = ?, subtotal_minor = ?, \
             discount_amount_minor = ?, tax_amount_minor = ?, total_minor = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(draft.customer_id)
        .bind(draft.quote_date)
        .bind(draft.valid_until)
        .bind(&draft.notes)
        .bind(&draft.terms)
        .bind(draft.is_interstate)
        .bind(draft.discount_type.map(DiscountType::as_db_str))
        .bind(draft.discount_value)
        .bind(draft.subtotal_minor)
        .bind(draft.discount_amount_minor)
        .bind(draft.tax_amount_minor)
        .bind(draft.total_minor)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        replace_line_items(&mut *conn, id, &draft.line_items).await?;
        fetch_quote_with_line_items(&mut *conn, id).await
    }

    async fn issue(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        data: IssueQuoteData,
    ) -> Result<QuoteWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET quote_number = ?, status = 'ISSUED', \
             customer_snapshot_name = ?, customer_snapshot_phone = ?, customer_snapshot_email = ?, \
             customer_snapshot_address = ?, customer_snapshot_gstin = ?, \
             business_snapshot_name = ?, business_snapshot_address = ?, business_snapshot_gstin = ?, \
             business_snapshot_phone = ?, business_snapshot_email = ?, business_snapshot_bank_details = ?, \
             business_snapshot_upi_id = ?, business_snapshot_logo_path = ?, tax_regime_snapshot = ?, \
             subtotal_minor = ?, discount_amount_minor = ?, tax_amount_minor = ?, total_minor = ?, \
             issued_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(&data.quote_number)
        .bind(&data.customer_snapshot.name)
        .bind(&data.customer_snapshot.phone)
        .bind(&data.customer_snapshot.email)
        .bind(&data.customer_snapshot.address)
        .bind(&data.customer_snapshot.gstin)
        .bind(&data.business_snapshot.name)
        .bind(&data.business_snapshot.address)
        .bind(&data.business_snapshot.gstin)
        .bind(&data.business_snapshot.phone)
        .bind(&data.business_snapshot.email)
        .bind(&data.business_snapshot.bank_details)
        .bind(&data.business_snapshot.upi_id)
        .bind(&data.business_snapshot.logo_path)
        .bind(data.tax_regime_snapshot.as_db_str())
        .bind(data.subtotal_minor)
        .bind(data.discount_amount_minor)
        .bind(data.tax_amount_minor)
        .bind(data.total_minor)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        replace_line_items(&mut *conn, id, &data.line_items).await?;
        fetch_quote_with_line_items(&mut *conn, id).await
    }

    async fn accept(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET status = 'ACCEPTED', accepted_at = CURRENT_TIMESTAMP, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn decline(&self, tx: &mut dyn Transaction, id: i64) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET status = 'DECLINED', declined_at = CURRENT_TIMESTAMP, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn mark_converted(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
    ) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET status = 'CONVERTED', converted_at = CURRENT_TIMESTAMP, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn cancel(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE quotes SET status = 'CANCELLED', cancelled_at = CURRENT_TIMESTAMP, cancel_reason = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(reason)
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn delete_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
    ) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query("DELETE FROM quotes WHERE id = ?")
            .bind(id)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }

    async fn get(&self, id: i64) -> Result<Option<QuoteWithLineItems>, InfrastructureError> {
        let row = sqlx::query(&format!("SELECT {QUOTE_COLUMNS} FROM quotes WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(None),
            Some(row) => {
                let line_items = fetch_line_items(&self.pool, id).await?;
                Ok(Some(QuoteWithLineItems {
                    quote: quote_from_row(&row),
                    line_items,
                }))
            }
        }
    }

    async fn list(&self, filter: QuoteFilter) -> Result<Vec<QuoteSummary>, InfrastructureError> {
        let where_clause = match filter.status {
            Some(status) => format!("WHERE q.status = '{}'", status.as_db_str()),
            None => String::new(),
        };
        let sql = format!(
            "SELECT \
                q.id, q.quote_number, q.status, q.quote_date, q.valid_until, q.total_minor, \
                COALESCE(q.customer_snapshot_name, c.name) AS customer_name, \
                (q.valid_until IS NOT NULL AND q.valid_until < date('now') AND q.status = 'ISSUED') AS is_expired \
             FROM quotes q \
             LEFT JOIN customers c ON c.id = q.customer_id \
             {where_clause} \
             ORDER BY q.quote_date DESC, q.id DESC"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| QuoteSummary {
                id: row.get("id"),
                quote_number: row.get("quote_number"),
                status: QuoteStatus::from_db_str(row.get::<String, _>("status").as_str()),
                customer_name: row.get("customer_name"),
                quote_date: row.get("quote_date"),
                valid_until: row.get("valid_until"),
                total_minor: row.get("total_minor"),
                is_expired: row.get::<bool, _>("is_expired"),
            })
            .collect())
    }

    async fn has_any_issued(&self) -> Result<bool, InfrastructureError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM quotes WHERE issued_at IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }
}
