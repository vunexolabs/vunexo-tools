use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::tax_rate_repository::TaxRateRepository;
use crate::application::ports::transaction::Transaction;
use crate::domain::tax_rate::{TaxRate, TaxRateFields};

use super::transaction::sqlite_tx;

pub struct SqliteTaxRateRepository {
    pool: SqlitePool,
}

impl SqliteTaxRateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "id, name, rate_basis_points";

fn tax_rate_from_row(row: &sqlx::sqlite::SqliteRow) -> TaxRate {
    TaxRate {
        id: row.get("id"),
        name: row.get("name"),
        rate_basis_points: row.get("rate_basis_points"),
    }
}

#[async_trait]
impl TaxRateRepository for SqliteTaxRateRepository {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        fields: TaxRateFields,
    ) -> Result<TaxRate, InfrastructureError> {
        let conn = sqlite_tx(tx);
        let id = sqlx::query("INSERT INTO tax_rates (name, rate_basis_points) VALUES (?, ?)")
            .bind(&fields.name)
            .bind(fields.rate_basis_points)
            .execute(&mut **conn)
            .await?
            .last_insert_rowid();
        Ok(TaxRate {
            id,
            name: fields.name,
            rate_basis_points: fields.rate_basis_points,
        })
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        id: i64,
        fields: TaxRateFields,
    ) -> Result<TaxRate, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE tax_rates SET name = ?, rate_basis_points = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
        )
        .bind(&fields.name)
        .bind(fields.rate_basis_points)
        .bind(id)
        .execute(&mut **conn)
        .await?;

        let row = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM tax_rates WHERE id = ?"
        ))
        .bind(id)
        .fetch_one(&mut **conn)
        .await?;
        Ok(tax_rate_from_row(&row))
    }

    async fn list(&self) -> Result<Vec<TaxRate>, InfrastructureError> {
        let rows = sqlx::query(&format!(
            "SELECT {SELECT_COLUMNS} FROM tax_rates ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(tax_rate_from_row).collect())
    }
}
