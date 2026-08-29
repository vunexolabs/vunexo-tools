import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { StatusBadge } from "../../components/StatusBadge";
import { createDraftInvoice } from "../../lib/tauri/commands";
import { useInvoices } from "../../hooks/useInvoices";
import { formatMinorAsRupees, type InvoiceStatus } from "../../lib/tauri/types";

const STATUS_FILTERS: { label: string; value: InvoiceStatus | null }[] = [
  { label: "All", value: null },
  { label: "Draft", value: "DRAFT" },
  { label: "Issued", value: "ISSUED" },
  { label: "Partially Paid", value: "PARTIALLY_PAID" },
  { label: "Paid", value: "PAID" },
  { label: "Cancelled", value: "CANCELLED" },
];

/** ui-ux.md §5 — filter bar, table, "+ New" action, quick row actions. */
export function InvoicesList({ onOpen }: { onOpen: (id: number) => void }) {
  const [statusFilter, setStatusFilter] = useState<InvoiceStatus | null>(null);
  const { invoices, error, cancel, remove, duplicate } = useInvoices(statusFilter);
  const [rowError, setRowError] = useState<unknown>(null);
  const [creating, setCreating] = useState(false);

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
          className="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
        >
          {creating ? "Creating…" : "+ New Invoice"}
        </button>
      </div>

      <div className="flex gap-2 text-sm">
        {STATUS_FILTERS.map((f) => (
          <button
            key={f.label}
            onClick={() => setStatusFilter(f.value)}
            className={`rounded px-2 py-1 ${statusFilter === f.value ? "bg-slate-800" : "text-slate-400 hover:bg-slate-900"}`}
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
          {invoices?.map((inv) => (
            <tr key={inv.id} className="border-t border-slate-800">
              <td className="py-2">
                <button onClick={() => onOpen(inv.id)} className="text-sky-400 hover:underline">
                  {inv.invoice_number ?? `Draft #${inv.id}`}
                </button>
              </td>
              <td className="py-2 text-slate-400">{inv.customer_name ?? "—"}</td>
              <td className="py-2 text-slate-400">{inv.invoice_date}</td>
              <td className="py-2 text-slate-400">₹{formatMinorAsRupees(inv.total_minor)}</td>
              <td className="py-2">
                <StatusBadge status={inv.status} isOverdue={inv.is_overdue} />
              </td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  {inv.status === "DRAFT" && (
                    <button onClick={() => runRowAction(() => remove(inv.id))} className="text-red-400 hover:underline">
                      Delete
                    </button>
                  )}
                  {(inv.status === "ISSUED" || inv.status === "PARTIALLY_PAID" || inv.status === "PAID") && (
                    <button
                      onClick={() => runRowAction(() => cancel(inv.id, null))}
                      className="text-amber-400 hover:underline"
                    >
                      Cancel
                    </button>
                  )}
                  {inv.status !== "DRAFT" && (
                    <button
                      onClick={() => runRowAction(async () => { const d = await duplicate(inv.id); onOpen(d.id); })}
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

      {invoices !== null && invoices.length === 0 && (
        <p className="text-sm text-slate-500">No invoices yet — click "+ New Invoice" to create one.</p>
      )}
    </div>
  );
}
