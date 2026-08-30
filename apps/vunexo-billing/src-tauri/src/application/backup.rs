//! Backup and restore use cases. user-flows.md §9, database-schema.md §9.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::domain::backup::{check_restorable, BackupMetadata, BackupRejection};
use crate::domain::business::resolve_logo_path;

use super::error::ApplicationError;
use super::ports::backup_archive::{ArchiveContents, BackupArchive};
use super::ports::business_repository::BusinessRepository;
use super::ports::database_file::DatabaseFile;

/// Where restored `assets/` land, under the app's data directory, so
/// `business.logo_path` keeps resolving after a restore (database-schema.md
/// §9's reason for `.vbx` being an archive rather than a bare database copy).
pub const ASSETS_DIRECTORY: &str = "assets";

pub struct BackupUseCases {
    database_file: Arc<dyn DatabaseFile>,
    archive: Arc<dyn BackupArchive>,
    business_repo: Arc<dyn BusinessRepository>,
    data_directory: PathBuf,
    app_version: String,
    platform: String,
}

impl BackupUseCases {
    pub fn new(
        database_file: Arc<dyn DatabaseFile>,
        archive: Arc<dyn BackupArchive>,
        business_repo: Arc<dyn BusinessRepository>,
        data_directory: PathBuf,
        app_version: String,
        platform: String,
    ) -> Self {
        Self {
            database_file,
            archive,
            business_repo,
            data_directory,
            app_version,
            platform,
        }
    }

    /// Writes a `.vbx` to `destination`. Read-only with respect to
    /// application data: it snapshots and copies, and a failure part-way
    /// leaves the live database untouched.
    pub async fn backup_to(&self, destination: &Path) -> Result<(), ApplicationError> {
        // The snapshot is staged next to the archive rather than in the data
        // directory, so a half-written temp can never be mistaken for the
        // live database if the process dies here.
        let snapshot = staging_path(destination);
        self.database_file.snapshot_to(&snapshot).await?;

        let assets = self.assets_to_archive().await?;
        let metadata = BackupMetadata::new(&self.app_version, &self.platform, Utc::now());

        let result = self.archive.write(
            destination,
            ArchiveContents {
                metadata: &metadata,
                database: &snapshot,
                assets: &assets,
            },
        );
        let _ = std::fs::remove_file(&snapshot);
        result?;
        Ok(())
    }

    /// Reads an archive's metadata without unpacking it, and refuses one this
    /// build can't read — so the UI can tell the user *before* asking them to
    /// confirm replacing all their data.
    pub fn inspect_backup(&self, source: &Path) -> Result<BackupMetadata, ApplicationError> {
        // Choosing the wrong file is a *user* mistake, not an infrastructure
        // failure, so it must not fall through to the generic "something went
        // wrong" that `Infrastructure` errors render as (ui-ux.md §3's error
        // mapping). The underlying message is dropped rather than forwarded —
        // it can carry paths and zip internals.
        let metadata = self.archive.read_metadata(source).map_err(|_| {
            rejected(BackupRejection::Malformed(
                "it may not be a .vbx backup, or it may be damaged".to_string(),
            ))
        })?;
        check_restorable(&metadata).map_err(rejected)?;
        Ok(metadata)
    }

