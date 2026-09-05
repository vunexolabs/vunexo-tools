//! Backup and restore use cases. user-flows.md §9, database-schema.md §7/§9.
//! Mirrors `vunexo-billing`'s `BackupUseCases` almost exactly; the one real
//! difference is that this product bundles a whole `receipts/` directory of
//! files (however many expenses have one attached) rather than a single
//! named logo asset.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::domain::backup::{check_restorable, BackupMetadata, BackupRejection};
use crate::domain::receipt::RECEIPTS_DIRECTORY;

use super::error::ApplicationError;
use super::ports::backup_archive::{ArchiveContents, BackupArchive};
use super::ports::database_file::DatabaseFile;
use super::ports::receipt_store::ReceiptStore;

pub struct BackupUseCases {
    database_file: Arc<dyn DatabaseFile>,
    archive: Arc<dyn BackupArchive>,
    receipt_store: Arc<dyn ReceiptStore>,
    data_directory: PathBuf,
    app_version: String,
    platform: String,
}

impl BackupUseCases {
    pub fn new(
        database_file: Arc<dyn DatabaseFile>,
        archive: Arc<dyn BackupArchive>,
        receipt_store: Arc<dyn ReceiptStore>,
        data_directory: PathBuf,
        app_version: String,
        platform: String,
    ) -> Self {
        Self {
            database_file,
            archive,
            receipt_store,
            data_directory,
            app_version,
            platform,
        }
    }

    /// Writes a `.vex` to `destination`. Read-only with respect to
    /// application data: it snapshots and copies, and a failure part-way
    /// leaves the live database untouched.
    pub async fn backup_to(&self, destination: &Path) -> Result<(), ApplicationError> {
        // The snapshot is staged next to the archive rather than in the data
        // directory, so a half-written temp can never be mistaken for the
        // live database if the process dies here.
        let snapshot = staging_path(destination);
        self.database_file.snapshot_to(&snapshot).await?;

        let receipts = self.receipt_store.list_all()?;
        let metadata = BackupMetadata::new(&self.app_version, &self.platform, Utc::now());

        let result = self.archive.write(
            destination,
            ArchiveContents {
                metadata: &metadata,
                database: &snapshot,
                receipts: &receipts,
            },
        );
        let _ = std::fs::remove_file(&snapshot);
        result?;
        Ok(())
    }

    /// Reads an archive's metadata without unpacking it, so the UI can tell
    /// the user *before* asking them to confirm replacing all their data.
    pub fn inspect_backup(&self, source: &Path) -> Result<BackupMetadata, ApplicationError> {
        let metadata = self.archive.read_metadata(source).map_err(|_| {
            rejected(BackupRejection::Malformed(
                "it may not be a .vex backup, or it may be damaged".to_string(),
            ))
        })?;
        check_restorable(&metadata).map_err(rejected)?;
        Ok(metadata)
    }

    /// Replaces all local data with the archive's contents. Same ordering
    /// discipline as Billing's restore: validate first, unpack to staging
    /// while the live database is still intact, only then close the pool
    /// and swap the file into place. The caller restarts the app afterwards.
    pub async fn restore_from(&self, source: &Path) -> Result<(), ApplicationError> {
        self.inspect_backup(source)?;

        let live = self.database_file.path();
        let staged = staging_path(&live);
        let receipts_directory = self.data_directory.join(RECEIPTS_DIRECTORY);
        self.archive.extract(source, &staged, &receipts_directory)?;

        self.database_file.close().await;

        // WAL and shared-memory sidecars belong to the *old* database. Left
        // behind, SQLite would try to replay them over the restored file.
        for suffix in ["-wal", "-shm"] {
            let sidecar = sidecar_path(&live, suffix);
            let _ = std::fs::remove_file(sidecar);
        }
        std::fs::rename(&staged, &live).map_err(|err| {
            crate::application::ports::infrastructure_error::InfrastructureError::Io(format!(
                "could not put the restored database in place: {err}"
            ))
        })?;
        Ok(())
    }
}

fn rejected(why: BackupRejection) -> ApplicationError {
    ApplicationError::Validation(why.to_string())
}

/// A sibling temp path — same directory, so the eventual rename is a
/// same-filesystem move rather than a copy that could half-complete.
fn staging_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".vunexo-staging");
    target.with_file_name(name)
}

fn sidecar_path(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    database.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_sits_beside_its_target_so_the_swap_is_a_rename() {
        let staged = staging_path(Path::new("/data/expense-manager.db"));
        assert_eq!(staged.parent(), Some(Path::new("/data")));
        assert_eq!(
            staged.file_name().unwrap(),
            "expense-manager.db.vunexo-staging"
        );
    }

    #[test]
    fn sidecars_are_named_the_way_sqlite_names_them() {
        let db = Path::new("/data/expense-manager.db");
        assert_eq!(
            sidecar_path(db, "-wal"),
            PathBuf::from("/data/expense-manager.db-wal")
        );
        assert_eq!(
            sidecar_path(db, "-shm"),
            PathBuf::from("/data/expense-manager.db-shm")
        );
    }
}

