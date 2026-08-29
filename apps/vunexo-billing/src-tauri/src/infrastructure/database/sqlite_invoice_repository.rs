use async_trait::async_trait;
use sqlx::{Row, Sqlite, SqlitePool, Transaction as SqlxTransaction};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::invoice_repository::InvoiceRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::invoice::{
    DiscountType, DraftInvoiceToSave, Invoice, InvoiceFilter, InvoiceStatus, InvoiceSummary,
    InvoiceWithLineItems, IssueInvoiceData,
};
use crate::domain::invoice_line_item::{InvoiceLineItem, LineItemToSave};

use super::transaction::sqlite_tx;

pub struct SqliteInvoiceRepository {
    pool: SqlitePool,
}

impl SqliteInvoiceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const INVOICE_COLUMNS: &str = "id, invoice_number, invoice_number_is_custom, status, customer_id, \
    customer_snapshot_name, customer_snapshot_phone, customer_snapshot_email, customer_snapshot_address, customer_snapshot_gstin, \
    business_snapshot_name, business_snapshot_address, business_snapshot_gstin, business_snapshot_phone, business_snapshot_email, \
    business_snapshot_bank_details, business_snapshot_upi_id, business_snapshot_logo_path, \
    is_interstate, invoice_date, due_date, notes, terms, discount_type, discount_value, \
    subtotal_minor, discount_amount_minor, tax_amount_minor, total_minor, \
    issued_at, cancelled_at, cancel_reason";

const LINE_ITEM_COLUMNS: &str =
    "id, product_id, description, unit, quantity_thousandths, unit_price_minor, \
    line_discount_type, line_discount_value, tax_rate_id, tax_rate_basis_points, \
    line_subtotal_minor, line_discount_amount_minor, invoice_discount_amount_minor, \
    taxable_amount_minor, line_tax_minor, line_total_minor, sort_order";

fn invoice_from_row(row: &sqlx::sqlite::SqliteRow) -> Invoice {
    Invoice {
        id: row.get("id"),
        invoice_number: row.get("invoice_number"),
        invoice_number_is_custom: row.get("invoice_number_is_custom"),
        status: InvoiceStatus::from_db_str(row.get::<String, _>("status").as_str()),
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
        is_interstate: row.get("is_interstate"),
        invoice_date: row.get("invoice_date"),
        due_date: row.get("due_date"),
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
        cancelled_at: row.get("cancelled_at"),
        cancel_reason: row.get("cancel_reason"),
    }
}

fn line_item_from_row(row: &sqlx::sqlite::SqliteRow) -> InvoiceLineItem {
    InvoiceLineItem {
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
        invoice_discount_amount_minor: row.get("invoice_discount_amount_minor"),
        taxable_amount_minor: row.get("taxable_amount_minor"),
        line_tax_minor: row.get("line_tax_minor"),
        line_total_minor: row.get("line_total_minor"),
        sort_order: row.get("sort_order"),
    }
}

