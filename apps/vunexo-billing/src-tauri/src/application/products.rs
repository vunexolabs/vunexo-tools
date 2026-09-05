//! Product use cases. Mirrors application/customers.rs exactly.

use std::sync::Arc;

use crate::domain::product::{Product, ProductFields, ProductFilter, ProductListItem};

use super::error::ApplicationError;
use super::ports::infrastructure_error::InfrastructureError;
use super::ports::product_repository::ProductRepository;
use super::ports::transaction::TransactionManager;

pub struct ProductUseCases {
    repo: Arc<dyn ProductRepository>,
    tx_manager: Arc<dyn TransactionManager>,
}

impl ProductUseCases {
    pub fn new(repo: Arc<dyn ProductRepository>, tx_manager: Arc<dyn TransactionManager>) -> Self {
        Self { repo, tx_manager }
    }

    pub async fn create_product(&self, fields: ProductFields) -> Result<Product, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "product name is required".into(),
            ));
        }
        if fields.unit.trim().is_empty() {
            return Err(ApplicationError::Validation("unit is required".into()));
        }
        if fields.price_minor < 0 {
            return Err(ApplicationError::Validation(
                "price cannot be negative".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.create(&mut *tx, fields).await {
            Ok(product) => {
                tx.commit().await?;
                Ok(product)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn update_product(
        &self,
        id: i64,
        fields: ProductFields,
    ) -> Result<Product, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "product name is required".into(),
            ));
        }
        if fields.unit.trim().is_empty() {
            return Err(ApplicationError::Validation("unit is required".into()));
        }
        if fields.price_minor < 0 {
            return Err(ApplicationError::Validation(
                "price cannot be negative".into(),
            ));
        }
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.update(&mut *tx, id, fields).await {
            Ok(product) => {
                tx.commit().await?;
                Ok(product)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn archive_product(&self, id: i64) -> Result<(), ApplicationError> {
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

    pub async fn restore_product(&self, id: i64) -> Result<(), ApplicationError> {
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

    pub async fn delete_product(&self, id: i64) -> Result<(), ApplicationError> {
        let mut tx = self.tx_manager.begin().await?;
        match self.repo.delete(&mut *tx, id).await {
            Ok(()) => {
                tx.commit().await?;
                Ok(())
            }
            Err(InfrastructureError::ConstraintViolation(_)) => {
                let _ = tx.rollback().await;
                Err(ApplicationError::Conflict(
                    "this product has invoice or quote history and can't be deleted — archive it instead"
                        .into(),
                ))
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e.into())
            }
        }
    }

    pub async fn get_product(&self, id: i64) -> Result<Product, ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "product",
                id,
            })
    }

    pub async fn list_products(
        &self,
        filter: ProductFilter,
    ) -> Result<Vec<ProductListItem>, ApplicationError> {
        Ok(self.repo.list(filter).await?)
    }
}

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use crate::application::ports::transaction::TransactionManager;
    use crate::infrastructure::database::sqlite_product_repository::SqliteProductRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};

    use super::*;

    async fn setup() -> (ProductUseCases, sqlx::SqlitePool, std::path::PathBuf) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "vunexo_product_test_{}_{}.db",
            std::process::id(),
            n
        ));
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let repo: Arc<dyn ProductRepository> = Arc::new(SqliteProductRepository::new(pool.clone()));
        (ProductUseCases::new(repo, tx_manager), pool, db_path)
    }

    fn sample_fields() -> ProductFields {
        ProductFields {
            name: "Consulting hour".into(),
            sku: None,
            description: None,
            unit: "hr".into(),
            price_minor: 100_000,
            tax_rate_id: None,
            hsn_sac_code: None,
        }
    }

    /// `create_product` already rejected a blank unit; `update_product` did
    /// not, so an existing product's unit could be cleared out via an edit
    /// even though the standalone create form requires it.
    #[tokio::test]
    async fn update_product_rejects_a_blank_unit_same_as_create() {
        let (products, _pool, db_path) = setup().await;
        let created = products
            .create_product(sample_fields())
            .await
            .expect("create_product");

        let mut fields = sample_fields();
        fields.unit = "   ".into();
        let result = products.update_product(created.id, fields).await;
        assert!(
            matches!(result, Err(ApplicationError::Validation(_))),
            "a blank unit must be rejected on update, exactly as it is on create"
        );

        let _ = std::fs::remove_file(&db_path);
    }

    /// `quote_line_items.product_id` is `ON DELETE RESTRICT` (migration
    /// 0002), same as `invoice_line_items.product_id` — a product referenced
    /// only by a Quote (never invoiced) must still report `has_invoices` and
    /// refuse a hard delete, not offer a Delete the database will then
    /// refuse. No `QuoteUseCases` wiring is pulled in here just to produce
    /// this row — a raw insert of the minimum valid `quotes`/
    /// `quote_line_items` rows is the deliberate, narrower way to reach past
    /// the application layer, same pattern `application::statements`'s test
    /// module uses for its own out-of-band fixture setup.
    #[tokio::test]
    async fn a_product_referenced_only_by_a_quote_line_item_blocks_delete() {
        let (products, pool, db_path) = setup().await;
        let created = products
            .create_product(sample_fields())
            .await
            .expect("create_product");

        let quote_id: i64 = sqlx::query("INSERT INTO quotes DEFAULT VALUES")
            .execute(&pool)
            .await
            .expect("insert quote")
            .last_insert_rowid();
        sqlx::query(
            "INSERT INTO quote_line_items \
             (quote_id, product_id, description, unit, quantity_thousandths, unit_price_minor) \
             VALUES (?, ?, 'Item', 'pcs', 1000, 100)",
        )
        .bind(quote_id)
        .bind(created.id)
        .execute(&pool)
        .await
        .expect("insert quote line item");

        let listed = products
            .list_products(ProductFilter::default())
            .await
            .expect("list_products");
        let row = listed.iter().find(|p| p.product.id == created.id).unwrap();
        assert!(
            row.has_invoices,
            "a product only referenced by a quote must still be reported as blocked-from-delete"
        );

        let result = products.delete_product(created.id).await;
        assert!(
            matches!(result, Err(ApplicationError::Conflict(_))),
            "deleting a product still referenced by a quote line item must be refused, not panic on the FK violation"
        );

        let _ = std::fs::remove_file(&db_path);
    }
}
