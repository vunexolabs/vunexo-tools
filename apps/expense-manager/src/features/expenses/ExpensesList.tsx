import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { PencilIcon, PlusIcon, TrashIcon } from "../../components/icons";
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
      <div className="page-header">
        <h1 className="text-xl font-semibold">Expenses</h1>
        <button onClick={onNew} className="btn-primary">
          <PlusIcon className="h-4 w-4" />
          New Expense
        </button>
      </div>

      <div className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">
          <span className="text-text-secondary">Category</span>
          <select
            value={filter.category_id ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, category_id: e.target.value ? Number(e.target.value) : null }))}
            className="select"
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
          <span className="text-text-secondary">Vendor</span>
          <select
            value={filter.vendor_id ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, vendor_id: e.target.value ? Number(e.target.value) : null }))}
            className="select"
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
          <span className="text-text-secondary">From</span>
          <input
            type="date"
            value={filter.date_from ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, date_from: e.target.value || null }))}
            className="input"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-text-secondary">To</span>
          <input
            type="date"
            value={filter.date_to ?? ""}
            onChange={(e) => setFilter((f) => ({ ...f, date_to: e.target.value || null }))}
            className="input"
          />
        </label>
        {(filter.category_id || filter.vendor_id || filter.date_from || filter.date_to) && (
          <button onClick={() => setFilter({})} className="btn-ghost btn-sm">
            Clear filters
          </button>
        )}
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <div className="card overflow-hidden">
        <table className="table-base">
          <thead>
            <tr>
              <th className="pl-4">Date</th>
              <th>Vendor</th>
              <th>Category</th>
              <th className="text-right">Amount</th>
              <th>Deductible</th>
              <th className="pr-4"></th>
            </tr>
          </thead>
          <tbody>
            {expenses?.map((e) => (
              <tr key={e.id} className="is-hoverable">
                <td className="pl-4">{e.date}</td>
                <td className="text-text-secondary">{e.vendor_name_snapshot ?? "—"}</td>
                <td className="text-text-secondary">{e.category_name_snapshot}</td>
                <td className="tabular-nums text-right">
                  {symbol}
                  {formatMinor(e.amount)}
                </td>
                <td className="text-text-secondary">
                  <span className={e.deductible ? "badge-success" : "badge-neutral"}>{e.deductible ? "Yes" : "No"}</span>
                </td>
                <td className="pr-4 text-right">
                  <div className="flex justify-end gap-3">
                    <button onClick={() => onOpen(e)} className="link inline-flex items-center gap-1">
                      <PencilIcon className="h-3.5 w-3.5" />
                      Edit
                    </button>
                    <button
                      onClick={() => setDeleteTarget(e)}
                      className="inline-flex items-center gap-1 text-sm text-danger hover:underline"
                    >
                      <TrashIcon className="h-3.5 w-3.5" />
                      Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {expenses !== null && expenses.length === 0 && (
          <p className="px-4 py-6 text-center text-sm text-text-muted">No expenses match these filters.</p>
        )}
      </div>

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
