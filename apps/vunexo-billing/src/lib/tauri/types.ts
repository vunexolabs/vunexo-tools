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
