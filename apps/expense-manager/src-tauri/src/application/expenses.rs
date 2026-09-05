//! Expense use cases. application-architecture.md's module layout — this is
//! the module the locked doc calls out as carrying "the one rule most likely
//! to be gotten wrong": `CreateExpense` resolves and writes
//! `vendor_name_snapshot`/`category_name_snapshot` from the live vendor/
//! category row at creation time; `UpdateExpense` must **not** re-snapshot
//! unless the user explicitly re-picks a different vendor/category on that
//! same edit (database-schema.md §4, application-architecture.md's
//! `UpdateExpense` note).

use std::path::Path;
use std::sync::Arc;

use crate::domain::expense::{Expense, ExpenseFilter, ExpenseInput, ExpenseToSave};

use super::error::ApplicationError;
use super::ports::category_repository::CategoryRepository;
use super::ports::expense_repository::ExpenseRepository;
use super::ports::receipt_store::ReceiptStore;
use super::ports::vendor_repository::VendorRepository;

pub struct ExpenseUseCases {
    repo: Arc<dyn ExpenseRepository>,
    vendor_repo: Arc<dyn VendorRepository>,
    category_repo: Arc<dyn CategoryRepository>,
    receipt_store: Arc<dyn ReceiptStore>,
}

impl ExpenseUseCases {
    pub fn new(
        repo: Arc<dyn ExpenseRepository>,
        vendor_repo: Arc<dyn VendorRepository>,
        category_repo: Arc<dyn CategoryRepository>,
        receipt_store: Arc<dyn ReceiptStore>,
    ) -> Self {
        Self {
            repo,
            vendor_repo,
            category_repo,
            receipt_store,
        }
    }

