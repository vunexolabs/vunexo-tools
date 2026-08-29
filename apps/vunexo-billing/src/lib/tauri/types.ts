// Mirrors the Rust domain/application types exactly, field-for-field,
// including their literal (snake_case) JSON field names — see
// src-tauri/src/domain/business.rs and domain/customer.rs.

export interface Business {
  name: string;
  logo_path: string | null;
  address: string | null;
  phone: string | null;
  email: string | null;
  gstin: string | null;
  bank_details: string | null;
  upi_id: string | null;
}

export type CustomerStatus = "ACTIVE" | "ARCHIVED";

export interface Customer {
  id: number;
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  gstin: string | null;
  status: CustomerStatus;
}

export interface CustomerFields {
  name: string;
  phone: string | null;
  email: string | null;
  address: string | null;
  gstin: string | null;
}

export interface CustomerListItem extends Customer {
  has_invoices: boolean;
}

export interface CustomerFilter {
  include_archived: boolean;
}

export type ProductStatus = "ACTIVE" | "ARCHIVED";

export interface Product {
  id: number;
  name: string;
  sku: string | null;
  description: string | null;
  unit: string;
  price_minor: number;
  tax_rate_id: number | null;
  hsn_sac_code: string | null;
  status: ProductStatus;
}

export interface ProductFields {
  name: string;
  sku: string | null;
  description: string | null;
  unit: string;
  price_minor: number;
  tax_rate_id: number | null;
  hsn_sac_code: string | null;
}

export interface ProductListItem extends Product {
  has_invoices: boolean;
}

export interface ProductFilter {
  include_archived: boolean;
}

/**
 * Parses a user-typed rupee amount (e.g. "1234.5") into minor units (paise)
 * using integer string arithmetic only — no floating point, per the money
 * rules in .ai/product.md, even for this simple a conversion.
 */
export function parseRupeesToMinor(input: string): number {
  const trimmed = input.trim();
  if (trimmed === "" || trimmed === "-") return 0;
  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [wholePartRaw, fracPartRaw = ""] = unsigned.split(".");
  const wholePart = wholePartRaw === "" ? "0" : wholePartRaw;
  const fracPart = (fracPartRaw + "00").slice(0, 2);
  const minor = parseInt(wholePart, 10) * 100 + parseInt(fracPart, 10);
  return negative ? -minor : minor;
}

export function formatMinorAsRupees(minor: number): string {
  const negative = minor < 0;
  const abs = Math.abs(minor);
  const whole = Math.floor(abs / 100);
  const frac = abs % 100;
  return `${negative ? "-" : ""}${whole}.${frac.toString().padStart(2, "0")}`;
}

/** Same integer-string-only approach as parseRupeesToMinor, scaled to 3 decimal places (calculation-engine.md §1's quantity_thousandths). */
export function parseQuantityToThousandths(input: string): number {
  const trimmed = input.trim();
  if (trimmed === "") return 0;
  const [wholePartRaw, fracPartRaw = ""] = trimmed.split(".");
  const wholePart = wholePartRaw === "" ? "0" : wholePartRaw;
  const fracPart = (fracPartRaw + "000").slice(0, 3);
  return parseInt(wholePart, 10) * 1000 + parseInt(fracPart, 10);
}

export function formatThousandthsAsQuantity(thousandths: number): string {
  const whole = Math.floor(thousandths / 1000);
  const frac = thousandths % 1000;
  return frac === 0 ? `${whole}` : `${whole}.${frac.toString().padStart(3, "0")}`;
}

/** Same approach, scaled to basis points (rate x 100 — calculation-engine.md §5). */
export function parsePercentToBasisPoints(input: string): number {
  const trimmed = input.trim();
  if (trimmed === "") return 0;
  const [wholePartRaw, fracPartRaw = ""] = trimmed.split(".");
  const wholePart = wholePartRaw === "" ? "0" : wholePartRaw;
  const fracPart = (fracPartRaw + "00").slice(0, 2);
  return parseInt(wholePart, 10) * 100 + parseInt(fracPart, 10);
}

export function formatBasisPointsAsPercent(basisPoints: number): string {
  const whole = Math.floor(basisPoints / 100);
  const frac = basisPoints % 100;
  return frac === 0 ? `${whole}` : `${whole}.${frac.toString().padStart(2, "0")}`;
}

