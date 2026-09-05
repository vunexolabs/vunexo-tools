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
      <h1 className="text-xl font-semibold">Dashboard</h1>
      <ErrorBanner error={error} />

      {metrics && (
        <>
          <div className="rounded border border-slate-800 bg-slate-900 p-4">
            <p className="text-sm text-slate-400">This period's spend</p>
            <p className="text-3xl font-semibold">
              {symbol}
              {formatMinor(metrics.period_total_minor)}
            </p>
          </div>

          <div className="space-y-2">
            <h2 className="text-sm font-semibold text-slate-200">Category breakdown</h2>
            {metrics.category_breakdown.length === 0 ? (
              <p className="text-sm text-slate-500">No expenses recorded this period.</p>
            ) : (
              <table className="w-full text-left text-sm">
                <tbody>
                  {metrics.category_breakdown.map((row) => (
                    <tr key={row.category_id} className="border-t border-slate-800">
                      <td className="py-2">
                        <button
                          onClick={() => onOpenCategory(row.category_id)}
                          className="text-sky-400 hover:underline"
                        >
                          {row.category_name}
                        </button>
                      </td>
                      <td className="py-2 text-right">
                        {symbol}
                        {formatMinor(row.total_minor)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          <div className="space-y-2">
            <h2 className="text-sm font-semibold text-slate-200">Recent expenses</h2>
            {metrics.recent_expenses.length === 0 ? (
              <p className="text-sm text-slate-500">No expenses yet.</p>
            ) : (
              <table className="w-full text-left text-sm">
                <thead className="text-slate-400">
                  <tr>
                    <th className="pb-2">Date</th>
                    <th className="pb-2">Vendor</th>
                    <th className="pb-2">Category</th>
                    <th className="pb-2">Amount</th>
                  </tr>
                </thead>
                <tbody>
                  {metrics.recent_expenses.map((e) => (
                    <tr key={e.id} className="cursor-pointer border-t border-slate-800 hover:bg-slate-900" onClick={() => onOpenExpense(e)}>
                      <td className="py-2">{e.date}</td>
                      <td className="py-2 text-slate-400">{e.vendor_name_snapshot ?? "—"}</td>
                      <td className="py-2 text-slate-400">{e.category_name_snapshot}</td>
                      <td className="py-2">
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