    fn validate(input: &ExpenseInput) -> Result<(), ApplicationError> {
        if input.payment_method.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "payment method is required".into(),
            ));
        }
        if input.amount.as_i64() < 0 {
            return Err(ApplicationError::Validation(
                "amount cannot be negative".into(),
            ));
        }
        if input.tax_amount.as_i64() < 0 {
            return Err(ApplicationError::Validation(
                "tax amount cannot be negative".into(),
            ));
        }
        Ok(())
    }

    /// Reads the live vendor/category rows once and copies their current
    /// names in — never re-read afterward (database-schema.md §4).
    pub async fn create_expense(&self, input: ExpenseInput) -> Result<Expense, ApplicationError> {
        Self::validate(&input)?;

        let category = self
            .category_repo
            .find_by_id(input.category_id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "category",
                id: input.category_id,
            })?;

        let vendor_name_snapshot = match input.vendor_id {
            None => None,
            Some(vendor_id) => {
                let vendor = self.vendor_repo.find_by_id(vendor_id).await?.ok_or(
                    ApplicationError::NotFound {
                        entity: "vendor",
                        id: vendor_id,
                    },
                )?;
                Some(vendor.name)
            }
        };

        let to_save = ExpenseToSave {
            date: input.date,
            amount: input.amount,
            tax_amount: input.tax_amount,
            itc_eligible: input.itc_eligible,
            deductible: input.deductible,
            payment_method: input.payment_method,
            notes: input.notes,
            vendor_id: input.vendor_id,
            vendor_name_snapshot,
            category_id: input.category_id,
            category_name_snapshot: category.name,
        };
        Ok(self.repo.create(to_save).await?)
    }

    /// Does **not** re-snapshot the vendor/category name from the current
    /// live record just because *other* fields changed — only when the user
    /// picks a *different* vendor/category on this edit does that newly
    /// picked one's current name get snapshotted, exactly the way picking it
    /// fresh on `CreateExpense` would (application-architecture.md's
    /// `UpdateExpense` note, the rule most likely to be gotten wrong).
    pub async fn update_expense(
        &self,
        id: i64,
        input: ExpenseInput,
    ) -> Result<Expense, ApplicationError> {
        Self::validate(&input)?;

        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })?;

        let category_name_snapshot = if input.category_id != existing.category_id {
            let category = self
                .category_repo
                .find_by_id(input.category_id)
                .await?
                .ok_or(ApplicationError::NotFound {
                    entity: "category",
                    id: input.category_id,
                })?;
            category.name
        } else {
            existing.category_name_snapshot
        };

        let vendor_name_snapshot = if input.vendor_id == existing.vendor_id {
            existing.vendor_name_snapshot
        } else {
            match input.vendor_id {
                None => None,
                Some(vendor_id) => {
                    let vendor = self.vendor_repo.find_by_id(vendor_id).await?.ok_or(
                        ApplicationError::NotFound {
                            entity: "vendor",
                            id: vendor_id,
                        },
                    )?;
                    Some(vendor.name)
                }
            }
        };

        let to_save = ExpenseToSave {
            date: input.date,
            amount: input.amount,
            tax_amount: input.tax_amount,
            itc_eligible: input.itc_eligible,
            deductible: input.deductible,
            payment_method: input.payment_method,
            notes: input.notes,
            vendor_id: input.vendor_id,
            vendor_name_snapshot,
            category_id: input.category_id,
            category_name_snapshot,
        };
        Ok(self.repo.update(id, to_save).await?)
    }

    /// Deletes the DB row, then deletes the receipt file if any — best
    /// effort, since a file already missing from disk must not block the
    /// (already-succeeded) DB delete.
    pub async fn delete_expense(&self, id: i64) -> Result<(), ApplicationError> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })?;
        self.repo.delete(id).await?;
        if let Some(receipt_path) = existing.receipt_path {
            let _ = self.receipt_store.remove(&receipt_path);
        }
        Ok(())
    }

    pub async fn get_expense(&self, id: i64) -> Result<Expense, ApplicationError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })
    }

    pub async fn list_expenses(
        &self,
        filter: ExpenseFilter,
    ) -> Result<Vec<Expense>, ApplicationError> {
        Ok(self.repo.list(filter).await?)
    }

    /// Only for an expense with no receipt yet — `replace_receipt` is the
    /// command for swapping an existing one, so the two commands stay
    /// unambiguous about which file (if any) is being replaced.
    pub async fn attach_receipt(
        &self,
        id: i64,
        source: &Path,
    ) -> Result<Expense, ApplicationError> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })?;
        if existing.receipt_path.is_some() {
            return Err(ApplicationError::Validation(
                "this expense already has a receipt attached — use replace instead".into(),
            ));
        }
        let managed = self.receipt_store.attach(source)?;
        self.repo.set_receipt_path(id, Some(managed)).await?;
        self.get_expense(id).await
    }

    /// Copies the new file in and points the row at it *before* deleting the
    /// old one — the row must never point at a deleted file mid-operation
    /// (application-architecture.md's `ReplaceReceipt` note).
    pub async fn replace_receipt(
        &self,
        id: i64,
        source: &Path,
    ) -> Result<Expense, ApplicationError> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })?;
        let managed = self.receipt_store.attach(source)?;
        self.repo.set_receipt_path(id, Some(managed)).await?;
        if let Some(old) = existing.receipt_path {
            let _ = self.receipt_store.remove(&old);
        }
        self.get_expense(id).await
    }

    pub async fn remove_receipt(&self, id: i64) -> Result<Expense, ApplicationError> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotFound {
                entity: "expense",
                id,
            })?;
        if let Some(old) = existing.receipt_path {
            self.repo.set_receipt_path(id, None).await?;
            let _ = self.receipt_store.remove(&old);
        }
        self.get_expense(id).await
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite, real filesystem receipt store — exercises exactly the
    //! rules application-architecture.md calls out: snapshot-on-create,
    //! no-resnapshot-on-unrelated-update, resnapshot-on-repick, and
    //! never-pointing-at-a-deleted-file during a replace.
    use std::sync::Arc;

    use crate::application::categories::CategoryUseCases;
    use crate::application::ports::category_repository::CategoryRepository;
    use crate::application::ports::expense_repository::ExpenseRepository;
    use crate::application::ports::receipt_store::ReceiptStore;
    use crate::application::ports::vendor_repository::VendorRepository;
    use crate::application::vendors::VendorUseCases;
    use crate::domain::category::CategoryFields;
    use crate::domain::money::MinorUnits;
    use crate::domain::vendor::VendorFields;
    use crate::infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
    use crate::infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
    use crate::infrastructure::database::{init_pool, run_migrations};
    use crate::infrastructure::filesystem::receipts::FsReceiptStore;

    use super::*;

    struct TestApp {
        expenses: ExpenseUseCases,
        vendors: VendorUseCases,
        categories: CategoryUseCases,
        data_dir: std::path::PathBuf,
        db_path: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }
    }

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "expense_manager_expense_test_{tag}_{}_{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn setup(tag: &str) -> TestApp {
        let data_dir = unique_dir(tag);
        let db_path = data_dir.join("expense-manager.db");
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");

        let vendor_repo: Arc<dyn VendorRepository> =
            Arc::new(SqliteVendorRepository::new(pool.clone()));
        let category_repo: Arc<dyn CategoryRepository> =
            Arc::new(SqliteCategoryRepository::new(pool.clone()));
        let expense_repo: Arc<dyn ExpenseRepository> = Arc::new(SqliteExpenseRepository::new(pool));
        let receipt_store: Arc<dyn ReceiptStore> = Arc::new(FsReceiptStore::new(data_dir.clone()));

        TestApp {
            expenses: ExpenseUseCases::new(
                expense_repo,
                vendor_repo.clone(),
                category_repo.clone(),
                receipt_store,
            ),
            vendors: VendorUseCases::new(vendor_repo),
            categories: CategoryUseCases::new(category_repo),
            data_dir,
            db_path,
        }
    }

    fn base_input(vendor_id: Option<i64>, category_id: i64) -> ExpenseInput {
        ExpenseInput {
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            amount: MinorUnits(50_000),
            tax_amount: MinorUnits(9_000),
            itc_eligible: true,
            deductible: true,
            payment_method: "Card".into(),
            notes: Some("Office chairs".into()),
            vendor_id,
            category_id,
        }
    }

    #[tokio::test]
    async fn create_expense_snapshots_vendor_and_category_name_and_deductible_at_creation() {
        let app = setup("create_snapshot").await;
        let vendor = app
            .vendors
            .create_vendor(VendorFields {
                name: "Office World".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Office Supplies".into(),
                default_deductible: true,
            })
            .await
            .unwrap();

        let expense = app
            .expenses
            .create_expense(base_input(Some(vendor.id), category.id))
            .await
            .expect("create_expense");

        assert_eq!(
            expense.vendor_name_snapshot.as_deref(),
            Some("Office World")
        );
        assert_eq!(expense.category_name_snapshot, "Office Supplies");
        assert!(expense.deductible);
        assert_eq!(expense.amount, MinorUnits(50_000));
        assert_eq!(expense.tax_amount, MinorUnits(9_000));
    }

    #[tokio::test]
    async fn renaming_vendor_or_category_never_changes_an_existing_expenses_snapshot() {
        let app = setup("immutability").await;
        let vendor = app
            .vendors
            .create_vendor(VendorFields {
                name: "Old Vendor Name".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Old Category Name".into(),
                default_deductible: false,
            })
            .await
            .unwrap();

        // The frontend would have pre-filled `deductible: false` from this
        // category's current default at the moment of creation — expressed
        // directly here since that pre-fill is a frontend concern, not
        // something this backend re-derives.
        let mut input = base_input(Some(vendor.id), category.id);
        input.deductible = false;
        let expense = app.expenses.create_expense(input).await.unwrap();
        assert!(!expense.deductible);

        app.vendors
            .update_vendor(
                vendor.id,
                VendorFields {
                    name: "New Vendor Name".into(),
                    contact: None,
                    notes: None,
                },
            )
            .await
            .unwrap();
        app.categories
            .update_category(
                category.id,
                CategoryFields {
                    name: "New Category Name".into(),
                    default_deductible: true, // flag flips; must not touch the existing expense
                },
            )
            .await
            .unwrap();

        let reloaded = app.expenses.get_expense(expense.id).await.unwrap();
        assert_eq!(
            reloaded.vendor_name_snapshot.as_deref(),
            Some("Old Vendor Name"),
            "renaming a vendor must never rewrite an existing expense's snapshot"
        );
        assert_eq!(
            reloaded.category_name_snapshot, "Old Category Name",
            "renaming a category must never rewrite an existing expense's snapshot"
        );
        assert!(
            !reloaded.deductible,
            "a category's default_deductible changing later must not flip an existing expense"
        );
    }

    #[tokio::test]
    async fn update_expense_does_not_resnapshot_when_vendor_and_category_are_unchanged() {
        let app = setup("no_resnapshot").await;
        let vendor = app
            .vendors
            .create_vendor(VendorFields {
                name: "Stable Vendor".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Stable Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        let expense = app
            .expenses
            .create_expense(base_input(Some(vendor.id), category.id))
            .await
            .unwrap();

        // Rename the live records *before* the edit, so a wrong
        // implementation that re-reads them on update would be caught.
        app.vendors
            .update_vendor(
                vendor.id,
                VendorFields {
                    name: "Renamed Mid-Flight".into(),
                    contact: None,
                    notes: None,
                },
            )
            .await
            .unwrap();

        let mut edited_input = base_input(Some(vendor.id), category.id);
        edited_input.notes = Some("Updated notes only".into());
        edited_input.amount = MinorUnits(60_000);

        let updated = app
            .expenses
            .update_expense(expense.id, edited_input)
            .await
            .expect("update_expense");

        assert_eq!(
            updated.vendor_name_snapshot.as_deref(),
            Some("Stable Vendor"),
            "an unrelated field edit must not re-snapshot the vendor's now-current name"
        );
        assert_eq!(updated.notes.as_deref(), Some("Updated notes only"));
        assert_eq!(updated.amount, MinorUnits(60_000));
    }

    #[tokio::test]
    async fn update_expense_resnapshots_only_when_a_different_vendor_or_category_is_picked() {
        let app = setup("resnapshot_on_repick").await;
        let vendor_a = app
            .vendors
            .create_vendor(VendorFields {
                name: "Vendor A".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let vendor_b = app
            .vendors
            .create_vendor(VendorFields {
                name: "Vendor B".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();

        let expense = app
            .expenses
            .create_expense(base_input(Some(vendor_a.id), category.id))
            .await
            .unwrap();
        assert_eq!(expense.vendor_name_snapshot.as_deref(), Some("Vendor A"));

        let repick_input = base_input(Some(vendor_b.id), category.id);
        let updated = app
            .expenses
            .update_expense(expense.id, repick_input)
            .await
            .expect("update_expense with a re-picked vendor");

        assert_eq!(
            updated.vendor_name_snapshot.as_deref(),
            Some("Vendor B"),
            "picking a different vendor on edit must snapshot its current name"
        );
    }

    #[tokio::test]
    async fn receipt_attach_replace_and_remove_never_leave_a_dangling_reference() {
        let app = setup("receipts").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        let expense = app
            .expenses
            .create_expense(base_input(None, category.id))
            .await
            .unwrap();
        assert!(expense.receipt_path.is_none());

        let receipt_a = app.data_dir.join("receipt-a.jpg");
        std::fs::write(&receipt_a, b"first receipt bytes").unwrap();
        let attached = app
            .expenses
            .attach_receipt(expense.id, &receipt_a)
            .await
            .expect("attach_receipt");
        let first_path = attached.receipt_path.clone().expect("has a receipt");
        let first_absolute = app.data_dir.join(&first_path);
        assert!(first_absolute.exists());

        // A second attach must be refused — replace is the command for that.
        let second_attach_err = app
            .expenses
            .attach_receipt(expense.id, &receipt_a)
            .await
            .unwrap_err();
        assert!(matches!(second_attach_err, ApplicationError::Validation(_)));

        let receipt_b = app.data_dir.join("receipt-b.jpg");
        std::fs::write(&receipt_b, b"second receipt bytes").unwrap();
        let replaced = app
            .expenses
            .replace_receipt(expense.id, &receipt_b)
            .await
            .expect("replace_receipt");
        let second_path = replaced.receipt_path.clone().expect("still has a receipt");
        assert_ne!(
            second_path, first_path,
            "replace must write under a new managed name"
        );
        assert!(
            app.data_dir.join(&second_path).exists(),
            "the new receipt file must exist"
        );
        assert!(
            !first_absolute.exists(),
            "the old receipt file must be deleted after the new one is confirmed written"
        );

        let removed = app
            .expenses
            .remove_receipt(expense.id)
            .await
            .expect("remove_receipt");
        assert!(removed.receipt_path.is_none());
        assert!(!app.data_dir.join(&second_path).exists());
    }

    #[tokio::test]
    async fn deleting_an_expense_also_deletes_its_receipt_file() {
        let app = setup("delete_with_receipt").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        let expense = app
            .expenses
            .create_expense(base_input(None, category.id))
            .await
            .unwrap();
        let receipt = app.data_dir.join("receipt.jpg");
        std::fs::write(&receipt, b"bytes").unwrap();
        let attached = app
            .expenses
            .attach_receipt(expense.id, &receipt)
            .await
            .unwrap();
        let stored_path = app.data_dir.join(attached.receipt_path.unwrap());
        assert!(stored_path.exists());

        app.expenses
            .delete_expense(expense.id)
            .await
            .expect("delete_expense");
        assert!(!stored_path.exists());
        assert!(app.expenses.get_expense(expense.id).await.is_err());

        let _ = &app.db_path;
    }
}
