use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::application::ports::business_repository::BusinessRepository;
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::transaction::Transaction;
use crate::domain::business::Business;
use crate::domain::tax_regime::TaxRegimeCode;

use super::transaction::sqlite_tx;

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
        logo_path: row.get("logo_path"),
        address: row.get("address"),
        phone: row.get("phone"),
        email: row.get("email"),
        gstin: row.get("gstin"),
        bank_details: row.get("bank_details"),
        upi_id: row.get("upi_id"),
        tax_regime_code: TaxRegimeCode::from_db_str(
            row.get::<String, _>("tax_regime_code").as_str(),
        ),
    }
}

#[async_trait]
impl BusinessRepository for SqliteBusinessRepository {
    async fn create(
        &self,
        tx: &mut dyn Transaction,
        business: Business,
    ) -> Result<Business, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "INSERT INTO business (id, name, logo_path, address, phone, email, gstin, bank_details, upi_id, tax_regime_code) \
             VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&business.name)
        .bind(&business.logo_path)
        .bind(&business.address)
        .bind(&business.phone)
        .bind(&business.email)
        .bind(&business.gstin)
        .bind(&business.bank_details)
        .bind(&business.upi_id)
        .bind(business.tax_regime_code.as_db_str())
        .execute(&mut **conn)
        .await?;
        Ok(business)
    }

    async fn get(&self) -> Result<Option<Business>, InfrastructureError> {
        let row = sqlx::query(
            "SELECT name, logo_path, address, phone, email, gstin, bank_details, upi_id, tax_regime_code FROM business WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(business_from_row))
    }

    async fn update(
        &self,
        tx: &mut dyn Transaction,
        business: Business,
    ) -> Result<Business, InfrastructureError> {
        let conn = sqlite_tx(tx);
        sqlx::query(
            "UPDATE business SET name = ?, logo_path = ?, address = ?, phone = ?, email = ?, \
             gstin = ?, bank_details = ?, upi_id = ?, tax_regime_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
        )
        .bind(&business.name)
        .bind(&business.logo_path)
        .bind(&business.address)
        .bind(&business.phone)
        .bind(&business.email)
        .bind(&business.gstin)
        .bind(&business.bank_details)
        .bind(&business.upi_id)
        .bind(business.tax_regime_code.as_db_str())
        .execute(&mut **conn)
        .await?;
        Ok(business)
    }
}
