import { useEffect, useRef, useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { Modal } from "../../components/Modal";
import { SearchablePicker } from "../../components/SearchablePicker";
import { QuoteStatusBadge } from "../../components/StatusBadge";
import { CustomerForm } from "../customers/CustomerForm";
import { ProductForm } from "../products/ProductForm";
import {
  acceptQuote,
  cancelQuote,
  convertQuoteToInvoice,
  createCustomer,
  createProduct,
  declineQuote,
  duplicateQuote,
  getQuote,
  getSettings,
  issueQuote,
  listCustomers,
  listProducts,
  listTaxRates,
  previewNextQuoteNumber,
  updateDraftQuote,
} from "../../lib/tauri/commands";
import { useBusiness } from "../../hooks/useBusiness";
import { useCurrency } from "../../hooks/useCurrency";
import { useTaxRegimeFields } from "../../hooks/useTaxRegimeFields";
import {
  formatThousandthsAsQuantity,
  formatBasisPointsAsPercent,
  parsePercentToBasisPoints,
  parseQuantityToThousandths,
  presentVat,
  splitGst,
  type CustomerListItem,
  type DraftQuoteInput,
  type ProductListItem,
  type QuoteWithLineItems,
  type TaxRate,
} from "../../lib/tauri/types";

/** Resolves a tax rate id against the loaded list, falling back to "no tax rate" if it's gone. */
function resolveTaxRate(taxRates: TaxRate[], taxRateId: number | null): { taxRateId: number | null; taxStr: string } {
  const rate = taxRateId === null ? undefined : taxRates.find((r) => r.id === taxRateId);
  return rate ? { taxRateId: rate.id, taxStr: formatBasisPointsAsPercent(rate.rate_basis_points) } : { taxRateId: null, taxStr: "0" };
}

/**
 * ui-ux-v2.md §4 — one component for create/edit-draft/view-issued/etc.,
 * mirroring InvoiceEditor.tsx's shape. The one structural difference: a
 * Quote is editable in `DRAFT` only (user-flows-v2.md §2 — no
 * `EditIssued`-equivalent the way invoices get one), so there is no
 * "re-snapshot on save while issued" branch to carry, and no PDF/payment
 * integration (neither is part of the locked V2 Quote scope).
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
  return `quote-line-${keyCounter}`;
}

function toEditableLine(li: QuoteWithLineItems["line_items"][number], key: string, formatMinor: (m: number) => string): EditableLine {
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

function toEditableLines(quote: QuoteWithLineItems, formatMinor: (m: number) => string): EditableLine[] {
  return quote.line_items.map((li) => toEditableLine(li, newKey(), formatMinor));
}

function mergeEditableLines(prevLines: EditableLine[], quote: QuoteWithLineItems, formatMinor: (m: number) => string): EditableLine[] {
  return quote.line_items.map((li, i) => toEditableLine(li, prevLines[i]?.key ?? newKey(), formatMinor));
}

const AUTOSAVE_DEBOUNCE_MS = 600;

export function QuoteEditor({
  quoteId,
  onBack,
  onOpenQuote,
  onConverted,
}: {
  quoteId: number;
  onBack: () => void;
  onOpenQuote: (id: number) => void;
  /** ui-ux-v2.md §4 — a converted quote's only footer action is a link to
   * the resulting invoice; the Invoices section owns opening it. */
  onConverted: (invoiceId: number) => void;
}) {
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const { business } = useBusiness();
  const [quote, setQuote] = useState<QuoteWithLineItems | null>(null);
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
  const [converting, setConverting] = useState(false);

  const [customerId, setCustomerId] = useState<number | null>(null);
  const [quoteDate, setQuoteDate] = useState("");
  const [validUntil, setValidUntil] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [terms, setTerms] = useState("");
  const [isInterstate, setIsInterstate] = useState(false);
  const [discountIsPercentage, setDiscountIsPercentage] = useState(false);
  const [discountStr, setDiscountStr] = useState("");
  const [lines, setLines] = useState<EditableLine[]>([]);

  // application-architecture-v2.md §4d — same fallback rule as the Invoice
  // Editor: an issued Quote prints its frozen snapshot, a Draft reflects the
  // business's current regime.
  const effectiveRegime = quote?.tax_regime_snapshot ?? business?.tax_regime_code ?? "IN_GST";
  const taxFields = useTaxRegimeFields(effectiveRegime);

  useEffect(() => {
    Promise.all([
      getQuote(quoteId),
      listCustomers({ include_archived: false }),
      listProducts({ include_archived: false }),
      listTaxRates(),
      getSettings(),
    ])
      .then(([q, custs, prods, rates, settings]) => {
        setQuote(q);
        setCustomers(custs);
        setProducts(prods);
        setTaxRates(rates);
        setDefaultTaxRateId(settings.default_tax_rate_id);
        setCustomerId(q.customer_id);
        setQuoteDate(q.quote_date);
        setValidUntil(q.valid_until);
        setNotes(q.notes ?? "");
        setTerms(q.terms ?? "");
        setIsInterstate(q.is_interstate);
        setDiscountIsPercentage(q.discount_type === "PERCENTAGE");
        setDiscountStr(
          q.discount_value === null ? "" : q.discount_type === "PERCENTAGE" ? formatBasisPointsAsPercent(q.discount_value) : formatMinor(q.discount_value),
        );
        setLines(toEditableLines(q, formatMinor));
        if (q.status === "DRAFT") {
          previewNextQuoteNumber().then(setNumberPreview).catch(() => setNumberPreview(null));
        }
      })
      .catch((err: unknown) => setError(err));
  }, [quoteId, formatMinor]);

  const buildInput = (): DraftQuoteInput => ({
    customer_id: customerId,
    quote_date: quoteDate,
    valid_until: validUntil,
    notes: notes || null,
    terms: terms || null,
    is_interstate: isInterstate,
    discount_type: discountStr.trim() === "" ? null : discountIsPercentage ? "PERCENTAGE" : "AMOUNT",
    discount_value:
      discountStr.trim() === "" ? null : discountIsPercentage ? parsePercentToBasisPoints(discountStr) : parseToMinor(discountStr),
    line_items: lines.map((l) => ({
      product_id: l.product_id,
      description: l.description,
      unit: l.unit,
      quantity_thousandths: parseQuantityToThousandths(l.quantityStr),
      unit_price_minor: parseToMinor(l.priceStr),
      line_discount_type: l.lineDiscountStr.trim() === "" ? null : l.lineDiscountIsPercentage ? "PERCENTAGE" : "AMOUNT",
      line_discount_value:
        l.lineDiscountStr.trim() === "" ? null : l.lineDiscountIsPercentage ? parsePercentToBasisPoints(l.lineDiscountStr) : parseToMinor(l.lineDiscountStr),
      tax_rate_id: l.taxRateId,
      tax_rate_basis_points: parsePercentToBasisPoints(l.taxStr),
    })),
  });
  const buildInputRef = useRef(buildInput);
  buildInputRef.current = buildInput;

  const autosaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    return () => {
      if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    };
  }, []);

  const runAutosave = async () => {
    setAutoSaving(true);
    try {
      const updated = await updateDraftQuote(quoteId, buildInputRef.current());
      setQuote(updated);
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

  if (!quote) {
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

  const isDraft = quote.status === "DRAFT";
  const isCancelled = quote.status === "CANCELLED";
  const isConverted = quote.status === "CONVERTED";
  // Quotes are editable in Draft only — no exception for a later status,
  // unlike an issued invoice (user-flows-v2.md §2's locked rule).
  const isEditable = isDraft;

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
      const updated = await updateDraftQuote(quoteId, buildInput());
      setQuote(updated);
      setLines((prev) => mergeEditableLines(prev, updated, formatMinor));
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  const handleIssue = async () => {
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    setError(null);
    setSaving(true);
    try {
      await updateDraftQuote(quote.id, buildInput());
      const issued = await issueQuote(quote.id);
      setQuote(issued);
      setLines(toEditableLines(issued, formatMinor));
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  const reloadQuote = () =>
    getQuote(quote.id)
      .then((q) => {
        setQuote(q);
        setLines(toEditableLines(q, formatMinor));
      })
      .catch((err: unknown) => setError(err));

  const handleAccept = async () => {
    setError(null);
    setSaving(true);
    try {
      await acceptQuote(quote.id);
      await reloadQuote();
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  const handleDecline = async () => {
    setError(null);
    setSaving(true);
    try {
      await declineQuote(quote.id);
      await reloadQuote();
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  const handleConvert = async () => {
    setError(null);
    setConverting(true);
    try {
      const invoice = await convertQuoteToInvoice(quote.id);
      onConverted(invoice.id);
    } catch (err) {
      setError(err);
      setConverting(false);
    }
  };

  const handleDuplicate = async () => {
    setError(null);
    setDuplicating(true);
    try {
      const dup = await duplicateQuote(quote.id);
      onOpenQuote(dup.id);
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
        <h1 className="text-xl font-semibold">{quote.quote_number ?? `Draft #${quote.id}`}</h1>
        <QuoteStatusBadge status={quote.status} isExpired={false} />
      </div>

      <ErrorBanner error={error} />

      {isCancelled && <p className="text-sm text-zinc-400 dark:text-zinc-500">Cancelled{quote.cancel_reason ? `: ${quote.cancel_reason}` : "."}</p>}
      {isConverted && (
        <p className="text-sm text-zinc-400 dark:text-zinc-500">
          This quote has been converted to an invoice. (Open it from the Invoices list — quote-to-invoice cross-navigation
          isn't wired up yet.)
        </p>
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
            <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">{quote.customer_snapshot_name ?? "—"}</p>
          )}
        </div>

        <div>
          {isDraft ? (
            numberPreview && (
              <p className="text-sm text-zinc-500 dark:text-zinc-400">
                Next quote number • automatic
                <br />
                <span className="text-zinc-900 dark:text-zinc-100">{numberPreview}</span>
              </p>
            )
          ) : (
            <p className="text-sm text-zinc-500 dark:text-zinc-400">Quote number (immutable): {quote.quote_number}</p>
          )}
        </div>

        <label className="block text-sm">
          Quote date
          <input
            type="date"
            disabled={!isEditable}
            value={quoteDate}
            onChange={(e) => {
              setQuoteDate(e.target.value);
              scheduleAutosave();
            }}
            className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 disabled:opacity-60 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
        </label>
        <label className="block text-sm">
          Valid until
          <input
            type="date"
            disabled={!isEditable}
            value={validUntil ?? ""}
            onChange={(e) => {
              setValidUntil(e.target.value || null);
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
            <span>
              {symbol}
              {formatMinor(quote.subtotal_minor)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-zinc-500 dark:text-zinc-400">Discount</span>
            <span>
              -{symbol}
              {formatMinor(quote.discount_amount_minor)}
            </span>
          </div>
          {effectiveRegime === "VAT_STANDARD" ? (
            <div className="flex justify-between">
              <span className="text-zinc-500 dark:text-zinc-400">VAT</span>
              <span>
                +{symbol}
                {formatMinor(presentVat(quote.tax_amount_minor).vatAmountMinor)}
              </span>
            </div>
          ) : quote.is_interstate ? (
            <div className="flex justify-between">
              <span className="text-zinc-500 dark:text-zinc-400">IGST</span>
              <span>
                +{symbol}
                {formatMinor(splitGst(quote.tax_amount_minor, true).igst)}
              </span>
            </div>
          ) : (
            <>
              <div className="flex justify-between">
                <span className="text-zinc-500 dark:text-zinc-400">CGST</span>
                <span>
                  +{symbol}
                  {formatMinor(splitGst(quote.tax_amount_minor, false).cgst)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500 dark:text-zinc-400">SGST</span>
                <span>
                  +{symbol}
                  {formatMinor(splitGst(quote.tax_amount_minor, false).sgst)}
                </span>
              </div>
            </>
          )}
          <div className="mt-2 flex justify-between border-t border-zinc-300 dark:border-zinc-700 pt-2 text-base font-semibold">
            <span>Total</span>
            <span>
              {symbol}
              {formatMinor(quote.total_minor)}
            </span>
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
            onClick={handleIssue}
            disabled={saving || customerId === null || lines.length === 0}
            className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50"
          >
            {saving ? "Issuing…" : "Issue"}
          </button>
        </div>
      )}

      {quote.status === "ISSUED" && (
        <div className="flex gap-2">
          <button onClick={() => void handleAccept()} disabled={saving} className="rounded-md bg-green-600 px-4 py-2 font-medium text-white transition-colors hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50 dark:bg-green-600 dark:hover:bg-green-500 dark:focus:ring-offset-zinc-900">
            {saving ? "Saving…" : "Accept"}
          </button>
          <button onClick={() => void handleDecline()} disabled={saving} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50">
            Decline
          </button>
          <button
            onClick={() => {
              setCancelReason("");
              setShowCancelDialog(true);
            }}
            className="rounded-md border border-amber-300 px-4 py-2 font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:border-amber-800 dark:text-amber-400 dark:hover:bg-amber-950"
          >
            Cancel Quote
          </button>
          <button onClick={() => void handleDuplicate()} disabled={duplicating} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50">
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
        </div>
      )}

      {quote.status === "ACCEPTED" && (
        <div className="flex gap-2">
          <button onClick={() => void handleConvert()} disabled={converting} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50">
            {converting ? "Converting…" : "Convert to Invoice"}
          </button>
          <button
            onClick={() => {
              setCancelReason("");
              setShowCancelDialog(true);
            }}
            className="rounded-md border border-amber-300 px-4 py-2 font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:border-amber-800 dark:text-amber-400 dark:hover:bg-amber-950"
          >
            Cancel Quote
          </button>
          <button onClick={() => void handleDuplicate()} disabled={duplicating} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 font-medium disabled:opacity-50">
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
        </div>
      )}

      {(quote.status === "DECLINED" || isCancelled) && (
        <div className="flex gap-2">
          <button onClick={() => void handleDuplicate()} disabled={duplicating} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50">
            {duplicating ? "Duplicating…" : "Duplicate"}
          </button>
        </div>
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
          title="Cancel this quote?"
          message="Cancelling is terminal — a cancelled quote can't be edited, accepted, or converted again."
          confirmLabel="Cancel Quote"
          danger
          onCancel={() => setShowCancelDialog(false)}
          onConfirm={async () => {
            await cancelQuote(quote.id, cancelReason.trim() || null);
            await reloadQuote();
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
