//! Report use cases. calculation-engine.md — thin pass-throughs to
//! `ReportRepository`; every aggregation happens in SQL there, never here.

use std::sync::Arc;

use chrono::NaiveDate;

use crate::domain::report::{
    CategorySummaryResult, DeductibleSummaryResult, PeriodSummaryResult, TaxItcSummaryResult,
    TopVendorsResult,
};

use super::error::ApplicationError;
use super::ports::report_repository::ReportRepository;

/// calculation-engine.md §5 — "Top vendors ... `LIMIT N`"; not specified
/// further, so a generous-but-bounded default that still fits one screen.
const TOP_VENDORS_LIMIT: i64 = 20;

pub struct ReportUseCases {
    repo: Arc<dyn ReportRepository>,
}

impl ReportUseCases {
    pub fn new(repo: Arc<dyn ReportRepository>) -> Self {
        Self { repo }
    }

    pub async fn generate_category_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<CategorySummaryResult, ApplicationError> {
        Ok(self.repo.category_summary(range_start, range_end).await?)
    }

    pub async fn generate_period_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<PeriodSummaryResult, ApplicationError> {
        Ok(self.repo.period_summary(range_start, range_end).await?)
    }

    pub async fn generate_deductible_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<DeductibleSummaryResult, ApplicationError> {
        Ok(self.repo.deductible_summary(range_start, range_end).await?)
    }

