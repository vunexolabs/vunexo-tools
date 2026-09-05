import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { Modal } from "../../components/Modal";
import { SearchablePicker } from "../../components/SearchablePicker";
import { useCategories } from "../../hooks/useCategories";
import { useCurrency } from "../../hooks/useCurrency";
import { chooseOpenPath } from "../../lib/tauri/client";
import { attachReceipt, deleteExpense, removeReceipt, replaceReceipt } from "../../lib/tauri/commands";
import type { Expense, ExpenseInput } from "../../lib/tauri/types";
import { useVendors } from "../../hooks/useVendors";
import { CategoryQuickAddForm } from "./CategoryQuickAddForm";
import { VendorForm } from "../vendors/VendorForm";

const PAYMENT_METHODS = ["Cash", "Card", "Bank Transfer", "UPI", "Other"];

function toInput(expense: Expense | null, defaultCategoryId: number | null): ExpenseInput {
  if (expense) {
    return {
      date: expense.date,
      amount: expense.amount,
      tax_amount: expense.tax_amount,
      itc_eligible: expense.itc_eligible,
      deductible: expense.deductible,
      payment_method: expense.payment_method,
      notes: expense.notes,
      vendor_id: expense.vendor_id,
      category_id: expense.category_id,
    };
  }
  return {
    date: new Date().toISOString().slice(0, 10),
    amount: 0,
    tax_amount: 0,
    itc_eligible: false,
    deductible: false,
    payment_method: "Cash",
    notes: null,
    vendor_id: null,
    category_id: defaultCategoryId ?? 0,
  };
}

/**
 * ui-ux.md §4 — the Expense Editor, the core screen. Fields in entry order
 * per user-flows.md §5: vendor (optional, searchable picker), category
 * (required, searchable picker — pre-fills the deductible toggle from the
 * category's `default_deductible`, editable), date, amount, tax amount, an
 * ITC-eligible toggle (separate from deductible), payment method, notes,
 * receipt attachment. One "Save" action — no draft/issued state machine.
 */