    /// Replaces all local data with the archive's contents.
    ///
    /// Ordering matters and is the whole risk of this operation:
    /// 1. validate first — never close the pool for an archive we'd reject;
    /// 2. unpack to a staging file, so a corrupt or truncated archive is
    ///    discovered while the live database is still intact;
    /// 3. only then close the pool and swap the file into place.
    ///
    /// The caller restarts the app afterwards: every repository still holds
    /// the now-closed pool, and nothing may touch the database again.
    pub async fn restore_from(&self, source: &Path) -> Result<(), ApplicationError> {
        self.inspect_backup(source)?;

        let live = self.database_file.path();
        let staged = staging_path(&live);
        let assets = self.data_directory.join(ASSETS_DIRECTORY);
        self.archive.extract(source, &staged, &assets)?;

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

    /// The files that travel alongside the database. Today that is the
    /// business logo; the `assets/` layout exists so future attachments need
    /// no second format change (database-schema.md §9).
    async fn assets_to_archive(&self) -> Result<Vec<(String, PathBuf)>, ApplicationError> {
        let Some(business) = self.business_repo.get().await? else {
            return Ok(Vec::new());
        };
        let Some(logo_path) = business.logo_path.filter(|p| !p.trim().is_empty()) else {
            return Ok(Vec::new());
        };
        // `logo_path` is relative for anything imported since managed logos
        // landed (`application::business::import_logo_if_chosen`) and
        // absolute for an older, pre-managed one — `resolve_logo_path`
        // handles both, the same way the PDF renderer does.
        let path = resolve_logo_path(&logo_path, &self.data_directory);
        // Archived under a fixed name plus the original extension, so the
        // restore side never has to parse a user-chosen file name.
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        Ok(vec![(format!("business-logo.{extension}"), path)])
    }
}

/// Every reason an archive can be turned away reaches the user as a
/// `Validation` error carrying the specific, actionable sentence.
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
        let staged = staging_path(Path::new("/data/vunexo-billing.db"));
        assert_eq!(staged.parent(), Some(Path::new("/data")));
        assert_eq!(
            staged.file_name().unwrap(),
            "vunexo-billing.db.vunexo-staging"
        );
    }

    #[test]
    fn sidecars_are_named_the_way_sqlite_names_them() {
        let db = Path::new("/data/vunexo-billing.db");
        assert_eq!(
            sidecar_path(db, "-wal"),
            PathBuf::from("/data/vunexo-billing.db-wal")
        );
        assert_eq!(
            sidecar_path(db, "-shm"),
            PathBuf::from("/data/vunexo-billing.db-shm")
        );
    }
}

#[cfg(test)]
mod integration_tests {
    //! Backup and restore are the only operations in this app that can lose
    //! a user's data, so they are exercised against a real SQLite file — the
    //! `VACUUM INTO` snapshot, the pool close, and the file swap are exactly
    //! the parts a unit test with fakes would not cover.

    use crate::application::ports::business_repository::BusinessRepository;
    use crate::application::ports::transaction::TransactionManager;
    use crate::domain::business::Business;
    use crate::infrastructure::database::database_file::SqliteDatabaseFile;
    use crate::infrastructure::database::sqlite_business_repository::SqliteBusinessRepository;
    use crate::infrastructure::database::transaction::SqlxTransactionManager;
    use crate::infrastructure::database::{init_pool, run_migrations, seed_defaults};
    use crate::infrastructure::filesystem::vbx_archive::VbxArchive;

    use super::*;

