import { Fragment, useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { StatusBadge } from "../../components/StatusBadge";
import { PaymentPanel } from "../payments/PaymentPanel";
import { ReminderModal } from "../reminders/ReminderModal";
import { InvoicePdfPreview } from "./InvoicePdfPreview";
import { createDraftInvoice } from "../../lib/tauri/commands";
import { useCurrency } from "../../hooks/useCurrency";
import { useInvoices } from "../../hooks/useInvoices";
import { useInvoicePdf } from "../../hooks/useInvoicePdf";
import type { InvoiceStatus } from "../../lib/tauri/types";

export type FilterOption = InvoiceStatus | "OVERDUE" | null;

const STATUS_FILTERS: { label: string; value: FilterOption }[] = [
  { label: "All", value: null },
  { label: "Draft", value: "DRAFT" },
  { label: "Issued", value: "ISSUED" },
  { label: "Partially Paid", value: "PARTIALLY_PAID" },
  { label: "Paid", value: "PAID" },
  { label: "Overdue", value: "OVERDUE" },
  { label: "Cancelled", value: "CANCELLED" },
];

/**
 * ui-ux.md §5 — filter bar, table, "+ New" action, quick row actions.
 * `OVERDUE` is a derived pseudo-status (database-schema.md §8) — it isn't a
 * stored `InvoiceFilter.status` value the backend can query by, so it's
 * filtered client-side over the already-computed `is_overdue` flag every
 * row already carries, rather than added as a fake stored status.
 *
 * `filter`/`onFilterChange` are lifted into `App` (rather than local state)
 * so the Dashboard's Overdue card can land here pre-filtered.
 */
export function InvoicesList({
  onOpen,
  filter,
  onFilterChange,
}: {
  onOpen: (id: number) => void;
  filter: FilterOption;
  onFilterChange: (filter: FilterOption) => void;
}) {
  const { symbol, formatMinor } = useCurrency();
  const queryStatus = filter === "OVERDUE" ? null : filter;
  const { invoices: fetchedInvoices, error, cancel, remove, duplicate, reload } = useInvoices(queryStatus);
  const invoices = filter === "OVERDUE" ? (fetchedInvoices?.filter((i) => i.is_overdue) ?? null) : fetchedInvoices;
  const [rowError, setRowError] = useState<unknown>(null);
  const [creating, setCreating] = useState(false);
  const [paymentPanelFor, setPaymentPanelFor] = useState<number | null>(null);
  const [deleteTargetId, setDeleteTargetId] = useState<number | null>(null);
  const [cancelTargetId, setCancelTargetId] = useState<number | null>(null);
  const [cancelReason, setCancelReason] = useState("");
  // user-flows.md §7: the PDF action is available on an existing invoice from
  // the list, not only from inside the editor.
  const [pdfInvoice, setPdfInvoice] = useState<{ id: number; number: string | null } | null>(null);
  const pdf = useInvoicePdf();
  const [remindInvoice, setRemindInvoice] = useState<{ id: number; number: string | null } | null>(null);

  const runRowAction = async (action: () => Promise<void>) => {
    setRowError(null);
    try {
      await action();
    } catch (err) {
      setRowError(err);
    }
  };

  const handleNewInvoice = async () => {
    setCreating(true);
    setRowError(null);
    try {
      const draft = await createDraftInvoice({
        customer_id: null,
        invoice_date: new Date().toISOString().slice(0, 10),
        due_date: null,
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
        <h1 className="text-xl font-semibold">Invoices</h1>
        <button
          onClick={handleNewInvoice}
          disabled={creating}
          className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
        >
          {creating ? "Creating…" : "+ New Invoice"}
        </button>
      </div>

      <div className="flex gap-2 text-sm">
        {STATUS_FILTERS.map((f) => (
          <button
            key={f.label}
            onClick={() => onFilterChange(f.value)}
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
            {invoices?.map((inv) => (
              <Fragment key={inv.id}>
                <tr className="border-t border-zinc-200 dark:border-zinc-800">
                  <td className="px-4 py-2.5">
                    <button onClick={() => onOpen(inv.id)} className="text-blue-600 dark:text-blue-400 transition-colors hover:underline">
                      {inv.invoice_number ?? `Draft #${inv.id}`}
                    </button>
                  </td>
                  <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{inv.customer_name ?? "—"}</td>
                  <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{inv.invoice_date}</td>
                  <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{symbol}{formatMinor(inv.total_minor)}</td>
                  <td className="px-4 py-2.5">
                    <StatusBadge status={inv.status} isOverdue={inv.is_overdue} />
                  </td>
                  <td className="px-4 py-2.5 text-right">
                    <div className="flex flex-wrap justify-end gap-1">
                      {inv.status === "DRAFT" && (
                        <button
                          onClick={() => setDeleteTargetId(inv.id)}
                          className="rounded-md px-2 py-1 text-xs font-medium text-red-600 transition-colors hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
                        >
                          Delete
                        </button>
                      )}
                      {(inv.status === "ISSUED" || inv.status === "PARTIALLY_PAID") && (
                        <button
                          onClick={() => setPaymentPanelFor(paymentPanelFor === inv.id ? null : inv.id)}
                          className="rounded-md px-2 py-1 text-xs font-medium text-green-600 transition-colors hover:bg-green-50 dark:text-green-400 dark:hover:bg-green-500/10"
                        >
                          {paymentPanelFor === inv.id ? "Close" : "Record Payment"}
                        </button>
                      )}
                      {(inv.status === "ISSUED" || inv.status === "PARTIALLY_PAID" || inv.status === "PAID") && (
                        <button
                          onClick={() => {
                            setCancelReason("");
                            setCancelTargetId(inv.id);
                          }}
                          className="rounded-md px-2 py-1 text-xs font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-500/10"
                        >
                          Cancel
                        </button>
                      )}
                      {inv.is_overdue && (
                        <button
                          onClick={() => setRemindInvoice({ id: inv.id, number: inv.invoice_number })}
                          className="rounded-md px-2 py-1 text-xs font-medium text-red-600 transition-colors hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
                        >
                          Remind
                        </button>
                      )}
                      {inv.status !== "DRAFT" && (
                        <button
                          onClick={() => {
                            setPdfInvoice({ id: inv.id, number: inv.invoice_number });
                            void pdf.preview(inv.id);
                          }}
                          disabled={pdf.busy}
                          className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 transition-colors hover:bg-blue-50 disabled:opacity-50 dark:text-blue-400 dark:hover:bg-blue-500/10"
                        >
                          PDF
                        </button>
                      )}
                      {inv.status !== "DRAFT" && (
                        <button
                          onClick={() => runRowAction(async () => { const d = await duplicate(inv.id); onOpen(d.id); })}
                          className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-500/10"
                        >
                          Duplicate
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
                {paymentPanelFor === inv.id && (
                  <tr className="border-t border-zinc-200 dark:border-zinc-800">
                    <td colSpan={6} className="px-4 py-2.5">
                      <PaymentPanel
                        invoiceId={inv.id}
                        invoiceStatus={inv.status}
                        totalMinor={inv.total_minor}
                        onChanged={reload}
                      />
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>

      <ErrorBanner error={pdf.error} />

      {pdf.previewUrl && pdfInvoice && (
        <InvoicePdfPreview
          url={pdf.previewUrl}
          title={pdf.previewTitle}
          saving={pdf.busy}
          onClose={() => {
            pdf.closePreview();
            setPdfInvoice(null);
          }}
          onSave={() =>
            void pdf.saveAs(pdfInvoice.id, pdf.suggestedFileName(pdfInvoice.number, pdfInvoice.id))
          }
        />
      )}

      {remindInvoice && (
        <ReminderModal invoiceId={remindInvoice.id} invoiceNumber={remindInvoice.number} onClose={() => setRemindInvoice(null)} />
      )}

      {invoices !== null && invoices.length === 0 && (
        <p className="text-sm text-zinc-400 dark:text-zinc-500">No invoices yet — click "+ New Invoice" to create one.</p>
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
          title="Cancel this invoice?"
          message="Cancelling is terminal — a cancelled invoice can't be edited or issued again."
          confirmLabel="Cancel Invoice"
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
