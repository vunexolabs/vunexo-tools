//! Customer use cases. application-architecture.md §4.

use std::sync::Arc;

use crate::domain::customer::{Customer, CustomerFields, CustomerFilter, CustomerListItem};

use super::error::ApplicationError;
use super::ports::customer_repository::CustomerRepository;
use super::ports::infrastructure_error::InfrastructureError;
use super::ports::transaction::TransactionManager;

pub struct CustomerUseCases {
    repo: Arc<dyn CustomerRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl CustomerUseCases {
    pub fn new(repo: Arc<dyn CustomerRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    pub async fn create_customer(
        &self,
        fields: CustomerFields,
    ) -> Result<Customer, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "customer name is required".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, fields).await {
            Ok(customer) => {
                tx.commit().await?;
                Ok(customer)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn update_customer(
        &self,
        id: i64,
        fields: CustomerFields,
    ) -> Result<Customer, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "customer name is required".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, id, fields).await {
            Ok(customer) => {
                tx.commit().await?;
                Ok(customer)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn archive_customer(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.archive(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn restore_customer(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.restore(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    /// Attempts a hard delete; a referenced customer surfaces as
    /// `ApplicationError::Conflict` (application-architecture.md §4)
    /// rather than a raw database error reaching the UI.
    pub async fn delete_customer(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.delete(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(InfrastructureError::ConstraintViolation(_)) => {
                let _ = tx.rollback().await;
                Err(ApplicationError::Conflict(
                    "this customer has invoice or quote history and can't be deleted — archive it instead"
                        .into(),
                ))
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_customer(&self, id: i64) -> Result<Customer, ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "customer",
                id,
            })
    }

    pub async fn list_customers(
        &self,
        filter: CustomerFilter,
    ) -> Result<Vec<CustomerListItem>, ApplicationError> {
        Ok(self.repo.list(filter).await?)
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::application::ports::transaction::TransactionManager;
    use crate::infrastructure::database::sqlite_customer_repository::SqliteCustomerRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    async fn setup() -> (CustomerUseCases, sqlx::SqlitePool, std::path::PathBuf) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_customer_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let repo: Arc<dyn CustomerRepository> =
            Arc::new(SqliteCustomerRepository::new(pool.clone()));
        (CustomerUseCases::new(repo, tx_manager), pool, db_path)
    }

    fn sample_fields() -> CustomerFields {
        CustomerFields {
            name: "Acme Traders".into(),
            phone: None,
            email: None,
            address: None,
            gstin: None,
        }
    }

    /// `quotes.customer_id` is `ON DELETE RESTRICT` (migration 0002), same as
    /// `invoices.customer_id` — a customer referenced only by a Quote (never
    /// invoiced) must still report `has_invoices` and refuse a hard delete,
    /// not offer a Delete the database will then refuse. A raw insert of the
    /// minimum valid `quotes` row is the deliberate, narrower way to produce
    /// this fixture, mirroring `application::statements`'s test module.
    #[tokio::test]
    async fn a_customer_referenced_only_by_a_quote_blocks_delete() {
        let (customers, pool, db_path) = setup().await;
        let created = customers
            .create_customer(sample_fields())
            .await
            .expect("create_customer");

        sqlx::query("INSERT INTO quotes (customer_id) VALUES (?)")
            .bind(created.id)
            .execute(&pool)
            .await
            .expect("insert quote");

        let listed = customers
            .list_customers(CustomerFilter::default())
            .await
            .expect("list_customers");
        let row = listed.iter().find(|c| c.customer.id == created.id).unwrap();
        assert!(
            row.has_invoices,
            "a customer only referenced by a quote must still be reported as blocked-from-delete"
        );

        let result = customers.delete_customer(created.id).await;
        assert!(
            matches!(result, Err(ApplicationError::Conflict(_))),
            "deleting a customer still referenced by a quote must be refused, not panic on the FK violation"
        );

        let _ = std::fs::remove_file(&db_path);
    }
}
