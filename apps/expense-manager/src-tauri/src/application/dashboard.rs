//! Dashboard use case. user-flows.md §8 — "this period's total spend, a
//! category breakdown, a recent-expenses list."

use std::sync::Arc;

use chrono::{Datelike, NaiveDate, Utc};

use crate::domain::dashboard::DashboardMetrics;

use super::error::ApplicationError;
use super::ports::dashboard_repository::DashboardRepository;

/// How many recent expenses the landing screen shows (user-flows.md §8).
const RECENT_EXPENSES_LIMIT: i64 = 10;

pub struct DashboardUseCases {
    repo: Arc<dyn DashboardRepository>,
}

impl DashboardUseCases {
    pub fn new(repo: Arc<dyn DashboardRepository>) -> Self {
        Self { repo }
    }

    /// Assembles its response entirely from `DashboardRepository` — does no
    /// expense iteration of its own. "This period" is the current calendar
    /// month, the same grain `reports::generate_period_summary` groups by.
    pub async fn get_dashboard_metrics(&self) -> Result<DashboardMetrics, ApplicationError> {
        let today = Utc::now().date_naive();
        let (period_start, period_end) = current_month_range(today);

        Ok(DashboardMetrics {
            period_total_minor: self.repo.period_total(period_start, period_end).await?,
            category_breakdown: self
                .repo
                .category_breakdown(period_start, period_end)
                .await?,
            recent_expenses: self.repo.recent_expenses(RECENT_EXPENSES_LIMIT).await?,
        })
    }
}

/// `[start, end)` for the calendar month `today` falls in.
fn current_month_range(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let (next_year, next_month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    let end = NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap();
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_month_range_spans_exactly_the_calendar_month() {
        let (start, end) = current_month_range(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
    }

    #[test]
    fn current_month_range_handles_december_rollover() {
        let (start, end) = current_month_range(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite — proves the exact metric definitions, not just that the
    //! SQL parses. Dates are computed relative to "now" so the test doesn't
    //! quietly rot as the calendar moves on.
    use std::sync::Arc;

    use crate::application::categories::CategoryUseCases;
    use crate::application::expenses::ExpenseUseCases;
    use crate::application::ports::category_repository::CategoryRepository;
    use crate::application::ports::dashboard_repository::DashboardRepository;
    use crate::application::ports::expense_repository::ExpenseRepository;
    use crate::application::ports::receipt_store::ReceiptStore;
    use crate::application::ports::vendor_repository::VendorRepository;
    use crate::domain::category::CategoryFields;
    use crate::domain::expense::ExpenseInput;
    use crate::domain::money::MinorUnits;
    use crate::infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::database::sqlite_dashboard_repository::SqliteDashboardRepository;
    use crate::infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
    use crate::infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
    use crate::infrastructure::database::{init_pool, run_migrations};
    use crate::infrastructure::filesystem::receipts::FsReceiptStore;

    use super::*;

    struct TestApp {
        dashboard: DashboardUseCases,
        expenses: ExpenseUseCases,
        categories: CategoryUseCases,
        data_dir: std::path::PathBuf,
    }

    impl Drop for TestApp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }
    }

    async fn setup(tag: &str) -> TestApp {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let data_dir = std::env::temp_dir().join(format!(
            "expense_manager_dashboard_test_{tag}_{}_{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("expense-manager.db");
        let pool = init_pool(&db_path).await.expect("init_pool");
        run_migrations(&pool).await.expect("run_migrations");

        let vendor_repo: Arc<dyn VendorRepository> =
            Arc::new(SqliteVendorRepository::new(pool.clone()));
        let category_repo: Arc<dyn CategoryRepository> =
            Arc::new(SqliteCategoryRepository::new(pool.clone()));
        let expense_repo: Arc<dyn ExpenseRepository> =
            Arc::new(SqliteExpenseRepository::new(pool.clone()));
        let dashboard_repo: Arc<dyn DashboardRepository> =
            Arc::new(SqliteDashboardRepository::new(pool));
        let receipt_store: Arc<dyn ReceiptStore> = Arc::new(FsReceiptStore::new(data_dir.clone()));

        TestApp {
            dashboard: DashboardUseCases::new(dashboard_repo),
            expenses: ExpenseUseCases::new(
                expense_repo,
                vendor_repo,
                category_repo.clone(),
                receipt_store,
            ),
            categories: CategoryUseCases::new(category_repo),
            data_dir,
        }
    }

    #[tokio::test]
    async fn dashboard_metrics_match_hand_computed_expectations() {
        let app = setup("metrics").await;
        let rent = app
            .categories
            .create_category(CategoryFields {
                name: "Rent".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        let travel = app
            .categories
            .create_category(CategoryFields {
                name: "Travel".into(),
                default_deductible: true,
            })
            .await
            .unwrap();

        let today = chrono::Utc::now().date_naive();
        let last_month = today - chrono::Duration::days(35);

        app.expenses
            .create_expense(ExpenseInput {
                date: today,
                amount: MinorUnits(10_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: rent.id,
            })
            .await
            .unwrap();
        app.expenses
            .create_expense(ExpenseInput {
                date: today,
                amount: MinorUnits(5_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: travel.id,
            })
            .await
            .unwrap();
        // Outside this period — must not contribute.
        app.expenses
            .create_expense(ExpenseInput {
                date: last_month,
                amount: MinorUnits(99_999),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: rent.id,
            })
            .await
            .unwrap();

        let metrics = app
            .dashboard
            .get_dashboard_metrics()
            .await
            .expect("get_dashboard_metrics");

        assert_eq!(
            metrics.period_total_minor, 15_000,
            "10000 + 5000, excluding last month"
        );
        assert_eq!(metrics.category_breakdown.len(), 2);
        let rent_row = metrics
            .category_breakdown
            .iter()
            .find(|r| r.category_name == "Rent")
            .unwrap();
        assert_eq!(rent_row.total_minor, 10_000);
        assert_eq!(
            metrics.recent_expenses.len(),
            3,
            "recency, not a period filter"
        );
    }
}
