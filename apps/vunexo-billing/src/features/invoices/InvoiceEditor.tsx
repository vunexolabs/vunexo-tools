import { useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { StatusBadge } from "../../components/StatusBadge";
import {
  getInvoice,
  issueInvoice,
  listCustomers,
  listProducts,
  previewNextInvoiceNumber,
  updateDraftInvoice,
} from "../../lib/tauri/commands";
import {
  formatMinorAsRupees,
  formatThousandthsAsQuantity,
  formatBasisPointsAsPercent,
  parsePercentToBasisPoints,
  parseQuantityToThousandths,
  parseRupeesToMinor,
  type CustomerListItem,
  type DraftInvoiceInput,
  type InvoiceWithLineItems,
  type ProductListItem,
} from "../../lib/tauri/types";

/**
 * ui-ux.md §4 — one component for create/edit-draft/view-issued, not three
 * screens; the fields/footer just change by `status`.
 *
 * Scope simplification versus the full spec: line-level discounts aren't
 * exposed in this editor (only the invoice-level discount) — the calculation
 * engine already supports them (calculation-engine.md §4 Step 2), this is a
 * UI-only trim to keep the first working editor small. Recalculation
 * happens on explicit "Save Draft", not on every keystroke — still nothing
 * is computed client-side (application-architecture.md §4a), it's just not
 * as continuously live as the full ui-ux.md vision describes.
 */

interface EditableLine {
  key: string;
  product_id: number | null;
  description: string;
  unit: string;
  quantityStr: string;
  priceStr: string;
  taxStr: string;
}

let keyCounter = 0;
function newKey() {
  keyCounter += 1;
  return `line-${keyCounter}`;
}

function toEditableLines(invoice: InvoiceWithLineItems): EditableLine[] {
  return invoice.line_items.map((li) => ({
    key: newKey(),
    product_id: li.product_id,
    description: li.description,
    unit: li.unit,
    quantityStr: formatThousandthsAsQuantity(li.quantity_thousandths),
    priceStr: formatMinorAsRupees(li.unit_price_minor),
    taxStr: formatBasisPointsAsPercent(li.tax_rate_basis_points),
  }));
}

export function InvoiceEditor({ invoiceId, onBack }: { invoiceId: number; onBack: () => void }) {
  const [invoice, setInvoice] = useState<InvoiceWithLineItems | null>(null);
  const [customers, setCustomers] = useState<CustomerListItem[]>([]);
  const [products, setProducts] = useState<ProductListItem[]>([]);
  const [numberPreview, setNumberPreview] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [saving, setSaving] = useState(false);

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

  useEffect(() => {
    Promise.all([
      getInvoice(invoiceId),
      listCustomers({ include_archived: false }),
      listProducts({ include_archived: false }),
    ])
      .then(([inv, custs, prods]) => {
        setInvoice(inv);
        setCustomers(custs);
        setProducts(prods);
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
              : formatMinorAsRupees(inv.discount_value),
        );
        setLines(toEditableLines(inv));
        if (inv.status === "DRAFT") {
          previewNextInvoiceNumber().then(setNumberPreview).catch(() => setNumberPreview(null));
        }
      })
      .catch((err: unknown) => setError(err));
  }, [invoiceId]);

  if (!invoice) {
    return (
      <div>
        <button onClick={onBack} className="mb-4 text-sm text-slate-400 hover:underline">
          ← Back
        </button>
        <ErrorBanner error={error} />
        {!error && <p className="text-slate-500">Loading…</p>}
      </div>
    );
  }

  const isDraft = invoice.status === "DRAFT";

  const addLine = () => setLines((ls) => [...ls, { key: newKey(), product_id: null, description: "", unit: "", quantityStr: "1", priceStr: "0", taxStr: "0" }]);
  const removeLine = (key: string) => setLines((ls) => ls.filter((l) => l.key !== key));
  const updateLine = (key: string, patch: Partial<EditableLine>) =>
    setLines((ls) => ls.map((l) => (l.key === key ? { ...l, ...patch } : l)));

  const pickProduct = (key: string, productId: number) => {
    const product = products.find((p) => p.id === productId);
    if (!product) return;
    updateLine(key, {
      product_id: product.id,
      description: product.name,
      unit: product.unit,
      priceStr: formatMinorAsRupees(product.price_minor),
    });
  };

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
          : parseRupeesToMinor(discountStr),
    line_items: lines.map((l) => ({
      product_id: l.product_id,
      description: l.description,
      unit: l.unit,
      quantity_thousandths: parseQuantityToThousandths(l.quantityStr),
      unit_price_minor: parseRupeesToMinor(l.priceStr),
      line_discount_type: null,
      line_discount_value: null,
      tax_rate_id: null,
      tax_rate_basis_points: parsePercentToBasisPoints(l.taxStr),
    })),
  });

  const handleSaveDraft = async () => {
    setError(null);
    setSaving(true);
    try {
      const updated = await updateDraftInvoice(invoice.id, buildInput());
      setInvoice(updated);
      setLines(toEditableLines(updated));
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  const handleIssue = async () => {
    setError(null);
    setSaving(true);
    try {
      await updateDraftInvoice(invoice.id, buildInput());
      const issued = await issueInvoice(invoice.id, useCustomNumber && customNumber.trim() !== "" ? customNumber.trim() : null);
      setInvoice(issued);
      setLines(toEditableLines(issued));
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="max-w-3xl space-y-4">
      <button onClick={onBack} className="text-sm text-slate-400 hover:underline">
        ← Back
      </button>

      <div className="flex items-center gap-3">
        <h1 className="text-xl font-semibold">{invoice.invoice_number ?? `Draft #${invoice.id}`}</h1>
        <StatusBadge status={invoice.status} isOverdue={false} />
      </div>

      <ErrorBanner error={error} />

      {invoice.status === "CANCELLED" && invoice.cancel_reason && (
        <p className="text-sm text-slate-500">Cancelled: {invoice.cancel_reason}</p>
      )}

      <div className="grid grid-cols-2 gap-4 rounded border border-slate-700 bg-slate-900 p-4">
        <label className="block text-sm">
          Customer
          <select
            disabled={!isDraft}
            value={customerId ?? ""}
            onChange={(e) => setCustomerId(e.target.value ? Number(e.target.value) : null)}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 disabled:opacity-60"
          >
            <option value="">Select customer…</option>
            {customers.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>

        <div>
          {isDraft ? (
            <div className="text-sm">
              {numberPreview && !useCustomNumber && (
                <p className="text-slate-400">
                  Next invoice number • automatic
                  <br />
                  <span className="text-slate-300">{numberPreview}</span>
                </p>
              )}
              <label className="mt-1 flex items-center gap-2 text-xs text-slate-500">
                <input type="checkbox" checked={useCustomNumber} onChange={(e) => setUseCustomNumber(e.target.checked)} />
                Use a custom number instead
              </label>
              {useCustomNumber && (
                <input
                  value={customNumber}
                  onChange={(e) => setCustomNumber(e.target.value)}
                  placeholder="e.g. OLD-INV-1042"
                  className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
                />
              )}
            </div>
          ) : (
            <p className="text-sm text-slate-400">Invoice number (immutable): {invoice.invoice_number}</p>
          )}
        </div>

        <label className="block text-sm">
          Invoice date
          <input
            type="date"
            disabled={!isDraft}
            value={invoiceDate}
            onChange={(e) => setInvoiceDate(e.target.value)}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 disabled:opacity-60"
          />
        </label>
        <label className="block text-sm">
          Due date
          <input
            type="date"
            disabled={!isDraft}
            value={dueDate ?? ""}
            onChange={(e) => setDueDate(e.target.value || null)}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 disabled:opacity-60"
          />
        </label>

        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" disabled={!isDraft} checked={isInterstate} onChange={(e) => setIsInterstate(e.target.checked)} />
          Interstate (IGST instead of CGST+SGST)
        </label>
      </div>

      <div className="rounded border border-slate-700 bg-slate-900 p-4">
        <table className="w-full text-left text-sm">
          <thead className="text-slate-400">
            <tr>
              <th className="pb-2">Item</th>
              <th className="pb-2">Unit</th>
              <th className="pb-2">Qty</th>
              <th className="pb-2">Rate (₹)</th>
              <th className="pb-2">Tax %</th>
              {isDraft && <th className="pb-2"></th>}
            </tr>
          </thead>
          <tbody>
            {lines.map((l) => (
              <tr key={l.key} className="border-t border-slate-800">
                <td className="py-2 pr-2">
                  {isDraft ? (
                    <div className="space-y-1">
                      <select
                        value={l.product_id ?? ""}
                        onChange={(e) => e.target.value && pickProduct(l.key, Number(e.target.value))}
                        className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs"
                      >
                        <option value="">Pick a product…</option>
                        {products.map((p) => (
                          <option key={p.id} value={p.id}>
                            {p.name}
                          </option>
                        ))}
                      </select>
                      <input
                        value={l.description}
                        onChange={(e) => updateLine(l.key, { description: e.target.value })}
                        placeholder="Description"
                        className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1"
                      />
                    </div>
                  ) : (
                    l.description
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isDraft ? (
                    <input value={l.unit} onChange={(e) => updateLine(l.key, { unit: e.target.value })} className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-1" />
                  ) : (
                    l.unit
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isDraft ? (
                    <input value={l.quantityStr} onChange={(e) => updateLine(l.key, { quantityStr: e.target.value })} className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-1" />
                  ) : (
                    l.quantityStr
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isDraft ? (
                    <input value={l.priceStr} onChange={(e) => updateLine(l.key, { priceStr: e.target.value })} className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1" />
                  ) : (
                    `₹${l.priceStr}`
                  )}
                </td>
                <td className="py-2 pr-2">
                  {isDraft ? (
                    <input value={l.taxStr} onChange={(e) => updateLine(l.key, { taxStr: e.target.value })} className="w-16 rounded border border-slate-700 bg-slate-950 px-2 py-1" />
                  ) : (
                    l.taxStr
                  )}
                </td>
                {isDraft && (
                  <td className="py-2">
                    <button onClick={() => removeLine(l.key)} className="text-red-400 hover:underline">
                      Remove
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
        {isDraft && (
          <button onClick={addLine} className="mt-2 text-sm text-sky-400 hover:underline">
            + Add item
          </button>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2 rounded border border-slate-700 bg-slate-900 p-4">
          <label className="block text-sm">
            Discount
            {isDraft ? (
              <div className="mt-1 flex gap-2">
                <select
                  value={discountIsPercentage ? "PERCENTAGE" : "AMOUNT"}
                  onChange={(e) => setDiscountIsPercentage(e.target.value === "PERCENTAGE")}
                  className="rounded border border-slate-700 bg-slate-950 px-2 py-2 text-sm"
                >
                  <option value="AMOUNT">₹ Amount</option>
                  <option value="PERCENTAGE">% Percentage</option>
                </select>
                <input
                  value={discountStr}
                  onChange={(e) => setDiscountStr(e.target.value)}
                  placeholder="0"
                  className="w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
                />
              </div>
            ) : (
              <p className="text-slate-400">{discountStr === "" ? "None" : `${discountIsPercentage ? `${discountStr}%` : `₹${discountStr}`}`}</p>
            )}
          </label>
          <label className="block text-sm">
            Notes
            <textarea disabled={!isDraft} value={notes} onChange={(e) => setNotes(e.target.value)} className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 disabled:opacity-60" />
          </label>
          <label className="block text-sm">
            Terms
            <textarea disabled={!isDraft} value={terms} onChange={(e) => setTerms(e.target.value)} className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 disabled:opacity-60" />
          </label>
        </div>

        <div className="space-y-1 rounded border border-slate-700 bg-slate-900 p-4 text-sm">
          <div className="flex justify-between">
            <span className="text-slate-400">Subtotal</span>
            <span>₹{formatMinorAsRupees(invoice.subtotal_minor)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Discount</span>
            <span>-₹{formatMinorAsRupees(invoice.discount_amount_minor)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Tax</span>
            <span>+₹{formatMinorAsRupees(invoice.tax_amount_minor)}</span>
          </div>
          <div className="mt-2 flex justify-between border-t border-slate-700 pt-2 text-base font-semibold">
            <span>Total</span>
            <span>₹{formatMinorAsRupees(invoice.total_minor)}</span>
          </div>
          <p className="pt-2 text-xs text-slate-500">
            Totals shown are what the backend last computed — click "Save Draft" to recalculate after editing.
          </p>
        </div>
      </div>

      {isDraft && (
        <div className="flex gap-2">
          <button onClick={handleSaveDraft} disabled={saving} className="rounded bg-slate-700 px-4 py-2 font-medium disabled:opacity-50">
            {saving ? "Saving…" : "Save Draft"}
          </button>
          <button
            onClick={handleIssue}
            disabled={saving || customerId === null || lines.length === 0}
            className="rounded bg-sky-600 px-4 py-2 font-medium disabled:opacity-50"
          >
            {saving ? "Issuing…" : "Issue"}
          </button>
        </div>
      )}
    </div>
  );
}
