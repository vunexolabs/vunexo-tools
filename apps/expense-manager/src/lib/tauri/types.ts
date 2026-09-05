// Mirrors the Rust domain/application types exactly, field-for-field,
// including their literal (snake_case) JSON field names — see
// src-tauri/src/domain/*.rs. Money fields (`amount`, `tax_amount`,
// `*_minor`) are always plain integer minor units — never a float anywhere
// on this boundary (Round 1's locked rule).

export interface Business {
  name: string;
  address: string | null;
  tax_info: string | null;
  currency_symbol: string;
}

export interface Vendor {
  id: number;
  name: string;
  contact: string | null;
  notes: string | null;
}

export interface VendorFields {
  name: string;
  contact: string | null;
  notes: string | null;
}

export interface VendorListItem extends Vendor {
  has_expenses: boolean;
}

export interface Category {
  id: number;
  name: string;
  default_deductible: boolean;
}

export interface CategoryFields {
  name: string;
  default_deductible: boolean;
}

export interface CategoryListItem extends Category {
  has_expenses: boolean;
}

export interface Expense {
  id: number;
  date: string;
  amount: number;
  tax_amount: number;
  itc_eligible: boolean;
  deductible: boolean;
  payment_method: string;
  notes: string | null;
  receipt_path: string | null;
  vendor_id: number | null;
  vendor_name_snapshot: string | null;
  category_id: number;
  category_name_snapshot: string;
  created_at: string;
  updated_at: string;
}

/** What the Expense Editor submits, for both create and update. */
export interface ExpenseInput {
  date: string;
  amount: number;
  tax_amount: number;
  itc_eligible: boolean;
  deductible: boolean;
  payment_method: string;
  notes: string | null;
  vendor_id: number | null;
  category_id: number;
}

/** Every field optional and combinable — ui-ux.md §5's Expenses List filters. */
export interface ExpenseFilter {
  category_id?: number | null;
  vendor_id?: number | null;
  date_from?: string | null;
  date_to?: string | null;
}

export interface CategoryBreakdownRow {
  category_id: number;
  category_name: string;
  total_minor: number;
}

export interface DashboardMetrics {
  period_total_minor: number;
  category_breakdown: CategoryBreakdownRow[];
  recent_expenses: Expense[];
}

export interface CategorySummaryRow {
  category_id: number;
  category_name: string;
  total_minor: number;
}

export interface CategorySummaryResult {
  total_minor: number;
  rows: CategorySummaryRow[];
}

export interface PeriodSummaryRow {
  period: string;
  total_minor: number;
}

export interface PeriodSummaryResult {
  total_minor: number;
  rows: PeriodSummaryRow[];
}

export interface DeductibleSummaryResult {
  deductible_minor: number;
  non_deductible_minor: number;
}

export interface TaxItcSummaryResult {
  tax_paid_minor: number;
  itc_eligible_minor: number;
}

export interface TopVendorRow {
  vendor_name_snapshot: string;
  total_minor: number;
}

export interface TopVendorsResult {
  rows: TopVendorRow[];
}

export interface BackupMetadata {
  format_version: number;
  app_version: string;
  created_at: string;
  platform: string;
}

// --- Error shape (application-architecture.md's error-handling section) ---

export interface ApplicationErrorPayload {
  kind: "not_found" | "validation" | "infrastructure";
  message: string;
}

function isApplicationError(err: unknown): err is ApplicationErrorPayload {
  return (
    typeof err === "object" &&
    err !== null &&
    "kind" in err &&
    "message" in err &&
    typeof (err as { message: unknown }).message === "string"
  );
}

export function errorMessage(err: unknown): string {
  if (isApplicationError(err)) {
    return err.kind === "infrastructure" ? "Something went wrong. Your data is safe." : err.message;
  }
  return "Something went wrong. Your data is safe.";
}