    struct TestApp {
        backups: BackupUseCases,
        business_repo: Arc<dyn BusinessRepository>,
        tx_manager: Arc<dyn TransactionManager>,
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
        let dir =
            std::env::temp_dir().join(format!("vunexo_backup_{tag}_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn setup(tag: &str) -> TestApp {
        let data_dir = unique_dir(tag);
        let db_path = data_dir.join("vunexo-billing.db");
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");
        seed_defaults(&pool).await.expect("seed_defaults");

        let tx_manager: Arc<dyn TransactionManager> =
            Arc::new(SqlxTransactionManager::new(pool.clone()));
        let business_repo: Arc<dyn BusinessRepository> =
            Arc::new(SqliteBusinessRepository::new(pool.clone()));
        let database_file: Arc<dyn DatabaseFile> = Arc::new(SqliteDatabaseFile::new(pool, db_path));

        TestApp {
            backups: BackupUseCases::new(
                database_file,
                Arc::new(VbxArchive::new()),
                business_repo.clone(),
                data_dir.clone(),
                "0.0.0".to_string(),
                "test".to_string(),
            ),
            business_repo,
            tx_manager,
            data_dir,
        }
    }

    async fn save_business(app: &TestApp, name: &str, logo_path: Option<String>) {
        let mut tx = app.tx_manager.begin().await.expect("begin");
        app.business_repo
            .create(
                tx.as_mut(),
                Business {
                    name: name.to_string(),
                    logo_path,
                    address: Some("1 Mill Road".into()),
                    phone: None,
                    email: None,
                    gstin: Some("29AAAAA0000A1Z5".into()),
                    bank_details: None,
                    upi_id: None,
                    tax_regime_code: crate::domain::tax_regime::TaxRegimeCode::InGst,
                },
            )
            .await
            .expect("create business");
        tx.commit().await.expect("commit");
    }

    #[tokio::test]
    async fn a_backup_captures_committed_data_and_leaves_the_live_database_alone() {
        let app = setup("capture").await;
        save_business(&app, "Acme Traders", None).await;

        let archive = app.data_dir.join("backup.vbx");
        app.backups.backup_to(&archive).await.expect("backup_to");

        assert!(archive.exists(), "the archive must be written");
        let metadata = app.backups.inspect_backup(&archive).expect("inspect");
        assert_eq!(metadata.app_version, "0.0.0");
        assert_eq!(metadata.platform, "test");

        // Read-only: the live database still answers, and still has the data.
        let live = app.business_repo.get().await.expect("get").expect("some");
        assert_eq!(live.name, "Acme Traders");

        // No staging file survives a successful backup.
        assert!(!app.data_dir.join("backup.vbx.vunexo-staging").exists());
    }

    #[tokio::test]
    async fn the_business_logo_travels_inside_the_archive() {
        // database-schema.md §9's whole reason for `.vbx` being an archive
        // rather than a bare database copy.
        let app = setup("logo").await;
        let logo = app.data_dir.join("logo.png");
        std::fs::write(&logo, b"pretend png bytes").unwrap();
        save_business(&app, "Acme", Some(logo.to_string_lossy().to_string())).await;

        let archive = app.data_dir.join("backup.vbx");
        app.backups.backup_to(&archive).await.expect("backup_to");

        let restore_dir = unique_dir("logo_restore");
        let restored_db = restore_dir.join("db.sqlite");
        let written = VbxArchive::new()
            .extract(&archive, &restored_db, &restore_dir.join("assets"))
            .expect("extract");
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].file_name().unwrap(), "business-logo.png");
        assert_eq!(std::fs::read(&written[0]).unwrap(), b"pretend png bytes");
        let _ = std::fs::remove_dir_all(&restore_dir);
    }

    #[tokio::test]
    async fn a_logo_that_has_gone_missing_does_not_sink_the_backup() {
        // The database is the irreplaceable part; a moved image is not a
        // reason to leave the user with no backup at all.
        let app = setup("missing_logo").await;
        save_business(&app, "Acme", Some("/nope/not/here.png".into())).await;

        let archive = app.data_dir.join("backup.vbx");
        app.backups.backup_to(&archive).await.expect("backup_to");
        assert!(app.backups.inspect_backup(&archive).is_ok());
    }

    #[tokio::test]
    async fn restoring_replaces_the_live_database_with_the_archives_contents() {
        let app = setup("restore").await;
        save_business(&app, "Before Backup", None).await;

        let archive = app.data_dir.join("backup.vbx");
        app.backups.backup_to(&archive).await.expect("backup_to");

        // Change the data *after* the backup, so a successful restore is
        // visibly a rollback rather than a no-op.
        let mut tx = app.tx_manager.begin().await.expect("begin");
        let mut business = app.business_repo.get().await.unwrap().unwrap();
        business.name = "After Backup".into();
        app.business_repo
            .update(tx.as_mut(), business)
            .await
            .expect("update");
        tx.commit().await.expect("commit");
        assert_eq!(
            app.business_repo.get().await.unwrap().unwrap().name,
            "After Backup"
        );

        app.backups.restore_from(&archive).await.expect("restore");

        // The pool is closed now, exactly as it is in the running app, so the
        // restored file is verified by opening it fresh — which is what the
        // app itself does after it restarts.
        let db_path = app.data_dir.join("vunexo-billing.db");
        let pool = init_pool(&db_path).await.expect("reopen");
        let repo = SqliteBusinessRepository::new(pool);
        assert_eq!(
            repo.get().await.unwrap().unwrap().name,
            "Before Backup",
            "restore must roll the database back to the archived state"
        );
        assert!(!app
            .data_dir
            .join("vunexo-billing.db.vunexo-staging")
            .exists());
    }

