import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { useDashboard } from "../../hooks/useDashboard";
import type { Expense } from "../../lib/tauri/types";

/**
 * user-flows.md §8 — landing screen: this period's total spend, a category
 * breakdown, a recent-expenses list. Clicking a category row filters the
 * Expenses list to that category (mirrors Billing's Overdue-card
 * click-through pattern).
 */
export function Dashboard({
  onOpenCategory,
  onOpenExpense,
}: {
  onOpenCategory: (categoryId: number) => void;
  onOpenExpense: (expense: Expense) => void;
}) {
  const { symbol, formatMinor } = useCurrency();
  const { metrics, error } = useDashboard();

  return (
    <div className="space-y-6">
      <div className="page-header">
        <h1 className="text-xl font-semibold">Dashboard</h1>
      </div>
      <ErrorBanner error={error} />

      {metrics && (
        <>
          <div className="card p-5">
            <p className="text-sm text-text-secondary">This period's spend</p>
            <p className="tabular-nums mt-1 text-3xl font-semibold">
              {symbol}
              {formatMinor(metrics.period_total_minor)}
            </p>
          </div>

          <div className="card">
            <h2 className="border-b border-border px-4 py-3 text-sm font-semibold">Category breakdown</h2>
            {metrics.category_breakdown.length === 0 ? (
              <p className="px-4 py-4 text-sm text-text-muted">No expenses recorded this period.</p>
            ) : (
              <table className="table-base">
                <tbody>
                  {metrics.category_breakdown.map((row) => (
                    <tr key={row.category_id} className="is-hoverable">
                      <td className="pl-4">
                        <button onClick={() => onOpenCategory(row.category_id)} className="link">
                          {row.category_name}
                        </button>
                      </td>
                      <td className="tabular-nums pr-4 text-right">
                        {symbol}
                        {formatMinor(row.total_minor)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="card">
            <h2 className="border-b border-border px-4 py-3 text-sm font-semibold">Recent expenses</h2>
            {metrics.recent_expenses.length === 0 ? (
              <p className="px-4 py-4 text-sm text-text-muted">No expenses yet.</p>
            ) : (
              <table className="table-base">
                <thead>
                  <tr>
                    <th className="pl-4">Date</th>
                    <th>Vendor</th>
                    <th>Category</th>
                    <th className="pr-4 text-right">Amount</th>
                  </tr>
                </thead>
                <tbody>
                  {metrics.recent_expenses.map((e) => (
                    <tr key={e.id} className="is-hoverable cursor-pointer" onClick={() => onOpenExpense(e)}>
                      <td className="pl-4">{e.date}</td>
                      <td className="text-text-secondary">{e.vendor_name_snapshot ?? "—"}</td>
                      <td className="text-text-secondary">{e.category_name_snapshot}</td>
                      <td className="tabular-nums pr-4 text-right">
                        {symbol}
                        {formatMinor(e.amount)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </div>
  );
}