#[cfg(test)]
mod integration_tests {
    //! Backup and restore are the only operations here that can lose a
    //! user's data, so they're exercised against a real SQLite file and a
    //! real receipt file on disk.
    use crate::application::ports::category_repository::CategoryRepository;
    use crate::application::ports::expense_repository::ExpenseRepository;
    use crate::application::ports::vendor_repository::VendorRepository;
    use crate::domain::category::CategoryFields;
    use crate::domain::expense::ExpenseInput;
    use crate::domain::money::MinorUnits;
    use crate::infrastructure::database::database_file::SqliteDatabaseFile;
    use crate::infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
    use crate::infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
    use crate::infrastructure::database::{init_pool, run_migrations};
    use crate::infrastructure::filesystem::backup::VexArchive;
    use crate::infrastructure::filesystem::receipts::FsReceiptStore;

    use super::*;

    struct TestApp {
        backups: BackupUseCases,
        expenses: crate::application::expenses::ExpenseUseCases,
        categories: crate::application::categories::CategoryUseCases,
        vendors: crate::application::vendors::VendorUseCases,
        data_dir: PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }
    }

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "expense_manager_backup_{tag}_{}_{n}",
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
        let expense_repo: Arc<dyn ExpenseRepository> =
            Arc::new(SqliteExpenseRepository::new(pool.clone()));
        let receipt_store: Arc<dyn ReceiptStore> = Arc::new(FsReceiptStore::new(data_dir.clone()));
        let database_file: Arc<dyn DatabaseFile> = Arc::new(SqliteDatabaseFile::new(pool, db_path));

        TestApp {
            backups: BackupUseCases::new(
                database_file,
                Arc::new(VexArchive::new()),
                receipt_store.clone(),
                data_dir.clone(),
                "0.0.0".to_string(),
                "test".to_string(),
            ),
            expenses: crate::application::expenses::ExpenseUseCases::new(
                expense_repo,
                vendor_repo.clone(),
                category_repo.clone(),
                receipt_store,
            ),
            categories: crate::application::categories::CategoryUseCases::new(category_repo),
            vendors: crate::application::vendors::VendorUseCases::new(vendor_repo),
            data_dir,
        }
    }

    #[tokio::test]
    async fn a_backup_round_trips_and_a_receipt_file_survives_it() {
        let app = setup("receipt_roundtrip").await;
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
            .create_expense(ExpenseInput {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                amount: MinorUnits(1_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: category.id,
            })
            .await
            .unwrap();

        let receipt_source = app.data_dir.join("original-receipt.jpg");
        std::fs::write(&receipt_source, b"receipt bytes to survive backup").unwrap();
        let attached = app
            .expenses
            .attach_receipt(expense.id, &receipt_source)
            .await
            .expect("attach_receipt");
        let receipt_relative = attached.receipt_path.clone().unwrap();

        let archive = app.data_dir.join("backup.vex");
        app.backups.backup_to(&archive).await.expect("backup_to");
        assert!(archive.exists());

        // Simulate the receipt vanishing from the live data directory after
        // backup, so a successful restore is visibly what brought it back.
        std::fs::remove_file(app.data_dir.join(&receipt_relative)).unwrap();
        assert!(!app.data_dir.join(&receipt_relative).exists());

        app.backups
            .restore_from(&archive)
            .await
            .expect("restore_from");

        assert!(
            app.data_dir.join(&receipt_relative).exists(),
            "the receipt file must be restored alongside the database"
        );
        assert_eq!(
            std::fs::read(app.data_dir.join(&receipt_relative)).unwrap(),
            b"receipt bytes to survive backup"
        );

        // The database itself is genuinely restored too, not just the file.
        let db_path = app.data_dir.join("expense-manager.db");
        let pool = init_pool(&db_path).await.expect("reopen");
        let repo = SqliteExpenseRepository::new(pool);
        let reloaded = repo.find_by_id(expense.id).await.unwrap().unwrap();
        assert_eq!(
            reloaded.receipt_path.as_deref(),
            Some(receipt_relative.as_str())
        );
    }

    #[tokio::test]
    async fn a_backup_from_a_newer_format_is_refused_before_anything_is_replaced() {
        let app = setup("too_new").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Untouched Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();

        let archive = app.data_dir.join("from-the-future.vex");
        let snapshot = app.data_dir.join("snapshot.sqlite");
        std::fs::write(&snapshot, b"SQLite format 3\0").unwrap();
        let mut metadata = BackupMetadata::new("99.0.0", "test", Utc::now());
        metadata.format_version = crate::domain::backup::BACKUP_FORMAT_VERSION + 1;
        VexArchive::new()
            .write(
                &archive,
                ArchiveContents {
                    metadata: &metadata,
                    database: &snapshot,
                    receipts: &[],
                },
            )
            .expect("write");

        let err = app.backups.restore_from(&archive).await.unwrap_err();
        match err {
            ApplicationError::Validation(message) => {
                assert!(message.contains("newer version"), "got: {message}");
            }
            other => panic!("expected a validation error, got {other:?}"),
        }

        // Refused *before* the pool closed, so the app is still fully usable.
        assert_eq!(
            app.categories.list_categories().await.unwrap()[0]
                .category
                .name,
            category.name
        );
        let _ = &app.vendors;
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_backup_is_refused_with_an_actionable_message() {
        let app = setup("garbage").await;
        let not_a_backup = app.data_dir.join("holiday-photo.vex");
        std::fs::write(&not_a_backup, b"definitely not a zip").unwrap();

        let err = app.backups.restore_from(&not_a_backup).await.unwrap_err();
        match err {
            ApplicationError::Validation(message) => {
                assert!(message.contains("isn't a usable Vunexo Expense Manager backup"));
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
    }
}
