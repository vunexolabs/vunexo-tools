import { useEffect, useRef, useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { Modal } from "../../components/Modal";
import { SearchablePicker } from "../../components/SearchablePicker";
import { StatusBadge } from "../../components/StatusBadge";
import { CustomerForm } from "../customers/CustomerForm";
import { PaymentPanel } from "../payments/PaymentPanel";
import { ProductForm } from "../products/ProductForm";
import { ReminderModal } from "../reminders/ReminderModal";
import { InvoicePdfPreview } from "./InvoicePdfPreview";
import {
  cancelInvoice,
  createCustomer,
  createProduct,
  duplicateInvoice,
  editIssuedInvoice,
  getInvoice,
  getSettings,
  issueInvoice,
  listCustomers,
  listProducts,
  listTaxRates,
  previewNextInvoiceNumber,
  updateDraftInvoice,
} from "../../lib/tauri/commands";
import { useBusiness } from "../../hooks/useBusiness";
import { useCurrency } from "../../hooks/useCurrency";
import { useInvoicePdf } from "../../hooks/useInvoicePdf";
import { useTaxRegimeFields } from "../../hooks/useTaxRegimeFields";
import {
  formatThousandthsAsQuantity,
  formatBasisPointsAsPercent,
  parsePercentToBasisPoints,
  parseQuantityToThousandths,
  presentVat,
  splitGst,
  type CustomerListItem,
  type DraftInvoiceInput,
  type InvoiceWithLineItems,
  type ProductListItem,
  type TaxRate,
} from "../../lib/tauri/types";

/** Resolves a tax rate id against the loaded list, falling back to "no tax rate" if it's gone. */
function resolveTaxRate(taxRates: TaxRate[], taxRateId: number | null): { taxRateId: number | null; taxStr: string } {
  const rate = taxRateId === null ? undefined : taxRates.find((r) => r.id === taxRateId);
  return rate ? { taxRateId: rate.id, taxStr: formatBasisPointsAsPercent(rate.rate_basis_points) } : { taxRateId: null, taxStr: "0" };
}

/**
 * ui-ux.md §4 — one component for create/edit-draft/view-issued, not three
 * screens; the fields/footer just change by `status`. `Issued`/
 * `PartiallyPaid`/`Paid` are all fully editable (user-flows.md's "Editing
 * an issued invoice" rule) via `edit_issued_invoice`, which re-snapshots
 * customer/business fresh at every save; only `CANCELLED` is read-only.
 *
 * Totals are still never computed client-side (application-architecture.md
 * §4a) — every line/discount edit debounces a real save call and re-renders
 * whatever the backend returns, so it *feels* live without moving the
 * arithmetic into React.
 */

interface EditableLine {
  key: string;
  product_id: number | null;
  description: string;
  unit: string;
  quantityStr: string;
  priceStr: string;
  taxRateId: number | null;
  taxStr: string;
  lineDiscountIsPercentage: boolean;
  lineDiscountStr: string;
}

let keyCounter = 0;
function newKey() {
  keyCounter += 1;
  return `line-${keyCounter}`;
}

// These three run outside the component (no hooks allowed), so the active
// currency's formatter is threaded through as a parameter rather than read
// via `useCurrency()` directly.
function toEditableLine(li: InvoiceWithLineItems["line_items"][number], key: string, formatMinor: (m: number) => string): EditableLine {
  return {
    key,
    product_id: li.product_id,
    description: li.description,
    unit: li.unit,
    quantityStr: formatThousandthsAsQuantity(li.quantity_thousandths),
    priceStr: formatMinor(li.unit_price_minor),
    taxRateId: li.tax_rate_id,
    taxStr: formatBasisPointsAsPercent(li.tax_rate_basis_points),
    lineDiscountIsPercentage: li.line_discount_type === "PERCENTAGE",
    lineDiscountStr:
      li.line_discount_value === null
        ? ""
        : li.line_discount_type === "PERCENTAGE"
          ? formatBasisPointsAsPercent(li.line_discount_value)
          : formatMinor(li.line_discount_value),
  };
}

function toEditableLines(invoice: InvoiceWithLineItems, formatMinor: (m: number) => string): EditableLine[] {
  return invoice.line_items.map((li) => toEditableLine(li, newKey(), formatMinor));
}

/**
 * Same shape as `toEditableLines`, but reuses each line's existing `key`
 * (by position — the calculation engine preserves input order) instead of
 * minting new ones. Used after an autosave round-trip so React doesn't
 * remount every row and steal focus mid-edit.
 */
function mergeEditableLines(prevLines: EditableLine[], invoice: InvoiceWithLineItems, formatMinor: (m: number) => string): EditableLine[] {
  return invoice.line_items.map((li, i) => toEditableLine(li, prevLines[i]?.key ?? newKey(), formatMinor));
}

const AUTOSAVE_DEBOUNCE_MS = 600;

export function InvoiceEditor({
  invoiceId,
  onBack,
  onOpenInvoice,
}: {
  invoiceId: number;
  onBack: () => void;
  onOpenInvoice: (id: number) => void;
}) {
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const { business } = useBusiness();
  const [invoice, setInvoice] = useState<InvoiceWithLineItems | null>(null);
  const [customers, setCustomers] = useState<CustomerListItem[]>([]);
  const [products, setProducts] = useState<ProductListItem[]>([]);
  const [taxRates, setTaxRates] = useState<TaxRate[]>([]);
  const [defaultTaxRateId, setDefaultTaxRateId] = useState<number | null>(null);
  const [numberPreview, setNumberPreview] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [saving, setSaving] = useState(false);
  const [autoSaving, setAutoSaving] = useState(false);
  const [showNewCustomerModal, setShowNewCustomerModal] = useState(false);
  const [newProductForLineKey, setNewProductForLineKey] = useState<string | null>(null);
  const [showCancelDialog, setShowCancelDialog] = useState(false);
  const [cancelReason, setCancelReason] = useState("");
  const [duplicating, setDuplicating] = useState(false);
  const [savedPdfPath, setSavedPdfPath] = useState<string | null>(null);
  const pdf = useInvoicePdf();
  const [showReminder, setShowReminder] = useState(false);

  const [customerId, setCustomerId] = useState<number | null>(null);
  const [invoiceDate, setInvoiceDate] = useState("");
  const [dueDate, setDueDate] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [terms, setTerms] = useState("");
  const [isInterstate, setIsInterstate] = useState(false);
  const [discountIsPercentage, setDiscountIsPercentage] = useState(false);
  const [discountStr, setDiscountStr] = useState("");
  const [lines, setLines] = useState<EditableLine[]>([]);
  const [useCustomNumber, setUseCustomNumber] = useState(false);
  const [customNumber, setCustomNumber] = useState("");

  // application-architecture-v2.md §4d: an issued invoice prints/shows its
  // own frozen `tax_regime_snapshot`; a Draft has none yet, so it reflects
  // the business's *current* regime instead — never `settings.country_code`
  // (ui-ux-v2.md §3's "one switch point" rule).
  const effectiveRegime = invoice?.tax_regime_snapshot ?? business?.tax_regime_code ?? "IN_GST";
  const taxFields = useTaxRegimeFields(effectiveRegime);

  useEffect(() => {
    Promise.all([
      getInvoice(invoiceId),
      listCustomers({ include_archived: false }),
      listProducts({ include_archived: false }),
      listTaxRates(),
      getSettings(),
    ])
      .then(([inv, custs, prods, rates, settings]) => {
        setInvoice(inv);
        setCustomers(custs);
        setProducts(prods);
        setTaxRates(rates);
        setDefaultTaxRateId(settings.default_tax_rate_id);
        setCustomerId(inv.customer_id);
        setInvoiceDate(inv.invoice_date);
        setDueDate(inv.due_date);
        setNotes(inv.notes ?? "");
        setTerms(inv.terms ?? "");
        setIsInterstate(inv.is_interstate);
        setDiscountIsPercentage(inv.discount_type === "PERCENTAGE");
        setDiscountStr(
          inv.discount_value === null
            ? ""
            : inv.discount_type === "PERCENTAGE"
              ? formatBasisPointsAsPercent(inv.discount_value)
              : formatMinor(inv.discount_value),
        );
        setLines(toEditableLines(inv, formatMinor));
        if (inv.status === "DRAFT") {
          previewNextInvoiceNumber().then(setNumberPreview).catch(() => setNumberPreview(null));
        }
      })
      .catch((err: unknown) => setError(err));
  }, [invoiceId, formatMinor]);

  // Keeps a stable pointer to "build the input from whatever the state is
  // *right now*" — read by the debounced autosave below, which fires well
  // after the keystroke that scheduled it, so it must never close over a
  // stale render's values (see buildInputRef usage below).
  const buildInput = (): DraftInvoiceInput => ({
    customer_id: customerId,
    invoice_date: invoiceDate,
    due_date: dueDate,
    notes: notes || null,
    terms: terms || null,
    is_interstate: isInterstate,
    discount_type: discountStr.trim() === "" ? null : discountIsPercentage ? "PERCENTAGE" : "AMOUNT",
    discount_value:
      discountStr.trim() === ""
        ? null
        : discountIsPercentage
          ? parsePercentToBasisPoints(discountStr)
          : parseToMinor(discountStr),
    line_items: lines.map((l) => ({
      product_id: l.product_id,
      description: l.description,
      unit: l.unit,
      quantity_thousandths: parseQuantityToThousandths(l.quantityStr),
      unit_price_minor: parseToMinor(l.priceStr),
      line_discount_type: l.lineDiscountStr.trim() === "" ? null : l.lineDiscountIsPercentage ? "PERCENTAGE" : "AMOUNT",
      line_discount_value:
        l.lineDiscountStr.trim() === ""
          ? null
          : l.lineDiscountIsPercentage
            ? parsePercentToBasisPoints(l.lineDiscountStr)
            : parseToMinor(l.lineDiscountStr),
      tax_rate_id: l.taxRateId,
      tax_rate_basis_points: parsePercentToBasisPoints(l.taxStr),
    })),
  });
  const buildInputRef = useRef(buildInput);
  buildInputRef.current = buildInput;

  // Draft saves go through `update_draft_invoice`; Issued/PartiallyPaid/Paid
  // go through `edit_issued_invoice` instead (the backend rejects the wrong
  // one for the wrong status) — this is the one place that branch lives, so
  // autosave, "Save Draft", and "Save Changes" can't drift apart.
  const persistRef = useRef(invoice);
  persistRef.current = invoice;
  const saveNow = (input: DraftInvoiceInput) => {
    const status = persistRef.current?.status;
    return status === "DRAFT" ? updateDraftInvoice(invoiceId, input) : editIssuedInvoice(invoiceId, input);
  };

  const autosaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    return () => {
      if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    };
  }, []);

  const runAutosave = async () => {
    setAutoSaving(true);
    try {
      const updated = await saveNow(buildInputRef.current());
      setInvoice(updated);
      setLines((prev) => mergeEditableLines(prev, updated, formatMinor));
    } catch (err) {
      setError(err);
    } finally {
      setAutoSaving(false);
    }
  };

  const scheduleAutosave = () => {
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    autosaveTimerRef.current = setTimeout(() => void runAutosave(), AUTOSAVE_DEBOUNCE_MS);
  };

  if (!invoice) {
    return (
      <div>
        <button onClick={onBack} className="mb-4 text-sm text-zinc-500 dark:text-zinc-400 transition-colors hover:underline">
          ← Back
        </button>
        <ErrorBanner error={error} />
        {!error && <p className="text-zinc-400 dark:text-zinc-500">Loading…</p>}
      </div>
    );
  }

  const isDraft = invoice.status === "DRAFT";
  const isCancelled = invoice.status === "CANCELLED";
  const isEditable = !isCancelled;
  // Same is_overdue predicate as database-schema.md §8 (status ISSUED/PARTIALLY_PAID,
  // due_date passed) — a button-visibility check, not a recomputed financial figure.
  const isOverdue =
    (invoice.status === "ISSUED" || invoice.status === "PARTIALLY_PAID") &&
    invoice.due_date !== null &&
    invoice.due_date < new Date().toISOString().slice(0, 10);

  // ui-ux.md §3: a payment mutation changes the invoice's `status` as a side
  // effect the payment panel itself doesn't return inline — refetch here so
  // the status badge and totals stay in sync without a manual refresh.
  const reloadInvoiceAfterPaymentChange = () => {
    getInvoice(invoiceId)
      .then((inv) => {
        setInvoice(inv);
        setLines(toEditableLines(inv, formatMinor));
      })
      .catch((err: unknown) => setError(err));
  };

  const addLine = () => {
    setLines((ls) => [
      ...ls,
      {
        key: newKey(),
        product_id: null,
        description: "",
        unit: "",
        quantityStr: "1",
        priceStr: "0",
        lineDiscountIsPercentage: false,
        lineDiscountStr: "",
        ...resolveTaxRate(taxRates, defaultTaxRateId),
      },
    ]);
    scheduleAutosave();
  };
  const removeLine = (key: string) => {
    setLines((ls) => ls.filter((l) => l.key !== key));
    scheduleAutosave();
  };
  const updateLine = (key: string, patch: Partial<EditableLine>) => {
    setLines((ls) => ls.map((l) => (l.key === key ? { ...l, ...patch } : l)));
    scheduleAutosave();
  };

  // A picked product's own tax rate wins; if it has none, fall back to the
  // business's configured default (Settings → Invoicing) rather than
  // silently dropping to 0%.
  const applyProduct = (key: string, product: ProductListItem) => {
    updateLine(key, {
      product_id: product.id,
      description: product.name,
      unit: product.unit,
      priceStr: formatMinor(product.price_minor),
      ...resolveTaxRate(taxRates, product.tax_rate_id ?? defaultTaxRateId),
    });
  };
  const pickProduct = (key: string, productId: number) => {
    const product = products.find((p) => p.id === productId);
    if (product) applyProduct(key, product);
  };

  const handleSave = async () => {
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    setError(null);
    setSaving(true);
    try {
      const updated = await saveNow(buildInput());
      setInvoice(updated);
      setLines((prev) => mergeEditableLines(prev, updated, formatMinor));
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  /** Returns whether the invoice actually got issued, so "Issue & PDF" can stop here on failure. */
  const handleIssue = async () => {
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    setError(null);
    setSaving(true);
    try {
      await updateDraftInvoice(invoice.id, buildInput());
      const issued = await issueInvoice(invoice.id, useCustomNumber && customNumber.trim() !== "" ? customNumber.trim() : null);
      setInvoice(issued);
      setLines(toEditableLines(issued, formatMinor));
      return true;
    } catch (err) {
      setError(err);
      return false;
    } finally {
      setSaving(false);
    }
  };

  // Every PDF action renders from the invoice's *saved* state, so a pending
  // autosave has to land first — otherwise "Save & PDF" on a just-edited
  // draft would print the version from before the last keystroke.
  const flushPendingEdits = async () => {
    if (!isEditable) return;
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    const updated = await saveNow(buildInput());
    setInvoice(updated);
    setLines((prev) => mergeEditableLines(prev, updated, formatMinor));
  };

  const handlePreview = async () => {
    setError(null);
    setSavedPdfPath(null);
    try {
      await flushPendingEdits();
    } catch (err) {
      setError(err);
      return;
    }
    await pdf.preview(invoice.id);
  };

  const handleSavePdf = async () => {
    setError(null);
    try {
      await flushPendingEdits();
    } catch (err) {
      setError(err);
      return;
    }
    const path = await pdf.saveAs(invoice.id, pdf.suggestedFileName(invoice.invoice_number, invoice.id));
    if (path) setSavedPdfPath(path);
  };

  /** user-flows.md §5: "Save & PDF" — issue, then go straight to the PDF. */
  const handleIssueAndPdf = async () => {
    if (!(await handleIssue())) return;
    await pdf.preview(invoice.id);
  };

  const handleDuplicate = async () => {
    setError(null);
    setDuplicating(true);
    try {
      const dup = await duplicateInvoice(invoice.id);
      onOpenInvoice(dup.id);
    } catch (err) {
      setError(err);
      setDuplicating(false);
    }
  };

  return (
    <div className="max-w-3xl space-y-4">
      <button onClick={onBack} className="text-sm text-zinc-500 dark:text-zinc-400 transition-colors hover:underline">
        ← Back
      </button>

      <div className="flex items-center gap-3">
        <h1 className="text-xl font-semibold">{invoice.invoice_number ?? `Draft #${invoice.id}`}</h1>
        <StatusBadge status={invoice.status} isOverdue={false} />
      </div>

      <ErrorBanner error={error} />

      {isCancelled && (
        <p className="text-sm text-zinc-400 dark:text-zinc-500">Cancelled{invoice.cancel_reason ? `: ${invoice.cancel_reason}` : "."}</p>
      )}

      <div className="grid grid-cols-2 gap-4 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
        <div>
          <label className="block text-sm">Customer</label>
          {isEditable ? (
            <SearchablePicker
              className="mt-1"
              items={customers.map((c) => ({ id: c.id, label: c.name }))}
              value={customerId}
              onChange={(id) => {
                setCustomerId(id);
                scheduleAutosave();
              }}
              placeholder="Search or select customer…"
              createLabel="Create new customer…"
              onCreateNew={() => setShowNewCustomerModal(true)}
            />
          ) : (
            <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">{invoice.customer_snapshot_name ?? "—"}</p>
          )}
        </div>

        <div>
          {isDraft ? (
            <div className="text-sm">
              {numberPreview && !useCustomNumber && (
                <p className="text-zinc-500 dark:text-zinc-400">
                  Next invoice number • automatic
                  <br />
                  <span className="text-zinc-900 dark:text-zinc-100">{numberPreview}</span>
                </p>
              )}
              <label className="mt-1 flex items-center gap-2 text-xs text-zinc-400 dark:text-zinc-500">
                <input type="checkbox" checked={useCustomNumber} onChange={(e) => setUseCustomNumber(e.target.checked)} />
                Use a custom number instead
              </label>
              {useCustomNumber && (
                <input
                  value={customNumber}
                  onChange={(e) => setCustomNumber(e.target.value)}
                  placeholder="e.g. OLD-INV-1042"
                  className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                />
              )}
            </div>
          ) : (
            <p className="text-sm text-zinc-500 dark:text-zinc-400">Invoice number (immutable): {invoice.invoice_number}</p>
          )}
        </div>

        <label className="block text-sm">
          Invoice date
          <input
            type="date"
            disabled={!isEditable}
            value={invoiceDate}
            onChange={(e) => {
              setInvoiceDate(e.target.value);
              scheduleAutosave();
            }}
            className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 disabled:opacity-60 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
        </label>
        <label className="block text-sm">
          Due date
          <input
            type="date"
            disabled={!isEditable}
            value={dueDate ?? ""}
            onChange={(e) => {
              setDueDate(e.target.value || null);
              scheduleAutosave();
            }}
            className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 disabled:opacity-60 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
        </label>

        {taxFields.has("is_interstate") && (
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              disabled={!isEditable}
              checked={isInterstate}
              onChange={(e) => {
                setIsInterstate(e.target.checked);
                scheduleAutosave();
              }}
            />
            Interstate (IGST instead of CGST+SGST)
          </label>
        )}
      </div>

      <div className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
        <table className="w-full text-left text-sm">
          <thead className="text-zinc-500 dark:text-zinc-400">
            <tr>
              <th className="pb-2">Item</th>
              <th className="pb-2">Unit</th>
              <th className="pb-2">Qty</th>
              <th className="pb-2">Rate ({symbol})</th>
              <th className="pb-2">Tax %</th>
              <th className="pb-2">Discount</th>
              {isEditable && <th className="pb-2"></th>}
            </tr>
          </thead>
          <tbody>
            {lines.map((l) => (
              <tr key={l.key} className="border-t border-zinc-200 dark:border-zinc-800">
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <div className="space-y-1">
                      <SearchablePicker
                        items={products.map((p) => ({ id: p.id, label: p.name }))}
                        value={l.product_id}
                        onChange={(id) => pickProduct(l.key, id)}
                        placeholder="Pick a product…"
                        createLabel="Create new product…"
                        onCreateNew={() => setNewProductForLineKey(l.key)}
                      />
                      <input
                        value={l.description}
                        onChange={(e) => updateLine(l.key, { description: e.target.value })}
                        placeholder="Description"
                        className="w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                      />
                    </div>
                  ) : (
                    l.description
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <input value={l.unit} onChange={(e) => updateLine(l.key, { unit: e.target.value })} className="w-16 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
                  ) : (
                    l.unit
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <input value={l.quantityStr} onChange={(e) => updateLine(l.key, { quantityStr: e.target.value })} className="w-16 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
                  ) : (
                    l.quantityStr
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <input value={l.priceStr} onChange={(e) => updateLine(l.key, { priceStr: e.target.value })} className="w-20 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
                  ) : (
                    `${symbol}${l.priceStr}`
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <div className="flex items-center gap-1">
                      <select
                        value={l.taxRateId ?? "custom"}
                        onChange={(e) => {
                          if (e.target.value === "custom") {
                            updateLine(l.key, { taxRateId: null });
                            return;
                          }
                          const rate = taxRates.find((r) => r.id === Number(e.target.value));
                          if (rate) updateLine(l.key, resolveTaxRate(taxRates, rate.id));
                        }}
                        className="w-24 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-1 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                      >
                        {taxRates.map((r) => (
                          <option key={r.id} value={r.id}>
                            {r.name}
                          </option>
                        ))}
                        <option value="custom">Custom %</option>
                      </select>
                      {l.taxRateId === null && (
                        <input
                          value={l.taxStr}
                          onChange={(e) => updateLine(l.key, { taxStr: e.target.value })}
                          placeholder="%"
                          className="w-12 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                        />
                      )}
                    </div>
                  ) : (
                    `${l.taxStr}%`
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isEditable ? (
                    <div className="flex items-center gap-1">
                      <select
                        value={l.lineDiscountIsPercentage ? "PERCENTAGE" : "AMOUNT"}
                        onChange={(e) => updateLine(l.key, { lineDiscountIsPercentage: e.target.value === "PERCENTAGE" })}
                        className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-1 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                      >
                        <option value="AMOUNT">{symbol}</option>
                        <option value="PERCENTAGE">%</option>
                      </select>
                      <input
                        value={l.lineDiscountStr}
                        onChange={(e) => updateLine(l.key, { lineDiscountStr: e.target.value })}
                        placeholder="0"
                        className="w-16 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                      />
                    </div>
                  ) : l.lineDiscountStr === "" ? (
                    "—"
                  ) : l.lineDiscountIsPercentage ? (
                    `${l.lineDiscountStr}%`
                  ) : (
                    `${symbol}${l.lineDiscountStr}`
                  )}
                </td>
                {isEditable && (
                  <td className="py-2">
                    <button onClick={() => removeLine(l.key)} className="text-red-600 dark:text-red-400 transition-colors hover:underline">
                      Remove
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
        {isEditable && (
          <button onClick={addLine} className="mt-2 text-sm text-blue-600 dark:text-blue-400 transition-colors hover:underline">
            + Add item
          </button>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
          <label className="block text-sm">
            Discount
            {isEditable ? (
              <div className="mt-1 flex gap-2">
                <select
                  value={discountIsPercentage ? "PERCENTAGE" : "AMOUNT"}
                  onChange={(e) => {
                    setDiscountIsPercentage(e.target.value === "PERCENTAGE");
                    scheduleAutosave();
                  }}
                  className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="AMOUNT">{symbol} Amount</option>
                  <option value="PERCENTAGE">% Percentage</option>
                </select>
                <input
                  value={discountStr}
                  onChange={(e) => {
                    setDiscountStr(e.target.value);
                    scheduleAutosave();
                  }}
                  placeholder="0"
                  className="w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
            ) : (
              <p className="text-zinc-500 dark:text-zinc-400">{discountStr === "" ? "None" : `${discountIsPercentage ? `${discountStr}%` : `${symbol}${discountStr}`}`}</p>
            )}
          </label>
          <label className="block text-sm">
            Notes
            <textarea
              disabled={!isEditable}
              value={notes}
              onChange={(e) => {
                setNotes(e.target.value);
                scheduleAutosave();
              }}
              className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 disabled:opacity-60 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
          </label>
          <label className="block text-sm">
            Terms
            <textarea
              disabled={!isEditable}
              value={terms}
              onChange={(e) => {
                setTerms(e.target.value);
                scheduleAutosave();
              }}
              className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 disabled:opacity-60 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
          </label>
        </div>

        <div className="space-y-1 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4 text-sm">
          <div className="flex justify-between">
            <span className="text-zinc-500 dark:text-zinc-400">Subtotal</span>
            <span>{symbol}{formatMinor(invoice.subtotal_minor)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-zinc-500 dark:text-zinc-400">Discount</span>
            <span>-{symbol}{formatMinor(invoice.discount_amount_minor)}</span>
          </div>
          {effectiveRegime === "VAT_STANDARD" ? (
            <div className="flex justify-between">
              <span className="text-zinc-500 dark:text-zinc-400">VAT</span>
              <span>+{symbol}{formatMinor(presentVat(invoice.tax_amount_minor).vatAmountMinor)}</span>
            </div>
          ) : invoice.is_interstate ? (
            <div className="flex justify-between">
              <span className="text-zinc-500 dark:text-zinc-400">IGST</span>
              <span>+{symbol}{formatMinor(splitGst(invoice.tax_amount_minor, true).igst)}</span>
            </div>
          ) : (
            <>
              <div className="flex justify-between">
                <span className="text-zinc-500 dark:text-zinc-400">CGST</span>
                <span>+{symbol}{formatMinor(splitGst(invoice.tax_amount_minor, false).cgst)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500 dark:text-zinc-400">SGST</span>
                <span>+{symbol}{formatMinor(splitGst(invoice.tax_amount_minor, false).sgst)}</span>
              </div>
            </>
          )}
          <div className="mt-2 flex justify-between border-t border-zinc-300 dark:border-zinc-700 pt-2 text-base font-semibold">
            <span>Total</span>
            <span>{symbol}{formatMinor(invoice.total_minor)}</span>
          </div>
          {isEditable && (
            <p className="pt-2 text-xs text-zinc-400 dark:text-zinc-500">{autoSaving ? "Recalculating totals…" : "Totals update automatically as you edit."}</p>
          )}
        </div>
      </div>

      {isDraft && (
        <div className="flex gap-2">
          <button onClick={handleSave} disabled={saving} className="rounded-md bg-zinc-200 px-4 py-2 font-medium text-zinc-900 transition-colors hover:bg-zinc-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 dark:bg-zinc-700 dark:text-zinc-100 dark:hover:bg-zinc-600 dark:focus:ring-offset-zinc-900">
            {saving ? "Saving…" : "Save Draft"}
          </button>
          <button
            onClick={() => void handlePreview()}
            disabled={saving || pdf.busy}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50"
          >
            {pdf.busy ? "Rendering…" : "Preview"}
          </button>
          <button
            onClick={handleIssue}
            disabled={saving || customerId === null || lines.length === 0}
            className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50"
          >
            {saving ? "Issuing…" : "Issue"}
          </button>
          <button
            onClick={() => void handleIssueAndPdf()}
            disabled={saving || pdf.busy || customerId === null || lines.length === 0}
            className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50"
          >
            Issue &amp; PDF
          </button>
        </div>
      )}

      {!isDraft && !isCancelled && (
        <div className="flex gap-2">
          <button onClick={handleSave} disabled={saving} className="rounded-md bg-zinc-200 px-4 py-2 font-medium text-zinc-900 transition-colors hover:bg-zinc-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 dark:bg-zinc-700 dark:text-zinc-100 dark:hover:bg-zinc-600 dark:focus:ring-offset-zinc-900">
            {saving ? "Saving…" : "Save Changes"}
          </button>
          <button
            onClick={() => void handleSavePdf()}
            disabled={pdf.busy}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50"
          >
            {pdf.busy ? "Rendering…" : "Print / Save PDF"}
          </button>
          <button
            onClick={() => void handlePreview()}
            disabled={pdf.busy}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50"
          >
            Preview
          </button>
          <button onClick={() => void handleDuplicate()} disabled={duplicating} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50">
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
          {isOverdue && (
            <button onClick={() => setShowReminder(true)} className="rounded-md border border-red-300 px-4 py-2 font-medium text-red-600 transition-colors hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-950">
              Remind
            </button>
          )}
          <button
            onClick={() => {
              setCancelReason("");
              setShowCancelDialog(true);
            }}
            className="rounded-md border border-amber-300 px-4 py-2 font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:border-amber-800 dark:text-amber-400 dark:hover:bg-amber-950"
          >
            Cancel Invoice
          </button>
        </div>
      )}

      {isCancelled && (
        <div className="flex gap-2">
          <button onClick={() => void handleDuplicate()} disabled={duplicating} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50">
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
          {/* A cancelled invoice still prints — stamped CANCELLED — because the
              copy already sent to the customer has to be answerable. */}
          <button
            onClick={() => void handlePreview()}
            disabled={pdf.busy}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50"
          >
            Preview
          </button>
        </div>
      )}

      {!isDraft && (
        <PaymentPanel
          invoiceId={invoice.id}
          invoiceStatus={invoice.status}
          totalMinor={invoice.total_minor}
          onChanged={reloadInvoiceAfterPaymentChange}
        />
      )}

      <ErrorBanner error={pdf.error} />
      {savedPdfPath && <p className="text-sm text-green-600 dark:text-green-400">Saved to {savedPdfPath}</p>}

      {pdf.previewUrl && (
        <InvoicePdfPreview
          url={pdf.previewUrl}
          title={pdf.previewTitle}
          saving={pdf.busy}
          onClose={pdf.closePreview}
          onSave={() => void handleSavePdf()}
        />
      )}

      {showReminder && (
        <ReminderModal invoiceId={invoice.id} invoiceNumber={invoice.invoice_number} onClose={() => setShowReminder(false)} />
      )}

      {showNewCustomerModal && (
        <Modal onClose={() => setShowNewCustomerModal(false)}>
          <CustomerForm
            onCancel={() => setShowNewCustomerModal(false)}
            onSubmit={async (fields) => {
              const created = await createCustomer(fields);
              setCustomers((cs) => [...cs, { ...created, has_invoices: false }]);
              setCustomerId(created.id);
              setShowNewCustomerModal(false);
              scheduleAutosave();
              return created;
            }}
          />
        </Modal>
      )}

      {newProductForLineKey !== null && (
        <Modal onClose={() => setNewProductForLineKey(null)}>
          <ProductForm
            onCancel={() => setNewProductForLineKey(null)}
            onSubmit={async (fields) => {
              const created = await createProduct(fields);
              setProducts((ps) => [...ps, { ...created, has_invoices: false }]);
              applyProduct(newProductForLineKey, { ...created, has_invoices: false });
              setNewProductForLineKey(null);
              return created;
            }}
          />
        </Modal>
      )}

      {showCancelDialog && (
        <ConfirmDialog
          title="Cancel this invoice?"
          message="Cancelling is terminal — a cancelled invoice can't be edited or issued again."
          confirmLabel="Cancel Invoice"
          danger
          onCancel={() => setShowCancelDialog(false)}
          onConfirm={async () => {
            const updated = await cancelInvoice(invoice.id, cancelReason.trim() || null).then(() => getInvoice(invoice.id));
            setInvoice(updated);
            setLines(toEditableLines(updated, formatMinor));
            setShowCancelDialog(false);
          }}
        >
          <label className="block text-sm">
            Reason (optional)
            <textarea
              value={cancelReason}
              onChange={(e) => setCancelReason(e.target.value)}
              className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
              rows={2}
            />
          </label>
        </ConfirmDialog>
      )}
    </div>
  );
}
