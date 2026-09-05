//! Vendor use cases. application-architecture.md's module layout.

use std::sync::Arc;

use crate::domain::vendor::{Vendor, VendorFields, VendorId, VendorListItem};

use super::error::ApplicationError;
use super::ports::vendor_repository::VendorRepository;

pub struct VendorUseCases {
    repo: Arc<dyn VendorRepository>,
}

impl VendorUseCases {
    pub fn new(repo: Arc<dyn VendorRepository>) -> Self {
        Self { repo }
    }

    pub async fn create_vendor(&self, fields: VendorFields) -> Result<Vendor, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "vendor name is required".into(),
            ));
        }
        Ok(self.repo.create(fields).await?)
    }

    pub async fn update_vendor(
        &self,
        id: VendorId,
        fields: VendorFields,
    ) -> Result<Vendor, ApplicationError> {
        if fields.name.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "vendor name is required".into(),
            ));
        }
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "vendor",
                id,
            })?;
        Ok(self.repo.update(id, fields).await?)
    }

    /// user-flows.md §3 — "Delete is blocked ... if the vendor has any
    /// expenses recorded". Checked explicitly here (rather than relying on
    /// the FK constraint alone) so the user gets a clear message before the
    /// attempt, matching application-architecture.md's `DeleteVendor` note.
    pub async fn delete_vendor(&self, id: VendorId) -> Result<(), ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "vendor",
                id,
            })?;
        if self.repo.has_expenses(id).await? {
            return Err(ApplicationError::Validation(
                "this vendor has expenses recorded and can't be deleted".into(),
            ));
        }
        self.repo.delete(id).await?;
        Ok(())
    }

    pub async fn list_vendors(&self) -> Result<Vec<VendorListItem>, ApplicationError> {
        Ok(self.repo.list().await?)
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite — proves the `has_expenses` blocked-delete rule end to
    //! end, which a fake repository would only assert by construction.
    use std::sync::Arc;

    use crate::application::categories::CategoryUseCases;
    use crate::application::expenses::ExpenseUseCases;
    use crate::application::ports::category_repository::CategoryRepository;
    use crate::application::ports::expense_repository::ExpenseRepository;
    use crate::application::ports::receipt_store::ReceiptStore;
    use crate::application::ports::vendor_repository::VendorRepository;
    use crate::domain::category::CategoryFields;
    use crate::domain::expense::ExpenseInput;
    use crate::domain::money::MinorUnits;
    use crate::infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
    use crate::infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
    use crate::infrastructure::database::{init_pool, run_migrations};
    use crate::infrastructure::filesystem::receipts::FsReceiptStore;

    use super::*;

    struct TestApp {
        vendors: VendorUseCases,
        categories: CategoryUseCases,
        expenses: ExpenseUseCases,
        db_path: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.db_path);
        }
    }

    async fn setup(tag: &str) -> TestApp {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let db_path = std::env::temp_dir().join(format!(
            "expense_manager_vendor_test_{tag}_{}_{n}.db",
            std::process::id()
        ));
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");

        let vendor_repo: Arc<dyn VendorRepository> =
            Arc::new(SqliteVendorRepository::new(pool.clone()));
        let category_repo: Arc<dyn CategoryRepository> =
            Arc::new(SqliteCategoryRepository::new(pool.clone()));
        let expense_repo: Arc<dyn ExpenseRepository> = Arc::new(SqliteExpenseRepository::new(pool));
        let receipt_store: Arc<dyn ReceiptStore> =
            Arc::new(FsReceiptStore::new(db_path.parent().unwrap().to_path_buf()));

        TestApp {
            vendors: VendorUseCases::new(vendor_repo.clone()),
            categories: CategoryUseCases::new(category_repo.clone()),
            expenses: ExpenseUseCases::new(expense_repo, vendor_repo, category_repo, receipt_store),
            db_path,
        }
    }

    #[tokio::test]
    async fn vendor_crud_round_trips() {
        let app = setup("crud").await;
        let created = app
            .vendors
            .create_vendor(VendorFields {
                name: "Acme Supplies".into(),
                contact: Some("acme@example.com".into()),
                notes: None,
            })
            .await
            .expect("create_vendor");
        assert_eq!(created.name, "Acme Supplies");

        let updated = app
            .vendors
            .update_vendor(
                created.id,
                VendorFields {
                    name: "Acme Supplies Ltd".into(),
                    contact: Some("acme@example.com".into()),
                    notes: Some("Renamed".into()),
                },
            )
            .await
            .expect("update_vendor");
        assert_eq!(updated.name, "Acme Supplies Ltd");

        let list = app.vendors.list_vendors().await.expect("list_vendors");
        assert_eq!(list.len(), 1);
        assert!(!list[0].has_expenses);

        app.vendors
            .delete_vendor(created.id)
            .await
            .expect("delete_vendor");
        assert!(app.vendors.list_vendors().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deleting_a_vendor_with_expenses_is_blocked() {
        let app = setup("blocked").await;
        let vendor = app
            .vendors
            .create_vendor(VendorFields {
                name: "Blocked Vendor".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Test Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();

        app.expenses
            .create_expense(ExpenseInput {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
                amount: MinorUnits(10_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: Some(vendor.id),
                category_id: category.id,
            })
            .await
            .expect("create_expense");

        let err = app.vendors.delete_vendor(vendor.id).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Validation(_)));

        let list = app.vendors.list_vendors().await.unwrap();
        assert!(list[0].has_expenses);
    }
}
