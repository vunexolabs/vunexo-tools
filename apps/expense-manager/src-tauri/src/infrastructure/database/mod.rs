//! SQLite connection pool and migration runner, via SQLx. Repository
//! implementations here implement the ports defined in
//! `crate::application::ports`. No `TransactionManager` (unlike Billing) —
//! application-architecture.md's explicit decision, since nothing in this
//! domain does a multi-table write that must succeed-or-fail together.

pub mod database_file;
pub mod sqlite_business_repository;
pub mod sqlite_category_repository;
pub mod sqlite_dashboard_repository;
pub mod sqlite_expense_repository;
pub mod sqlite_report_repository;
pub mod sqlite_vendor_repository;

use std::path::Path;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::domain::category::STARTER_CATEGORIES;

/// Opens (creating if needed) the SQLite database file at `db_path` and
/// returns a connection pool.
pub async fn init_pool(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new().connect(&url).await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;
    Ok(pool)
}

/// Runs pending migrations embedded from `src-tauri/migrations/`.
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!().run(pool).await?;
    Ok(())
}

/// Seeds the starter category set on first run (user-flows.md §4). Only
/// seeds when `categories` is empty, so this is safe to call on every
/// startup and never re-adds a category the user deliberately deleted.
/// `business` is deliberately NOT seeded here: its absence is the first-run
/// signal (user-flows.md §1).
pub async fn seed_defaults(pool: &SqlitePool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        for (name, default_deductible) in STARTER_CATEGORIES {
            sqlx::query("INSERT INTO categories (name, default_deductible) VALUES (?, ?)")
                .bind(name)
                .bind(default_deductible)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seed_defaults_seeds_the_starter_category_set_exactly_once() {
        let dir =
            std::env::temp_dir().join(format!("expense_manager_seed_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("seed.db");
        let pool = init_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();
        seed_defaults(&pool).await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, STARTER_CATEGORIES.len() as i64);

        let rent_deductible: bool =
            sqlx::query_scalar("SELECT default_deductible FROM categories WHERE name = 'Rent'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(rent_deductible);

        // Deleting a seeded category, then calling seed_defaults again, must
        // not resurrect it — seeding only ever fires when the table is
        // completely empty.
        sqlx::query("DELETE FROM categories WHERE name = 'Miscellaneous'")
            .execute(&pool)
            .await
            .unwrap();
        seed_defaults(&pool).await.unwrap();
        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_after, STARTER_CATEGORIES.len() as i64 - 1);

        drop(pool);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
