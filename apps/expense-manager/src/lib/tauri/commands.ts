// One typed wrapper per Tauri command — application-architecture.md's
// "Tauri command surface" section is the source of truth for the set below
// (plus `suggestedBackupFileName`, a thin additive convenience not named in
// that list — see `src-tauri/src/commands/mod.rs`'s own comment on it).
import { callCommand } from "./client";
import type {
  Business,
  Category,
  CategoryFields,
  CategoryListItem,
  CategorySummaryResult,
  DashboardMetrics,
  DeductibleSummaryResult,
  Expense,
  ExpenseFilter,
  ExpenseInput,
  PeriodSummaryResult,
  TaxItcSummaryResult,
  TopVendorsResult,
  Vendor,
  VendorFields,
  VendorListItem,
} from "./types";

export function createBusiness(business: Business) {
  return callCommand<Business>("create_business", { business });
}
export function updateBusiness(business: Business) {
  return callCommand<Business>("update_business", { business });
}
export function getBusiness() {
  return callCommand<Business | null>("get_business");
}

export function createVendor(fields: VendorFields) {
  return callCommand<Vendor>("create_vendor", { fields });
}
export function updateVendor(id: number, fields: VendorFields) {
  return callCommand<Vendor>("update_vendor", { id, fields });
}
export function deleteVendor(id: number) {
  return callCommand<void>("delete_vendor", { id });
}
export function listVendors() {
  return callCommand<VendorListItem[]>("list_vendors");
}

export function createCategory(fields: CategoryFields) {
  return callCommand<Category>("create_category", { fields });
}
export function updateCategory(id: number, fields: CategoryFields) {
  return callCommand<Category>("update_category", { id, fields });
}
export function deleteCategory(id: number) {
  return callCommand<void>("delete_category", { id });
}
export function listCategories() {
  return callCommand<CategoryListItem[]>("list_categories");
}

export function createExpense(input: ExpenseInput) {
  return callCommand<Expense>("create_expense", { input });
}
export function updateExpense(id: number, input: ExpenseInput) {
  return callCommand<Expense>("update_expense", { id, input });
}
export function deleteExpense(id: number) {
  return callCommand<void>("delete_expense", { id });
}
export function listExpenses(filter: ExpenseFilter) {
  return callCommand<Expense[]>("list_expenses", { filter });
}

export function attachReceipt(id: number, path: string) {
  return callCommand<Expense>("attach_receipt", { id, path });
}
export function replaceReceipt(id: number, path: string) {
  return callCommand<Expense>("replace_receipt", { id, path });
}
export function removeReceipt(id: number) {
  return callCommand<Expense>("remove_receipt", { id });
}

export function getDashboardMetrics() {
  return callCommand<DashboardMetrics>("get_dashboard_metrics");
}

export function generateCategorySummary(rangeStart: string, rangeEnd: string) {
  return callCommand<CategorySummaryResult>("generate_category_summary", {
    rangeStart,
    rangeEnd,
  });
}
export function generatePeriodSummary(rangeStart: string, rangeEnd: string) {
  return callCommand<PeriodSummaryResult>("generate_period_summary", {
    rangeStart,
    rangeEnd,
  });
}
export function generateDeductibleSummary(rangeStart: string, rangeEnd: string) {
  return callCommand<DeductibleSummaryResult>("generate_deductible_summary", {
    rangeStart,
    rangeEnd,
  });
}
export function generateTaxItcSummary(rangeStart: string, rangeEnd: string) {
  return callCommand<TaxItcSummaryResult>("generate_tax_itc_summary", {
    rangeStart,
    rangeEnd,
  });
}
export function generateTopVendors(rangeStart: string, rangeEnd: string) {
  return callCommand<TopVendorsResult>("generate_top_vendors", {
    rangeStart,
    rangeEnd,
  });
}

export function writeExportFile(path: string, contents: string) {
  return callCommand<void>("write_export_file", { path, contents });
}

export function suggestedBackupFileName() {
  return callCommand<string>("suggested_backup_file_name");
}
export function backupData(path: string) {
  return callCommand<void>("backup_data", { path });
}
export function restoreBackup(path: string) {
  return callCommand<void>("restore_backup", { path });
}
