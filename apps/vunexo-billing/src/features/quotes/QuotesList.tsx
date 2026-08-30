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
          className="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
        >
          {creating ? "Creating…" : "+ New Quote"}
        </button>
      </div>

      <div className="flex gap-2 text-sm">
        {STATUS_FILTERS.map((f) => (
          <button
            key={f.label}
            onClick={() => setFilter(f.value)}
            className={`rounded px-2 py-1 ${filter === f.value ? "bg-slate-800" : "text-slate-400 hover:bg-slate-900"}`}
          >
            {f.label}
          </button>
        ))}
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <table className="w-full text-left text-sm">
        <thead className="text-slate-400">
          <tr>
            <th className="pb-2">Number</th>
            <th className="pb-2">Customer</th>
            <th className="pb-2">Date</th>
            <th className="pb-2">Total</th>
            <th className="pb-2">Status</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {quotes?.map((q) => (
            <tr key={q.id} className="border-t border-slate-800">
              <td className="py-2">
                <button onClick={() => onOpen(q.id)} className="text-sky-400 hover:underline">
                  {q.quote_number ?? `Draft #${q.id}`}
                </button>
              </td>
              <td className="py-2 text-slate-400">{q.customer_name ?? "—"}</td>
              <td className="py-2 text-slate-400">{q.quote_date}</td>
              <td className="py-2 text-slate-400">
                {symbol}
                {formatMinor(q.total_minor)}
              </td>
              <td className="py-2">
                <QuoteStatusBadge status={q.status} isExpired={q.is_expired} />
              </td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  {q.status === "DRAFT" && (
                    <button onClick={() => setDeleteTargetId(q.id)} className="text-red-400 hover:underline">
                      Delete
                    </button>
                  )}
                  {(q.status === "ISSUED" || q.status === "ACCEPTED") && (
                    <button
                      onClick={() => {
                        setCancelReason("");
                        setCancelTargetId(q.id);
                      }}
                      className="text-amber-400 hover:underline"
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
                      className="text-sky-400 hover:underline"
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

      {quotes !== null && quotes.length === 0 && (
        <p className="text-sm text-slate-500">No quotes yet — click "+ New Quote" to create one.</p>
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
              className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
              rows={2}
            />
          </label>
        </ConfirmDialog>
      )}
    </div>
  );
}
