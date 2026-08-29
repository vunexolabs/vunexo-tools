use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::settings_repository::SettingsRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::settings::{Settings, SettingsFields};

use super::transaction::sqlite_tx;

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn settings_from_row(row: &sqlx::sqlite::SqliteRow) -> Settings {
    Settings {
        country_code: row.get("country_code"),
        currency_code: row.get("currency_code"),
        date_format: row.get("date_format"),
        invoice_number_format: row.get("invoice_number_format"),
        default_due_days: row.get("default_due_days"),
        default_tax_rate_id: row.get("default_tax_rate_id"),
    }
}

const SELECT_COLUMNS: &str =
    "country_code, currency_code, date_format, invoice_number_format, default_due_days, default_tax_rate_id";

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self) -> Result<Settings, InfrastructureError> {
        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM settings WHERE id = 1"
        ))
        .fetch_one(&self.pool)
        .await?;
        Ok(settings_from_row(&row))
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        fields: SettingsFields,
    ) -> Result<Settings, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE settings SET country_code = ?, currency_code = ?, date_format = ?, \
             invoice_number_format = ?, default_due_days = ?, default_tax_rate_id = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = 1",
        )
        .bind(&fields.country_code)
        .bind(&fields.currency_code)
        .bind(&fields.date_format)
        .bind(&fields.invoice_number_format)
        .bind(fields.default_due_days)
        .bind(fields.default_tax_rate_id)
        .execute(&mut **conn)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM settings WHERE id = 1"
        ))
        .fetch_one(&mut **conn)
        .await?;
        Ok(settings_from_row(&row))
    }
}
