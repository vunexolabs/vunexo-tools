use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::business_repository::BusinessRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::business::Business;

pub struct SqliteBusinessRepository {
    pool: SqlitePool,
}

impl SqliteBusinessRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn business_from_row(row: &sqlx::sqlite::SqliteRow) -> Business {
    Business {
        name: row.get("name"),
        address: row.get("address"),
        tax_info: row.get("tax_info"),
        currency_symbol: row.get("currency_symbol"),
    }
}

#[async_trait]
impl BusinessRepository for SqliteBusinessRepository {
    async fn create(&self, business: Business) -> Result<Business, InfrastructureError> {
        sqlx::query(
            "INSERT INTO business (id, name, address, tax_info, currency_symbol) VALUES (1, ?, ?, ?, ?)",
        )
        .bind(&business.name)
        .bind(&business.address)
        .bind(&business.tax_info)
        .bind(&business.currency_symbol)
        .execute(&self.pool)
        .await?;
        Ok(business)
    }

    async fn get(&self) -> Result<Option<Business>, InfrastructureError> {
        let row = sqlx::query(
            "SELECT name, address, tax_info, currency_symbol FROM business WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(business_from_row))
    }

    async fn update(&self, business: Business) -> Result<Business, InfrastructureError> {
        sqlx::query(
            "UPDATE business SET name = ?, address = ?, tax_info = ?, currency_symbol = ?, \
             updated_at = datetime('now') WHERE id = 1",
        )
        .bind(&business.name)
        .bind(&business.address)
        .bind(&business.tax_info)
        .bind(&business.currency_symbol)
        .execute(&self.pool)
        .await?;
        Ok(business)
    }
}