async fn replace_line_items(
    conn: &mut SqlxTransaction<'static, Sqlite>,
    invoice_id: i64,
    line_items: &[LineItemToSave],
) -> Result<(), InfrastructureError> {
    sqlx::query("DELETE FROM invoice_line_items WHERE invoice_id = ?")
        .bind(invoice_id)
        .execute(&mut **conn)
        .await?;

    for item in line_items {
        sqlx::query(
            "INSERT INTO invoice_line_items (\
                invoice_id, product_id, description, unit, quantity_thousandths, unit_price_minor, \
                line_discount_type, line_discount_value, tax_rate_id, tax_rate_basis_points, \
                line_subtotal_minor, line_discount_amount_minor, invoice_discount_amount_minor, \
                taxable_amount_minor, line_tax_minor, line_total_minor, sort_order \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(invoice_id)
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
        .bind(item.invoice_discount_amount_minor)
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
    invoice_id: i64,
) -> Result<Vec<InvoiceLineItem>, InfrastructureError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let rows = sqlx::query(&format!(
        "SELECT {LINE_ITEM_COLUMNS} FROM invoice_line_items WHERE invoice_id = ? ORDER BY sort_order"
    ))
    .bind(invoice_id)
    .fetch_all(pool_or_conn)
    .await?;
    Ok(rows.iter().map(line_item_from_row).collect())
}

#[async_trait]
impl InvoiceRepository for SqliteInvoiceRepository {
    async fn create_draft(
        &self,
        tx: &mut dyn Transaction,
        draft: DraftInvoiceToSave,
    ) -> Result<InvoiceWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = sqlx::query(
            "INSERT INTO invoices (\
                customer_id, invoice_date, due_date, notes, terms, is_interstate, \
                discount_type, discount_value, subtotal_minor, discount_amount_minor, \
                tax_amount_minor, total_minor \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(draft.customer_id)
        .bind(draft.invoice_date)
        .bind(draft.due_date)
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

        replace_line_items(&mut *conn, id, &draft.line_items).await?;

        let row = sqlx::query(&format!(
            "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        let line_items = fetch_line_items(&mut **conn, id).await?;
        Ok(InvoiceWithLineItems {
            invoice: invoice_from_row(&row),
            line_items,
        })
    }

    async fn update_draft(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        draft: DraftInvoiceToSave,
    ) -> Result<InvoiceWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE invoices SET customer_id = ?, invoice_date = ?, due_date = ?, notes = ?, terms = ?, \
             is_interstate = ?, discount_type = ?, discount_value = ?, subtotal_minor = ?, \
             discount_amount_minor = ?, tax_amount_minor = ?, total_minor = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(draft.customer_id)
        .bind(draft.invoice_date)
        .bind(draft.due_date)
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

        let row = sqlx::query(&format!(
            "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        let line_items = fetch_line_items(&mut **conn, id).await?;
        Ok(InvoiceWithLineItems {
            invoice: invoice_from_row(&row),
            line_items,
        })
    }

    async fn issue(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        data: IssueInvoiceData,
    ) -> Result<InvoiceWithLineItems, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE invoices SET invoice_number = ?, invoice_number_is_custom = ?, status = 'ISSUED', \
             customer_snapshot_name = ?, customer_snapshot_phone = ?, customer_snapshot_email = ?, \
             customer_snapshot_address = ?, customer_snapshot_gstin = ?, \
             business_snapshot_name = ?, business_snapshot_address = ?, business_snapshot_gstin = ?, \
             business_snapshot_phone = ?, business_snapshot_email = ?, business_snapshot_bank_details = ?, \
             business_snapshot_upi_id = ?, business_snapshot_logo_path = ?, \
             subtotal_minor = ?, discount_amount_minor = ?, tax_amount_minor = ?, total_minor = ?, \
             issued_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(&data.invoice_number)
        .bind(data.invoice_number_is_custom)
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
        .bind(data.subtotal_minor)
        .bind(data.discount_amount_minor)
        .bind(data.tax_amount_minor)
        .bind(data.total_minor)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        replace_line_items(&mut *conn, id, &data.line_items).await?;

        let row = sqlx::query(&format!(
            "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        let line_items = fetch_line_items(&mut **conn, id).await?;
        Ok(InvoiceWithLineItems {
            invoice: invoice_from_row(&row),
            line_items,
        })
    }

    async fn cancel(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        reason: Option<String>,
    ) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE invoices SET status = 'CANCELLED', cancelled_at = CURRENT_TIMESTAMP, cancel_reason = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(reason)
        .bind(id)
        .execute(&mut **conn)
        .await?;
        Ok(())
    }

    async fn set_status(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        status: InvoiceStatus,
    ) -> Result<(), InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query("UPDATE invoices SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(status.as_db_str())
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
        sqlx::query("DELETE FROM invoices WHERE id = ?")
            .bind(id)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }

    async fn get(&self, id: i64) -> Result<Option<InvoiceWithLineItems>, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {INVOICE_COLUMNS} FROM invoices WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            None => Ok(None),
            Some(row) => {
                let line_items = fetch_line_items(&self.pool, id).await?;
                Ok(Some(InvoiceWithLineItems {
                    invoice: invoice_from_row(&row),
                    line_items,
                }))
            }
        }
    }

    async fn list(
        &self,
        filter: InvoiceFilter,
    ) -> Result<Vec<InvoiceSummary>, InfrastructureError> {
        let where_clause = match filter.status {
            Some(status) => format!("WHERE i.status = '{}'", status.as_db_str()),
            None => String::new(),
        };
        let sql = format!(
            "SELECT \
                i.id, i.invoice_number, i.status, i.invoice_date, i.due_date, i.total_minor, \
                COALESCE(i.customer_snapshot_name, c.name) AS customer_name, \
                COALESCE((SELECT SUM(p.amount_minor) FROM payments p WHERE p.invoice_id = i.id), 0) AS amount_paid_minor, \
                (i.due_date IS NOT NULL AND i.due_date < date('now') AND i.status NOT IN ('DRAFT', 'CANCELLED') \
                 AND (i.total_minor - COALESCE((SELECT SUM(p2.amount_minor) FROM payments p2 WHERE p2.invoice_id = i.id), 0)) > 0 \
                ) AS is_overdue \
             FROM invoices i \
             LEFT JOIN customers c ON c.id = i.customer_id \
             {where_clause} \
             ORDER BY i.invoice_date DESC, i.id DESC"
        );
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| InvoiceSummary {
                id: row.get("id"),
                invoice_number: row.get("invoice_number"),
                status: InvoiceStatus::from_db_str(row.get::<String, _>("status").as_str()),
                customer_name: row.get("customer_name"),
                invoice_date: row.get("invoice_date"),
                due_date: row.get("due_date"),
                total_minor: row.get("total_minor"),
                amount_paid_minor: row.get("amount_paid_minor"),
                is_overdue: row.get::<bool, _>("is_overdue"),
            })
            .collect())
    }

    async fn has_any_issued(&self) -> Result<bool, InfrastructureError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE issued_at IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }
}