    pub async fn generate_tax_itc_summary(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TaxItcSummaryResult, ApplicationError> {
        Ok(self.repo.tax_itc_summary(range_start, range_end).await?)
    }

    pub async fn generate_top_vendors(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<TopVendorsResult, ApplicationError> {
        Ok(self
            .repo
            .top_vendors(range_start, range_end, TOP_VENDORS_LIMIT)
            .await?)
    }
}

#[cfg(test)]
mod integration_tests {
    //! Real SQLite. Every assertion here is one of calculation-engine.md
    //! §7's literal test vectors.
    use std::sync::Arc;

    use crate::application::categories::CategoryUseCases;
    use crate::application::expenses::ExpenseUseCases;
    use crate::application::ports::category_repository::CategoryRepository;
    use crate::application::ports::expense_repository::ExpenseRepository;
    use crate::application::ports::receipt_store::ReceiptStore;
    use crate::application::ports::report_repository::ReportRepository;
    use crate::application::ports::vendor_repository::VendorRepository;
    use crate::application::vendors::VendorUseCases;
    use crate::domain::category::CategoryFields;
    use crate::domain::expense::ExpenseInput;
    use crate::domain::money::MinorUnits;
    use crate::domain::vendor::VendorFields;
    use crate::infrastructure::database::sqlite_category_repository::SqliteCategoryRepository;
    use crate::infrastructure::database::sqlite_expense_repository::SqliteExpenseRepository;
    use crate::infrastructure::database::sqlite_report_repository::SqliteReportRepository;
    use crate::infrastructure::database::sqlite_vendor_repository::SqliteVendorRepository;
    use crate::infrastructure::database::{init_pool, run_migrations};
    use crate::infrastructure::filesystem::receipts::FsReceiptStore;

    use super::*;

    struct TestApp {
        reports: ReportUseCases,
        expenses: ExpenseUseCases,
        vendors: VendorUseCases,
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
            "expense_manager_report_test_{tag}_{}_{n}",
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
        let report_repo: Arc<dyn ReportRepository> = Arc::new(SqliteReportRepository::new(pool));
        let receipt_store: Arc<dyn ReceiptStore> = Arc::new(FsReceiptStore::new(data_dir.clone()));

        TestApp {
            reports: ReportUseCases::new(report_repo),
            expenses: ExpenseUseCases::new(
                expense_repo,
                vendor_repo.clone(),
                category_repo.clone(),
                receipt_store,
            ),
            vendors: VendorUseCases::new(vendor_repo),
            categories: CategoryUseCases::new(category_repo),
            data_dir,
        }
    }

    fn day(offset: i64) -> NaiveDate {
        // A fixed anchor date, not "today" — reports here filter by an
        // explicit range the test also controls, so there's no need to
        // chase the calendar the way the dashboard test does.
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap() + chrono::Duration::days(offset)
    }

    #[tokio::test]
    async fn category_summary_sums_three_expenses_in_one_category() {
        // calculation-engine.md §7 Vector 1: 10000/25000/5000 -> total 40000.
        let app = setup("category_summary").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Vector1 Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        for amount in [10_000, 25_000, 5_000] {
            app.expenses
                .create_expense(ExpenseInput {
                    date: day(0),
                    amount: MinorUnits(amount),
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
        }

        let result = app
            .reports
            .generate_category_summary(day(-1), day(1))
            .await
            .expect("generate_category_summary");
        assert_eq!(result.total_minor, 40_000);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].total_minor, 40_000);
    }

    #[tokio::test]
    async fn deductible_summary_reads_the_stored_flag_not_the_categorys_current_default() {
        // calculation-engine.md §7 Vector 2: deductible=1 amount 10000,
        // deductible=0 amount 5000 -> 10000/5000, never 15000/0.
        let app = setup("deductible_summary").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Vector2 Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        app.expenses
            .create_expense(ExpenseInput {
                date: day(0),
                amount: MinorUnits(10_000),
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
        app.expenses
            .create_expense(ExpenseInput {
                date: day(0),
                amount: MinorUnits(5_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: false,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: category.id,
            })
            .await
            .unwrap();

        // Flip the category's current default after the fact — the summary
        // must still read each expense's own stored flag.
        app.categories
            .update_category(
                category.id,
                CategoryFields {
                    name: "Vector2 Category".into(),
                    default_deductible: false,
                },
            )
            .await
            .unwrap();

        let result = app
            .reports
            .generate_deductible_summary(day(-1), day(1))
            .await
            .expect("generate_deductible_summary");
        assert_eq!(result.deductible_minor, 10_000);
        assert_eq!(result.non_deductible_minor, 5_000);
    }

    #[tokio::test]
    async fn tax_itc_summary_splits_tax_paid_from_itc_eligible() {
        // calculation-engine.md §7 Vector 3: itc_eligible=1 tax 1800,
        // itc_eligible=0 tax 900 -> tax-paid total 2700, ITC-eligible 1800.
        let app = setup("tax_itc_summary").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Vector3 Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        app.expenses
            .create_expense(ExpenseInput {
                date: day(0),
                amount: MinorUnits(10_000),
                tax_amount: MinorUnits(1_800),
                itc_eligible: true,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: category.id,
            })
            .await
            .unwrap();
        app.expenses
            .create_expense(ExpenseInput {
                date: day(0),
                amount: MinorUnits(5_000),
                tax_amount: MinorUnits(900),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: None,
                category_id: category.id,
            })
            .await
            .unwrap();

        let result = app
            .reports
            .generate_tax_itc_summary(day(-1), day(1))
            .await
            .expect("generate_tax_itc_summary");
        assert_eq!(result.tax_paid_minor, 2_700);
        assert_eq!(result.itc_eligible_minor, 1_800);
    }

    #[tokio::test]
    async fn top_vendors_groups_by_name_snapshot_not_by_live_vendor_id() {
        // calculation-engine.md §7 Vector 4: a vendor renamed after 2 of its
        // 3 expenses were recorded must show two ranked rows, one per name
        // snapshot, each with its own partial total — never merged.
        let app = setup("top_vendors").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Vector4 Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        let vendor = app
            .vendors
            .create_vendor(VendorFields {
                name: "Vendor Original Name".into(),
                contact: None,
                notes: None,
            })
            .await
            .unwrap();

        for amount in [10_000, 20_000] {
            app.expenses
                .create_expense(ExpenseInput {
                    date: day(0),
                    amount: MinorUnits(amount),
                    tax_amount: MinorUnits(0),
                    itc_eligible: false,
                    deductible: true,
                    payment_method: "Cash".into(),
                    notes: None,
                    vendor_id: Some(vendor.id),
                    category_id: category.id,
                })
                .await
                .unwrap();
        }

        app.vendors
            .update_vendor(
                vendor.id,
                VendorFields {
                    name: "Vendor Renamed".into(),
                    contact: None,
                    notes: None,
                },
            )
            .await
            .unwrap();

        app.expenses
            .create_expense(ExpenseInput {
                date: day(0),
                amount: MinorUnits(5_000),
                tax_amount: MinorUnits(0),
                itc_eligible: false,
                deductible: true,
                payment_method: "Cash".into(),
                notes: None,
                vendor_id: Some(vendor.id),
                category_id: category.id,
            })
            .await
            .unwrap();

        let result = app
            .reports
            .generate_top_vendors(day(-1), day(1))
            .await
            .expect("generate_top_vendors");

        assert_eq!(result.rows.len(), 2, "two rows, one per name snapshot");
        let original_row = result
            .rows
            .iter()
            .find(|r| r.vendor_name_snapshot == "Vendor Original Name")
            .expect("row for the old snapshot");
        assert_eq!(original_row.total_minor, 30_000);
        let renamed_row = result
            .rows
            .iter()
            .find(|r| r.vendor_name_snapshot == "Vendor Renamed")
            .expect("row for the new snapshot");
        assert_eq!(renamed_row.total_minor, 5_000);
    }

    #[tokio::test]
    async fn period_summary_groups_by_calendar_month() {
        let app = setup("period_summary").await;
        let category = app
            .categories
            .create_category(CategoryFields {
                name: "Period Category".into(),
                default_deductible: true,
            })
            .await
            .unwrap();
        app.expenses
            .create_expense(ExpenseInput {
                date: NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
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
        app.expenses
            .create_expense(ExpenseInput {
                date: NaiveDate::from_ymd_opt(2026, 2, 5).unwrap(),
                amount: MinorUnits(2_000),
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

        let result = app
            .reports
            .generate_period_summary(
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            )
            .await
            .expect("generate_period_summary");
        assert_eq!(result.total_minor, 3_000);
        assert_eq!(result.rows.len(), 2);
        let jan = result.rows.iter().find(|r| r.period == "2026-01").unwrap();
        assert_eq!(jan.total_minor, 1_000);
        let feb = result.rows.iter().find(|r| r.period == "2026-02").unwrap();
        assert_eq!(feb.total_minor, 2_000);
    }
}