    #[tokio::test]
    async fn a_managed_logo_survives_a_restore_onto_a_different_data_directory() {
        // The bug this exists to catch: a *relative*, app-managed logo_path
        // must resolve correctly again after landing in a brand new data
        // directory (a different machine, or a different user account) —
        // unlike a legacy absolute path, which only ever worked on the
        // machine that chose it.
        let app = setup("managed_logo_restore").await;
        let logo = app.data_dir.join("assets").join("business-logo.png");
        std::fs::create_dir_all(logo.parent().unwrap()).unwrap();
        std::fs::write(&logo, b"pretend png bytes").unwrap();
        // What `import_logo_if_chosen` actually stores: relative, not absolute.
        save_business(&app, "Acme", Some("assets/business-logo.png".to_string())).await;

        let archive = app.data_dir.join("backup.vbx");
        app.backups.backup_to(&archive).await.expect("backup_to");

        // A fresh data directory — standing in for "a different machine" —
        // with nothing pre-existing at the path the restored logo_path names.
        let other_machine_dir = unique_dir("managed_logo_other_machine");
        let restored_db = other_machine_dir.join("vunexo-billing.db");
        let restored_assets = other_machine_dir.join("assets");
        VbxArchive::new()
            .extract(&archive, &restored_db, &restored_assets)
            .expect("extract");

        let pool = init_pool(&restored_db).await.expect("reopen");
        let repo = SqliteBusinessRepository::new(pool);
        let restored = repo.get().await.unwrap().unwrap();

        // The stored value is unchanged (still relative) — resolving it
        // against the *new* data directory must still find the file.
        assert_eq!(
            restored.logo_path.as_deref(),
            Some("assets/business-logo.png")
        );
        let resolved = crate::domain::business::resolve_logo_path(
            restored.logo_path.as_deref().unwrap(),
            &other_machine_dir,
        );
        assert_eq!(std::fs::read(&resolved).unwrap(), b"pretend png bytes");

        let _ = std::fs::remove_dir_all(&other_machine_dir);
    }

    #[tokio::test]
    async fn a_backup_from_a_newer_format_is_refused_before_anything_is_replaced() {
        let app = setup("too_new").await;
        save_business(&app, "Untouched", None).await;

        // Hand-build an archive claiming a format this build can't read.
        let archive = app.data_dir.join("from-the-future.vbx");
        let snapshot = app.data_dir.join("snapshot.sqlite");
        std::fs::write(&snapshot, b"SQLite format 3\0").unwrap();
        let mut metadata = BackupMetadata::new("99.0.0", "test", Utc::now());
        metadata.format_version = crate::domain::backup::BACKUP_FORMAT_VERSION + 1;
        VbxArchive::new()
            .write(
                &archive,
                ArchiveContents {
                    metadata: &metadata,
                    database: &snapshot,
                    assets: &[],
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
            app.business_repo.get().await.unwrap().unwrap().name,
            "Untouched"
        );
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_backup_is_refused_with_a_message_the_user_can_act_on() {
        let app = setup("garbage").await;
        save_business(&app, "Untouched", None).await;

        let not_a_backup = app.data_dir.join("holiday-photo.vbx");
        std::fs::write(&not_a_backup, b"definitely not a zip").unwrap();

        let err = app.backups.restore_from(&not_a_backup).await.unwrap_err();
        match err {
            ApplicationError::Validation(message) => {
                // Not the generic infrastructure message — picking the wrong
                // file is a user mistake and has to read like one.
                assert!(message.contains("isn't a usable Vunexo Billing backup"));
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
        assert_eq!(
            app.business_repo.get().await.unwrap().unwrap().name,
            "Untouched"
        );
    }
}
