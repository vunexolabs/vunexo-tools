// Typed call signatures for Tauri commands. Add one entry here per command
// exposed in src-tauri/src/commands/mod.rs.
import { callCommand } from "./client";
import type {
  Business,
  Customer,
  CustomerFields,
  CustomerFilter,
  CustomerListItem,
  DashboardMetrics,
  DraftInvoiceInput,
  InvoiceFilter,
  InvoiceSummary,
  InvoiceWithLineItems,
  NewPayment,
  Payment,
  PaymentFields,
  Product,
  ProductFields,
  ProductFilter,
  ProductListItem,
  Settings,
  SettingsFields,
  TaxRate,
  TaxRateFields,
} from "./types";

/** Round 1 technical spike: proves the React -> Tauri -> Rust round trip. */
export function greet(name: string): Promise<string> {
  return callCommand<string>("greet", { name });
}

export function createBusiness(business: Business): Promise<Business> {
  return callCommand<Business>("create_business", { business });
}

export function getBusiness(): Promise<Business | null> {
  return callCommand<Business | null>("get_business");
}

export function updateBusiness(business: Business): Promise<Business> {
  return callCommand<Business>("update_business", { business });
}

export function createCustomer(fields: CustomerFields): Promise<Customer> {
  return callCommand<Customer>("create_customer", { fields });
}

export function updateCustomer(id: number, fields: CustomerFields): Promise<Customer> {
  return callCommand<Customer>("update_customer", { id, fields });
}

export function archiveCustomer(id: number): Promise<void> {
  return callCommand<void>("archive_customer", { id });
}

export function restoreCustomer(id: number): Promise<void> {
  return callCommand<void>("restore_customer", { id });
}

export function deleteCustomer(id: number): Promise<void> {
  return callCommand<void>("delete_customer", { id });
}

export function getCustomer(id: number): Promise<Customer> {
  return callCommand<Customer>("get_customer", { id });
}

export function listCustomers(filter: CustomerFilter): Promise<CustomerListItem[]> {
  return callCommand<CustomerListItem[]>("list_customers", { filter });
}

export function createProduct(fields: ProductFields): Promise<Product> {
  return callCommand<Product>("create_product", { fields });
}

export function updateProduct(id: number, fields: ProductFields): Promise<Product> {
  return callCommand<Product>("update_product", { id, fields });
}

export function archiveProduct(id: number): Promise<void> {
  return callCommand<void>("archive_product", { id });
}

export function restoreProduct(id: number): Promise<void> {
  return callCommand<void>("restore_product", { id });
}

export function deleteProduct(id: number): Promise<void> {
  return callCommand<void>("delete_product", { id });
}

export function getProduct(id: number): Promise<Product> {
  return callCommand<Product>("get_product", { id });
}

export function listProducts(filter: ProductFilter): Promise<ProductListItem[]> {
  return callCommand<ProductListItem[]>("list_products", { filter });
}

export function getSettings(): Promise<Settings> {
  return callCommand<Settings>("get_settings");
}

export function updateSettings(fields: SettingsFields): Promise<Settings> {
  return callCommand<Settings>("update_settings", { fields });
}

export function previewNextInvoiceNumber(): Promise<string> {
  return callCommand<string>("preview_next_invoice_number");
}

export function createDraftInvoice(input: DraftInvoiceInput): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("create_draft_invoice", { input });
}

export function updateDraftInvoice(id: number, input: DraftInvoiceInput): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("update_draft_invoice", { id, input });
}

export function issueInvoice(id: number, customNumber: string | null): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("issue_invoice", { id, customNumber });
}

export function editIssuedInvoice(id: number, input: DraftInvoiceInput): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("edit_issued_invoice", { id, input });
}

export function cancelInvoice(id: number, reason: string | null): Promise<void> {
  return callCommand<void>("cancel_invoice", { id, reason });
}

export function deleteDraftInvoice(id: number): Promise<void> {
  return callCommand<void>("delete_draft_invoice", { id });
}

export function duplicateInvoice(id: number): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("duplicate_invoice", { id });
}

export function getInvoice(id: number): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("get_invoice", { id });
}

export function listInvoices(filter: InvoiceFilter): Promise<InvoiceSummary[]> {
  return callCommand<InvoiceSummary[]>("list_invoices", { filter });
}

export function recordPayment(payment: NewPayment): Promise<Payment> {
  return callCommand<Payment>("record_payment", { payment });
}

export function updatePayment(id: number, fields: PaymentFields): Promise<Payment> {
  return callCommand<Payment>("update_payment", { id, fields });
}

export function deletePayment(id: number): Promise<void> {
  return callCommand<void>("delete_payment", { id });
}

export function listPaymentsForInvoice(invoiceId: number): Promise<Payment[]> {
  return callCommand<Payment[]>("list_payments_for_invoice", { invoiceId });
}

export function createTaxRate(fields: TaxRateFields): Promise<TaxRate> {
  return callCommand<TaxRate>("create_tax_rate", { fields });
}

export function updateTaxRate(id: number, fields: TaxRateFields): Promise<TaxRate> {
  return callCommand<TaxRate>("update_tax_rate", { id, fields });
}

export function listTaxRates(): Promise<TaxRate[]> {
  return callCommand<TaxRate[]>("list_tax_rates");
}

export function getDashboardMetrics(): Promise<DashboardMetrics> {
  return callCommand<DashboardMetrics>("get_dashboard_metrics");
}
