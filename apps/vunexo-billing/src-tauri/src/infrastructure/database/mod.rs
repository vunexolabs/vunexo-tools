//! SQLite connection pool and migration runner, via SQLx. Repository
//! implementations (e.g. `SqliteInvoiceRepository`) land here in later
//! rounds, implementing ports defined in `crate::application`.

use std::path::Path;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// Opens (creating if needed) the SQLite database file at `db_path` and
/// returns a connection pool. Business tables are not defined yet — see
/// `migrations/`.
pub async fn init_pool(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new().connect(&url).await?;
    Ok(pool)
}

/// Runs pending migrations embedded from `src-tauri/migrations/`.
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!().run(pool).await?;
    Ok(())
}