export type DiscountType = "AMOUNT" | "PERCENTAGE";

export interface Settings {
  country_code: string;
  currency_code: string;
  date_format: string;
  invoice_number_format: string;
  default_due_days: number;
  default_tax_rate_id: number | null;
}

export interface SettingsFields {
  country_code: string;
  currency_code: string;
  date_format: string;
  invoice_number_format: string;
  default_due_days: number;
  default_tax_rate_id: number | null;
}

export type InvoiceStatus = "DRAFT" | "ISSUED" | "PARTIALLY_PAID" | "PAID" | "CANCELLED";

export interface InvoiceLineItem {
  id: number;
  product_id: number | null;
  description: string;
  unit: string;
  quantity_thousandths: number;
  unit_price_minor: number;
  line_discount_type: DiscountType | null;
  line_discount_value: number | null;
  tax_rate_id: number | null;
  tax_rate_basis_points: number;
  line_subtotal_minor: number;
  line_discount_amount_minor: number;
  invoice_discount_amount_minor: number;
  taxable_amount_minor: number;
  line_tax_minor: number;
  line_total_minor: number;
  sort_order: number;
}

/** The raw, pre-calculation shape sent to create_draft_invoice/update_draft_invoice. */
export interface LineItemInput {
  product_id: number | null;
  description: string;
  unit: string;
  quantity_thousandths: number;
  unit_price_minor: number;
  line_discount_type: DiscountType | null;
  line_discount_value: number | null;
  tax_rate_id: number | null;
  tax_rate_basis_points: number;
}

export interface Invoice {
  id: number;
  invoice_number: string | null;
  invoice_number_is_custom: boolean;
  status: InvoiceStatus;
  customer_id: number | null;
  customer_snapshot_name: string | null;
  customer_snapshot_phone: string | null;
  customer_snapshot_email: string | null;
  customer_snapshot_address: string | null;
  customer_snapshot_gstin: string | null;
  business_snapshot_name: string | null;
  business_snapshot_address: string | null;
  business_snapshot_gstin: string | null;
  business_snapshot_phone: string | null;
  business_snapshot_email: string | null;
  business_snapshot_bank_details: string | null;
  business_snapshot_upi_id: string | null;
  business_snapshot_logo_path: string | null;
  is_interstate: boolean;
  invoice_date: string;
  due_date: string | null;
  notes: string | null;
  terms: string | null;
  discount_type: DiscountType | null;
  discount_value: number | null;
  subtotal_minor: number;
  discount_amount_minor: number;
  tax_amount_minor: number;
  total_minor: number;
  issued_at: string | null;
  cancelled_at: string | null;
  cancel_reason: string | null;
}

export interface InvoiceWithLineItems extends Invoice {
  line_items: InvoiceLineItem[];
}

export interface InvoiceSummary {
  id: number;
  invoice_number: string | null;
  status: InvoiceStatus;
  customer_name: string | null;
  invoice_date: string;
  due_date: string | null;
  total_minor: number;
  amount_paid_minor: number;
  is_overdue: boolean;
}

export interface DraftInvoiceInput {
  customer_id: number | null;
  invoice_date: string;
  due_date: string | null;
  notes: string | null;
  terms: string | null;
  is_interstate: boolean;
  discount_type: DiscountType | null;
  discount_value: number | null;
  line_items: LineItemInput[];
}

export interface InvoiceFilter {
  status: InvoiceStatus | null;
}

/** application-architecture.md §6 — the shape every command error rejects with. */
export interface ApplicationErrorPayload {
  kind: "not_found" | "validation" | "conflict" | "infrastructure";
  message: string;
}

export function isApplicationError(err: unknown): err is ApplicationErrorPayload {
  return (
    typeof err === "object" &&
    err !== null &&
    "kind" in err &&
    "message" in err &&
    typeof (err as ApplicationErrorPayload).message === "string"
  );
}

/** ui-ux.md §3's error-mapping rule: never show an infrastructure message verbatim. */
export function errorMessage(err: unknown): string {
  if (isApplicationError(err)) {
    return err.kind === "infrastructure" ? "Something went wrong. Your data is safe." : err.message;
  }
  return "Something went wrong. Your data is safe.";
}
