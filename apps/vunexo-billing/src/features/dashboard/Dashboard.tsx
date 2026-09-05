import { ErrorBanner } from "../../components/ErrorBanner";
import { StatusBadge } from "../../components/StatusBadge";
import { useCurrency } from "../../hooks/useCurrency";
import { useDashboard } from "../../hooks/useDashboard";

/**
 * ui-ux.md §1/§8 — the default landing screen: today/month/outstanding/
 * paid/overdue metrics plus a recent-invoices list. Every recent-invoice
 * row is clickable through to the invoice detail per user-flows.md §8.
 *
 * Only the Overdue card is clickable through to a filtered Invoices List:
 * it's the one metric with an exact status-based equivalent (the same
 * `is_overdue` flag the Invoices List's "Overdue" filter already uses). The
 * sales/outstanding/paid cards are date- or multi-status-scoped sums with no
 * matching `InvoiceFilter` value, so linking them would land on a list that
 * doesn't actually match the number shown.
 */
export function Dashboard({
  onOpenInvoice,
  onOpenOverdueInvoices,
}: {
  onOpenInvoice: (id: number) => void;
  onOpenOverdueInvoices: () => void;
}) {
  const { symbol, formatMinor } = useCurrency();
  const { metrics, error } = useDashboard();

  if (!metrics) {
    return (
      <div className="space-y-4">
        <h1 className="text-xl font-semibold">Dashboard</h1>
        <ErrorBanner error={error} />
        {!error && <p className="text-zinc-400 dark:text-zinc-500">Loading…</p>}
      </div>
    );
  }

  const cards = [
    { label: "Today's sales", value: metrics.today_sales_minor },
    { label: "This month's sales", value: metrics.month_sales_minor },
    { label: "Outstanding", value: metrics.outstanding_total_minor },
    { label: "Paid this month", value: metrics.paid_total_minor },
  ];

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">Dashboard</h1>
      <ErrorBanner error={error} />

      <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
        {cards.map((c) => (
          <div key={c.label} className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
            <p className="text-xs text-zinc-500 dark:text-zinc-400">{c.label}</p>
            <p className="mt-1 text-lg font-semibold">{symbol}{formatMinor(c.value)}</p>
          </div>
        ))}
        <button
          onClick={onOpenOverdueInvoices}
          className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4 text-left transition-colors hover:bg-zinc-100 dark:hover:bg-zinc-800"
        >
          <p className="text-xs text-zinc-500 dark:text-zinc-400">Overdue</p>
          <p className="mt-1 text-lg font-semibold text-red-600 dark:text-red-400">
            {metrics.overdue.count} · {symbol}{formatMinor(metrics.overdue.total_minor)}
          </p>
        </button>
      </div>

      <div>
        <h2 className="mb-2 text-sm font-semibold text-zinc-900 dark:text-zinc-100">Recent invoices</h2>
        <table className="w-full text-left text-sm">
          <thead className="text-zinc-500 dark:text-zinc-400">
            <tr>
              <th className="pb-2">Number</th>
              <th className="pb-2">Customer</th>
              <th className="pb-2">Date</th>
              <th className="pb-2">Total</th>
              <th className="pb-2">Status</th>
            </tr>
          </thead>
          <tbody>
            {metrics.recent_invoices.map((inv) => (
              <tr key={inv.id} className="border-t border-zinc-200 dark:border-zinc-800">
                <td className="py-2">
                  <button onClick={() => onOpenInvoice(inv.id)} className="text-blue-600 dark:text-blue-400 transition-colors hover:underline">
                    {inv.invoice_number ?? `Draft #${inv.id}`}
                  </button>
                </td>
                <td className="py-2 text-zinc-500 dark:text-zinc-400">{inv.customer_name ?? "—"}</td>
                <td className="py-2 text-zinc-500 dark:text-zinc-400">{inv.invoice_date}</td>
                <td className="py-2 text-zinc-500 dark:text-zinc-400">{symbol}{formatMinor(inv.total_minor)}</td>
                <td className="py-2">
                  <StatusBadge status={inv.status} isOverdue={inv.is_overdue} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {metrics.recent_invoices.length === 0 && <p className="text-sm text-zinc-400 dark:text-zinc-500">No invoices yet.</p>}
      </div>
    </div>
  );
}
