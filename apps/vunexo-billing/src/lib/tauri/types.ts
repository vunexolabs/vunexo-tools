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
