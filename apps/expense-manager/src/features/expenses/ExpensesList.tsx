import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCategories } from "../../hooks/useCategories";
import { useCurrency } from "../../hooks/useCurrency";
import { useExpenses } from "../../hooks/useExpenses";
import { useVendors } from "../../hooks/useVendors";
import type { Expense, ExpenseFilter } from "../../lib/tauri/types";

/**
 * ui-ux.md §5 — Expenses List: filterable table (category/vendor/date-range,
 * the same axes reports use), row actions, a primary "New" action. The
 * category/vendor filters double as the dashboard's category-row
 * click-through target (`initialFilter`).
 */
export function ExpensesList({
  onOpen,
  onNew,
  initialFilter,
}: {
  onOpen: (expense: Expense) => void;
  onNew: () => void;
  initialFilter?: ExpenseFilter;
}) {
  const { symbol, formatMinor } = useCurrency();
  const [filter, setFilter] = useState<ExpenseFilter>(initialFilter ?? {});
  const { expenses, error, remove } = useExpenses(filter);
  const { vendors } = useVendors();
  const { categories } = useCategories();
  const [rowError, setRowError] = useState<unknown>(null);
  const [deleteTarget, setDeleteTarget] = useState<Expense | null>(null);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Expenses</h1>
        <button onClick={onNew} className="rounded bg-emerald-600 px-3 py-1.5 text-sm font-medium">
          + New Expense
        </button>
      </div>

      <div className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">
          Category
          <select
            value={filter.category_id ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, category_id: e.target.value ? Number(e.target.value) : null }))}
            className="rounded border border-slate-700 bg-slate-950 px-2 py-1"
          >
            <option value="">All</option>
            {categories?.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1">
          Vendor
          <select
            value={filter.vendor_id ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, vendor_id: e.target.value ? Number(e.target.value) : null }))}
            className="rounded border border-slate-700 bg-slate-950 px-2 py-1"
          >
            <option value="">All</option>
            {vendors?.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1">
          From
          <input
            type="date"
            value={filter.date_from ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, date_from: e.target.value || null }))}
            className="rounded border border-slate-700 bg-slate-950 px-2 py-1"
          />
        </label>
        <label className="flex flex-col gap-1">
          To
          <input
            type="date"
            value={filter.date_to ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, date_to: e.target.value || null }))}
            className="rounded border border-slate-700 bg-slate-950 px-2 py-1"
          />
        </label>
        {(filter.category_id || filter.vendor_id || filter.date_from || filter.date_to) && (
          <button onClick={() => setFilter({})} className="text-sm text-slate-400 hover:underline">
            Clear filters
          </button>
        )}
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <table className="w-full text-left text-sm">
        <thead className="text-slate-400">
          <tr>
            <th className="pb-2">Date</th>
            <th className="pb-2">Vendor</th>
            <th className="pb-2">Category</th>
            <th className="pb-2">Amount</th>
            <th className="pb-2">Deductible</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {expenses?.map((e) => (
            <tr key={e.id} className="border-t border-slate-800">
              <td className="py-2">{e.date}</td>
              <td className="py-2 text-slate-400">{e.vendor_name_snapshot ?? "—"}</td>
              <td className="py-2 text-slate-400">{e.category_name_snapshot}</td>
              <td className="py-2">
                {symbol}
                {formatMinor(e.amount)}
              </td>
              <td className="py-2 text-slate-400">{e.deductible ? "Yes" : "No"}</td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  <button onClick={() => onOpen(e)} className="text-sky-400 hover:underline">
                    Edit
                  </button>
                  <button onClick={() => setDeleteTarget(e)} className="text-red-400 hover:underline">
                    Delete
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {expenses !== null && expenses.length === 0 && (
        <p className="text-sm text-slate-500">No expenses match these filters.</p>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="Delete this expense?"
          message="This permanently removes the expense and its receipt attachment, if any. This can't be undone."
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={async () => {
            setRowError(null);
            try {
              await remove(deleteTarget.id);
            } catch (err) {
              setRowError(err);
            }
            setDeleteTarget(null);
          }}
        />
      )}
    </div>
  );
}
