// Typed call signatures for Tauri commands. Add one entry here per command
// exposed in src-tauri/src/commands/mod.rs.
import { callCommand } from "./client";
import type {
  BackupMetadata,
  Business,
  Customer,
  CustomerFields,
  CustomerFilter,
  CustomerListItem,
  DashboardMetrics,
  DraftInvoiceInput,
  DraftQuoteInput,
  ExportEntity,
  InvoiceFilter,
  InvoiceSummary,
  InvoiceWithLineItems,
  LogoProbe,
  NewPayment,
  Payment,
  PaymentFields,
  Product,
  ProductFields,
  ProductFilter,
  ProductListItem,
  QuoteFilter,
  QuoteSummary,
  QuoteWithLineItems,
  RenderedInvoicePdf,
  SalesGrouping,
  SalesSummaryResult,
  Settings,
  SettingsFields,
  StatementResult,
  TaxRate,
  TaxRateFields,
  TaxSummaryResult,
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

export function previewNextQuoteNumber(): Promise<string> {
  return callCommand<string>("preview_next_quote_number");
}

export function createDraftQuote(input: DraftQuoteInput): Promise<QuoteWithLineItems> {
  return callCommand<QuoteWithLineItems>("create_draft_quote", { input });
}

export function updateDraftQuote(id: number, input: DraftQuoteInput): Promise<QuoteWithLineItems> {
  return callCommand<QuoteWithLineItems>("update_draft_quote", { id, input });
}

export function issueQuote(id: number): Promise<QuoteWithLineItems> {
  return callCommand<QuoteWithLineItems>("issue_quote", { id });
}

export function acceptQuote(id: number): Promise<void> {
  return callCommand<void>("accept_quote", { id });
}

export function declineQuote(id: number): Promise<void> {
  return callCommand<void>("decline_quote", { id });
}

export function cancelQuote(id: number, reason: string | null): Promise<void> {
  return callCommand<void>("cancel_quote", { id, reason });
}

/** application-architecture-v2.md §4c — returns the resulting Draft invoice. */
export function convertQuoteToInvoice(id: number): Promise<InvoiceWithLineItems> {
  return callCommand<InvoiceWithLineItems>("convert_quote_to_invoice", { id });
}

export function duplicateQuote(id: number): Promise<QuoteWithLineItems> {
  return callCommand<QuoteWithLineItems>("duplicate_quote", { id });
}

export function deleteDraftQuote(id: number): Promise<void> {
  return callCommand<void>("delete_draft_quote", { id });
}

export function getQuote(id: number): Promise<QuoteWithLineItems> {
  return callCommand<QuoteWithLineItems>("get_quote", { id });
}

export function listQuotes(filter: QuoteFilter): Promise<QuoteSummary[]> {
  return callCommand<QuoteSummary[]>("list_quotes", { filter });
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

/**
 * Renders an invoice at any status — Preview is reachable from a draft
 * (user-flows.md §5 step 5), so this is not issued-only. The bytes come back
 * base64-encoded because Tauri would otherwise serialize them as a JSON array
 * of numbers; `invoicePdfBlob` turns them back into something the preview
 * pane can display.
 */
export function renderInvoicePdf(id: number): Promise<RenderedInvoicePdf> {
  return callCommand<RenderedInvoicePdf>("render_invoice_pdf", { id });
}

/**
 * Whether the business logo at `path` can actually be printed. The PDF
 * renderer skips a logo it can't load rather than failing the invoice, so
 * Settings asks this instead of letting the user find out from a logo-less
 * invoice.
 */
export function probeBusinessLogo(path: string): Promise<LogoProbe> {
  return callCommand<LogoProbe>("probe_business_logo", { path });
}

/** Writes the invoice PDF to a path already chosen in the OS save dialog. */
export function saveInvoicePdf(id: number, path: string): Promise<void> {
  return callCommand<void>("save_invoice_pdf", { id, path });
}

export function suggestedBackupFileName(): Promise<string> {
  return callCommand<string>("suggested_backup_file_name");
}

export function backupDatabase(path: string): Promise<void> {
  return callCommand<void>("backup_database", { path });
}

/**
 * Reads a `.vbx`'s metadata without unpacking it, so the confirmation can say
 * what is about to replace the user's data — and so an archive this build
 * can't read is refused before anything is touched.
 */
export function inspectBackup(path: string): Promise<BackupMetadata> {
  return callCommand<BackupMetadata>("inspect_backup", { path });
}

/**
 * Replaces all local data and **restarts the app**, so this never resolves on
 * success. Every repository holds a pool the restore has closed; carrying on
 * would mean serving a database nothing can read.
 */
export function restoreBackup(path: string): Promise<void> {
  return callCommand<void>("restore_backup", { path });
}

export function suggestedExportFileName(entity: ExportEntity): Promise<string> {
  return callCommand<string>("suggested_export_file_name", { entity });
}

export function exportData(entity: ExportEntity, path: string): Promise<void> {
  return callCommand<void>("export_data", { entity, path });
}

export function generateCustomerStatement(
  customerId: number,
  rangeStart: string,
  rangeEnd: string,
): Promise<StatementResult> {
  return callCommand<StatementResult>("generate_customer_statement", {
    customerId,
    rangeStart,
    rangeEnd,
  });
}

export function generateSalesReport(
  rangeStart: string,
  rangeEnd: string,
  groupBy: SalesGrouping,
): Promise<SalesSummaryResult> {
  return callCommand<SalesSummaryResult>("generate_sales_report", { rangeStart, rangeEnd, groupBy });
}

export function generateTaxSummaryReport(rangeStart: string, rangeEnd: string): Promise<TaxSummaryResult> {
  return callCommand<TaxSummaryResult>("generate_tax_summary_report", { rangeStart, rangeEnd });
}

export function generateReminderMessage(invoiceId: number): Promise<string> {
  return callCommand<string>("generate_reminder_message", { invoiceId });
}

/** Writes CSV/JSON built client-side (Statement/Reports exports) to a path already chosen in the OS save dialog. */
export function writeExportFile(path: string, contents: string): Promise<void> {
  return callCommand<void>("write_export_file", { path, contents });
}
