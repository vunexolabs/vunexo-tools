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
 * Parses a user-typed amount (e.g. "1234.5") into minor units using integer
 * string arithmetic only — no floating point, per the money rules in
 * .ai/product.md — scaled to `decimals` places (a currency's ISO 4217 minor
 * -unit exponent, `lib/currency.ts`) rather than hardcoded to 2 (paise).
 * Screens should generally go through `useCurrency()`'s `parseToMinor`
 * instead of calling this directly, so the active currency's scale is
 * always applied consistently.
 */
export function parseAmountToMinor(input: string, decimals: number): number {
  const trimmed = input.trim();
  if (trimmed === "" || trimmed === "-") return 0;
  const negative = trimmed.startsWith("-");
  const unsigned = negative ? trimmed.slice(1) : trimmed;
  const [wholePartRaw, fracPartRaw = ""] = unsigned.split(".");
  const wholePart = wholePartRaw === "" ? "0" : wholePartRaw;
  const scale = 10 ** decimals;
  const fracPart = decimals === 0 ? "" : (fracPartRaw + "0".repeat(decimals)).slice(0, decimals);
  const minor = parseInt(wholePart, 10) * scale + (fracPart === "" ? 0 : parseInt(fracPart, 10));
  return negative ? -minor : minor;
}

/** See `parseAmountToMinor` — prefer `useCurrency()`'s `formatMinor` at call sites. */
export function formatMinorAsAmount(minor: number, decimals: number): string {
  const negative = minor < 0;
  const abs = Math.abs(minor);
  const scale = 10 ** decimals;
  const whole = Math.floor(abs / scale);
  if (decimals === 0) return `${negative ? "-" : ""}${whole}`;
  const frac = abs % scale;
  return `${negative ? "-" : ""}${whole}.${frac.toString().padStart(decimals, "0")}`;
}

/**
 * database-schema.md §6 / calculation-engine.md §5 — CGST/SGST vs IGST is a
 * *display* split of the already-final, backend-computed `tax_amount_minor`,
 * never a separately stored or separately calculated figure. This mirrors
 * `domain::calculation::split_gst` in the Rust backend exactly (integer
 * halving, no rounding judgment call), so the one narrow exception to "the
 * frontend never calculates financial totals" is a derivation with only one
 * possible correct answer, not a place the two sides of the app could
 * disagree.
 */
export function splitGst(taxAmountMinor: number, isInterstate: boolean): { cgst: number; sgst: number; igst: number } {
  if (isInterstate) return { igst: taxAmountMinor, cgst: 0, sgst: 0 };
  const cgst = Math.floor(taxAmountMinor / 2);
  return { igst: 0, cgst, sgst: taxAmountMinor - cgst };
}

/** Same integer-string-only approach as parseAmountToMinor, scaled to 3 decimal places (calculation-engine.md §1's quantity_thousandths). */
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

export interface TaxRate {
  id: number;
  name: string;
  rate_basis_points: number;
}

export interface TaxRateFields {
  name: string;
  rate_basis_points: number;
}

export type InvoiceStatus = "DRAFT" | "ISSUED" | "PARTIALLY_PAID" | "PAID" | "CANCELLED";

/** ui-ux.md §6 — the three CSVs and the one JSON "everything" export. */
export type ExportEntity = "CUSTOMERS" | "PRODUCTS" | "INVOICES" | "ALL";

/** The `metadata.json` block inside a `.vbx` (database-schema.md §9). */
export interface BackupMetadata {
  format_version: number;
  app_version: string;
  created_at: string;
  platform: string;
}

/** What `probe_business_logo` returns — whether the chosen logo can actually be printed. */
export type LogoProbe =
  | { status: "OK"; width_px: number; height_px: number }
  | { status: "NOT_FOUND" }
  | { status: "UNREADABLE" };

/** What `render_invoice_pdf` returns — see the command's own note on the base64. */
export interface RenderedInvoicePdf {
  file_name: string;
  bytes_base64: string;
}

/**
 * Turns a rendered invoice into an object URL the preview pane can point an
 * `<iframe>` at. The caller owns the URL and must `URL.revokeObjectURL` it,
 * or every preview leaks a copy of the document.
 */
export function invoicePdfObjectUrl(rendered: RenderedInvoicePdf): string {
  const binary = atob(rendered.bytes_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
}

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

export interface OverdueSummary {
  count: number;
  total_minor: number;
}

export interface DashboardMetrics {
  today_sales_minor: number;
  month_sales_minor: number;
  outstanding_total_minor: number;
  paid_total_minor: number;
  overdue: OverdueSummary;
  recent_invoices: InvoiceSummary[];
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

export type PaymentMethod = "CASH" | "BANK_TRANSFER" | "UPI" | "CHEQUE" | "OTHER";

export interface Payment {
  id: number;
  invoice_id: number;
  amount_minor: number;
  method: PaymentMethod;
  paid_on: string;
  reference: string | null;
  created_at: string;
  updated_at: string;
}

export interface NewPayment {
  invoice_id: number;
  amount_minor: number;
  method: PaymentMethod;
  paid_on: string;
  reference: string | null;
}

export interface PaymentFields {
  amount_minor: number;
  method: PaymentMethod;
  paid_on: string;
  reference: string | null;
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
