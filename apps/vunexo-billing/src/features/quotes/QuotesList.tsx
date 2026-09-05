import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { QuoteStatusBadge } from "../../components/StatusBadge";
import { createDraftQuote } from "../../lib/tauri/commands";
import { useCurrency } from "../../hooks/useCurrency";
import { useQuotes } from "../../hooks/useQuotes";
import type { QuoteStatus } from "../../lib/tauri/types";

export type QuoteFilterOption = QuoteStatus | null;

const STATUS_FILTERS: { label: string; value: QuoteFilterOption }[] = [
  { label: "All", value: null },
  { label: "Draft", value: "DRAFT" },
  { label: "Issued", value: "ISSUED" },
  { label: "Accepted", value: "ACCEPTED" },
  { label: "Declined", value: "DECLINED" },
  { label: "Converted", value: "CONVERTED" },
  { label: "Cancelled", value: "CANCELLED" },
];

/**
 * ui-ux-v2.md §2 — filter bar, table, "+ New" action, mirroring
 * InvoicesList.tsx's shape. Full lifecycle actions (Accept/Decline/Convert)
 * live in the Quote Editor, not as list quick-actions — reviewing a quote
 * up close before accepting/converting it is the point, not a shortcut
 * worth skipping that review for.
 */
export function QuotesList({ onOpen }: { onOpen: (id: number) => void }) {
  const { symbol, formatMinor } = useCurrency();
  const [filter, setFilter] = useState<QuoteFilterOption>(null);
  const { quotes, error, cancel, remove, duplicate } = useQuotes(filter);
  const [rowError, setRowError] = useState<unknown>(null);
  const [creating, setCreating] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<number | null>(null);
  const [cancelTargetId, setCancelTargetId] = useState<number | null>(null);
  const [cancelReason, setCancelReason] = useState("");

  const runRowAction = async (action: () => Promise<void>) => {
    setRowError(null);
    try {
      await action();
    } catch (err) {
      setRowError(err);
    }
  };

  const handleNewQuote = async () => {
    setCreating(true);
    setRowError(null);
    try {
      const draft = await createDraftQuote({
        customer_id: null,
        quote_date: new Date().toISOString().slice(0, 10),
        valid_until: null,
        notes: null,
        terms: null,
        is_interstate: false,
        discount_type: null,
        discount_value: null,
        line_items: [],
      });
      onOpen(draft.id);
    } catch (err) {
      setRowError(err);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Quotes</h1>
        <button
          onClick={handleNewQuote}
          disabled={creating}
          className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
        >
          {creating ? "Creating…" : "+ New Quote"}
        </button>
      </div>

      <div className="flex gap-2 text-sm">
        {STATUS_FILTERS.map((f) => (
          <button
            key={f.label}
            onClick={() => setFilter(f.value)}
            className={`rounded-md px-2 py-1 transition-colors ${filter === f.value ? "bg-blue-50 text-blue-700 dark:bg-blue-500/10 dark:text-blue-400" : "text-zinc-500 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800"}`}
          >
            {f.label}
          </button>
        ))}
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <div className="overflow-hidden rounded-lg border border-zinc-200 dark:border-zinc-800">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 bg-zinc-50 text-zinc-500 dark:border-zinc-800 dark:bg-zinc-950/50 dark:text-zinc-400">
            <tr>
              <th className="px-4 py-2.5 font-medium">Number</th>
              <th className="px-4 py-2.5 font-medium">Customer</th>
              <th className="px-4 py-2.5 font-medium">Date</th>
              <th className="px-4 py-2.5 font-medium">Total</th>
              <th className="px-4 py-2.5 font-medium">Status</th>
              <th className="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody className="bg-white dark:bg-zinc-900">
            {quotes?.map((q) => (
              <tr key={q.id} className="border-t border-zinc-200 dark:border-zinc-800">
                <td className="px-4 py-2.5">
                  <button onClick={() => onOpen(q.id)} className="text-blue-600 dark:text-blue-400 transition-colors hover:underline">
                    {q.quote_number ?? `Draft #${q.id}`}
                  </button>
                </td>
                <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{q.customer_name ?? "—"}</td>
                <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{q.quote_date}</td>
                <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">
                  {symbol}
                  {formatMinor(q.total_minor)}
                </td>
                <td className="px-4 py-2.5">
                  <QuoteStatusBadge status={q.status} isExpired={q.is_expired} />
                </td>
                <td className="px-4 py-2.5 text-right">
                  <div className="flex flex-wrap justify-end gap-1">
                    {q.status === "DRAFT" && (
                      <button
                        onClick={() => setDeleteTargetId(q.id)}
                        className="rounded-md px-2 py-1 text-xs font-medium text-red-600 transition-colors hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
                      >
                        Delete
                      </button>
                    )}
                    {(q.status === "ISSUED" || q.status === "ACCEPTED") && (
                      <button
                        onClick={() => {
                          setCancelReason("");
                          setCancelTargetId(q.id);
                        }}
                        className="rounded-md px-2 py-1 text-xs font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-500/10"
                      >
                        Cancel
                      </button>
                    )}
                    {q.status !== "DRAFT" && (
                      <button
                        onClick={() =>
                          runRowAction(async () => {
                            const d = await duplicate(q.id);
                            onOpen(d.id);
                          })
                        }
                        className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-500/10"
                      >
                        Duplicate
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {quotes !== null && quotes.length === 0 && (
        <p className="text-sm text-zinc-400 dark:text-zinc-500">No quotes yet — click "+ New Quote" to create one.</p>
      )}

      {deleteTargetId !== null && (
        <ConfirmDialog
          title="Delete this draft?"
          message="This permanently deletes the draft and its line items. This can't be undone."
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteTargetId(null)}
          onConfirm={async () => {
            await runRowAction(() => remove(deleteTargetId));
            setDeleteTargetId(null);
          }}
        />
      )}

      {cancelTargetId !== null && (
        <ConfirmDialog
          title="Cancel this quote?"
          message="Cancelling is terminal — a cancelled quote can't be edited, accepted, or converted again."
          confirmLabel="Cancel Quote"
          danger
          onCancel={() => setCancelTargetId(null)}
          onConfirm={async () => {
            await runRowAction(() => cancel(cancelTargetId, cancelReason.trim() || null));
            setCancelTargetId(null);
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
