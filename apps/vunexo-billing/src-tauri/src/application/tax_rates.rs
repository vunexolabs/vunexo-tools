//! Tax rate use cases. application-architecture.md §4 ("Tax Rates" bullet).

use std::sync::Arc;

use crate::domain::tax_rate::{TaxRate, TaxRateFields};

use super::error::ApplicationError;
use super::ports::tax_rate_repository::TaxRateRepository;
use super::ports::transaction::TransactionManager;

pub struct TaxRateUseCases {
    repo: Arc<dyn TaxRateRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl TaxRateUseCases {
    pub fn new(repo: Arc<dyn TaxRateRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    fn validate(fields: &TaxRateFields) -> Result<(), ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "tax rate name is required".into(),
            ));
        }
        if fields.rate_basis_points < 0 {
            return Err(ApplicationError::Validation(
                "tax rate can't be negative".into(),
            ));
        }
        Ok(())
    }

    pub async fn create_tax_rate(
        &self,
        fields: TaxRateFields,
    ) -> Result<TaxRate, ApplicationError> {
        Self::validate(&fields)?;
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, fields).await {
            Ok(rate) => {
                tx.commit().await?;
                Ok(rate)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn update_tax_rate(
        &self,
        id: i64,
        fields: TaxRateFields,
    ) -> Result<TaxRate, ApplicationError> {
        Self::validate(&fields)?;
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, id, fields).await {
            Ok(rate) => {
                tx.commit().await?;
                Ok(rate)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn list_tax_rates(&self) -> Result<Vec<TaxRate>, ApplicationError> {
        Ok(self.repo.list().await?)
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::application::ports::tax_rate_repository::TaxRateRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::infrastructure::database::sqlite_tax_rate_repository::SqliteTaxRateRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    struct TestApp {
        tax_rates: TaxRateUseCases,
        db_path: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.db_path);
        }
    }

    async fn setup() -> TestApp {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_tax_rate_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let repo: Arc<dyn TaxRateRepository> = Arc::new(SqliteTaxRateRepository::new(pool));

        TestApp {
            tax_rates: TaxRateUseCases::new(repo, tx_manager),
            db_path,
        }
    }

    #[tokio::test]
    async fn create_update_and_list_round_trip() {
        let app = setup().await;

        let gst18 = app
            .tax_rates
            .create_tax_rate(TaxRateFields {
                name: "GST 18%".into(),
                rate_basis_points: 1800,
            })
            .await
            .expect("create_tax_rate");
        assert_eq!(gst18.rate_basis_points, 1800);

        app.tax_rates
            .create_tax_rate(TaxRateFields {
                name: "GST 5%".into(),
                rate_basis_points: 500,
            })
            .await
            .expect("create_tax_rate");

        let renamed = app
            .tax_rates
            .update_tax_rate(
                gst18.id,
                TaxRateFields {
                    name: "GST 18% (Standard)".into(),
                    rate_basis_points: 1800,
                },
            )
            .await
            .expect("update_tax_rate");
        assert_eq!(renamed.name, "GST 18% (Standard)");

        let all = app
            .tax_rates
            .list_tax_rates()
            .await
            .expect("list_tax_rates");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let app = setup().await;
        let result = app
            .tax_rates
            .create_tax_rate(TaxRateFields {
                name: "   ".into(),
                rate_basis_points: 1800,
            })
            .await;
        assert!(matches!(result, Err(ApplicationError::Validation(_))));
    }

    #[tokio::test]
    async fn negative_rate_is_rejected() {
        let app = setup().await;
        let result = app
            .tax_rates
            .create_tax_rate(TaxRateFields {
                name: "Bad Rate".into(),
                rate_basis_points: -100,
            })
            .await;
        assert!(matches!(result, Err(ApplicationError::Validation(_))));
    }
}