export function ExpenseEditor({
  expense,
  onSaved,
  onDeleted,
  onBack,
  createExpense,
  updateExpense,
}: {
  expense: Expense | null;
  onSaved: (expense: Expense) => void;
  onDeleted: () => void;
  onBack: () => void;
  createExpense: (input: ExpenseInput) => Promise<Expense>;
  updateExpense: (id: number, input: ExpenseInput) => Promise<Expense>;
}) {
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const { categories, create: createCategory } = useCategories();
  const { vendors, create: createVendor } = useVendors();

  const [saved, setSaved] = useState<Expense | null>(expense);
  const [fields, setFields] = useState<ExpenseInput>(() => toInput(expense, categories?.[0]?.id ?? null));
  const [amountText, setAmountText] = useState(expense ? formatMinor(expense.amount) : "");
  const [taxText, setTaxText] = useState(expense ? formatMinor(expense.tax_amount) : "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [quickAdd, setQuickAdd] = useState<"vendor" | "category" | null>(null);
  const [receiptBusy, setReceiptBusy] = useState(false);
  const [receiptError, setReceiptError] = useState<unknown>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const set = <K extends keyof ExpenseInput>(key: K, value: ExpenseInput[K]) =>
    setFields((f) => ({ ...f, [key]: value }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const input: ExpenseInput = { ...fields, amount: parseToMinor(amountText), tax_amount: parseToMinor(taxText) };
      const result = saved ? await updateExpense(saved.id, input) : await createExpense(input);
      setSaved(result);
      onSaved(result);
    } catch (err) {
      setError(err);
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!saved) return;
    await deleteExpense(saved.id);
    onDeleted();
  };

  const handleAttach = async () => {
    if (!saved) return;
    setReceiptError(null);
    setReceiptBusy(true);
    try {
      const path = await chooseOpenPath({ filters: [{ name: "Receipt image", extensions: ["jpg", "jpeg", "png"] }] });
      if (!path) return;
      const updated = saved.receipt_path ? await replaceReceipt(saved.id, path) : await attachReceipt(saved.id, path);
      setSaved(updated);
      onSaved(updated);
    } catch (err) {
      setReceiptError(err);
    } finally {
      setReceiptBusy(false);
    }
  };

  const handleRemoveReceipt = async () => {
    if (!saved) return;
    setReceiptError(null);
    setReceiptBusy(true);
    try {
      const updated = await removeReceipt(saved.id);
      setSaved(updated);
      onSaved(updated);
    } catch (err) {
      setReceiptError(err);
    } finally {
      setReceiptBusy(false);
    }
  };

  return (
    <div className="max-w-xl space-y-4">
      <button onClick={onBack} className="text-sm text-slate-400 hover:underline">
        ← Back to Expenses
      </button>
      <h1 className="text-xl font-semibold">{saved ? "Edit Expense" : "New Expense"}</h1>

      <form onSubmit={handleSubmit} className="space-y-3 rounded border border-slate-700 bg-slate-900 p-4">
        <ErrorBanner error={error} />

        <label className="block text-sm">
          Vendor (optional)
          <SearchablePicker
            items={(vendors ?? []).map((v) => ({ id: v.id, label: v.name }))}
            value={fields.vendor_id}
            onChange={(id) => set("vendor_id", id)}
            placeholder="Search vendors…"
            createLabel="Create new vendor"
            onCreateNew={() => setQuickAdd("vendor")}
            className="mt-1"
          />
        </label>

        <label className="block text-sm">
          Category *
          <SearchablePicker
            items={(categories ?? []).map((c) => ({ id: c.id, label: c.name }))}
            value={fields.category_id || null}
            onChange={(id) => {
              set("category_id", id);
              // ui-ux.md §4 — picking a category pre-fills the expense's own
              // deductible flag from the category's current default; still
              // freely editable afterward.
              const category = categories?.find((c) => c.id === id);
              if (category) set("deductible", category.default_deductible);
            }}
            placeholder="Search categories…"
            createLabel="Create new category"
            onCreateNew={() => setQuickAdd("category")}
            className="mt-1"
          />
        </label>

        <div className="grid grid-cols-2 gap-3">
          <label className="block text-sm">
            Date
            <input
              type="date"
              required
              value={fields.date}
              onChange={(e) => set("date", e.target.value)}
              className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
            />
          </label>
          <label className="block text-sm">
            Payment method
            <select
              value={fields.payment_method}
              onChange={(e) => set("payment_method", e.target.value)}
              className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
            >
              {PAYMENT_METHODS.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <label className="block text-sm">
            Amount ({symbol})
            <input
              required
              inputMode="decimal"
              value={amountText}
              onChange={(e) => setAmountText(e.target.value)}
              className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
            />
          </label>
          <label className="block text-sm">
            Tax amount ({symbol})
            <input
              inputMode="decimal"
              value={taxText}
              onChange={(e) => setTaxText(e.target.value)}
              className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
            />
          </label>
        </div>

        <div className="flex gap-6">
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={fields.deductible} onChange={(e) => set("deductible", e.target.checked)} />
            Deductible
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={fields.itc_eligible} onChange={(e) => set("itc_eligible", e.target.checked)} />
            ITC-eligible
          </label>
        </div>
        <p className="text-xs text-slate-500">
          Deductible and ITC-eligible are what you record, not a statutory determination — this app does not decide legal tax
          eligibility.
        </p>

        <label className="block text-sm">
          Notes
          <textarea
            value={fields.notes ?? ""}
            onChange={(e) => set("notes", e.target.value || null)}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
            rows={2}
          />
        </label>

        <div className="flex gap-2 pt-2">
          <button
            type="submit"
            disabled={submitting || !fields.category_id}
            className="rounded bg-emerald-600 px-4 py-2 font-medium disabled:opacity-50"
          >
            {submitting ? "Saving…" : "Save"}
          </button>
          {saved && (
            <button
              type="button"
              onClick={() => setConfirmingDelete(true)}
              className="rounded border border-red-800 px-4 py-2 text-sm text-red-400"
            >
              Delete
            </button>
          )}
        </div>
      </form>

      {saved && (
        <div className="space-y-2 rounded border border-slate-700 bg-slate-900 p-4">
          <h2 className="text-sm font-semibold text-slate-200">Receipt</h2>
          <ErrorBanner error={receiptError} />
          {saved.receipt_path ? (
            <p className="text-sm text-slate-400">
              Attached: <span className="text-slate-200">{saved.receipt_path}</span>
            </p>
          ) : (
            <p className="text-sm text-slate-500">No receipt attached.</p>
          )}
          <div className="flex gap-2">
            <button
              onClick={() => void handleAttach()}
              disabled={receiptBusy}
              className="rounded border border-slate-700 px-3 py-1.5 text-sm disabled:opacity-50"
            >
              {receiptBusy ? "Working…" : saved.receipt_path ? "Replace" : "Attach"}
            </button>
            {saved.receipt_path && (
              <button
                onClick={() => void handleRemoveReceipt()}
                disabled={receiptBusy}
                className="rounded border border-slate-700 px-3 py-1.5 text-sm disabled:opacity-50"
              >
                Remove
              </button>
            )}
          </div>
        </div>
      )}

      {quickAdd === "vendor" && (
        <Modal onClose={() => setQuickAdd(null)}>
          <VendorForm
            onCancel={() => setQuickAdd(null)}
            onSubmit={async (fields2) => {
              const created = await createVendor(fields2);
              set("vendor_id", created.id);
              setQuickAdd(null);
            }}
          />
        </Modal>
      )}
      {quickAdd === "category" && (
        <Modal onClose={() => setQuickAdd(null)}>
          <CategoryQuickAddForm
            onCancel={() => setQuickAdd(null)}
            onSubmit={async (fields2) => {
              const created = await createCategory(fields2);
              set("category_id", created.id);
              set("deductible", created.default_deductible);
              setQuickAdd(null);
            }}
          />
        </Modal>
      )}

      {confirmingDelete && saved && (
        <Modal onClose={() => setConfirmingDelete(false)}>
          <div className="space-y-3 rounded border border-slate-700 bg-slate-900 p-4">
            <h2 className="text-base font-semibold">Delete this expense?</h2>
            <p className="text-sm text-slate-400">
              This permanently removes the expense and its receipt attachment, if any. This can't be undone.
            </p>
            <div className="flex gap-2 pt-2">
              <button onClick={() => void handleDelete()} className="rounded bg-red-600 px-4 py-2 text-sm font-medium text-white">
                Delete
              </button>
              <button onClick={() => setConfirmingDelete(false)} className="rounded border border-slate-700 px-4 py-2 text-sm">
                Cancel
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
