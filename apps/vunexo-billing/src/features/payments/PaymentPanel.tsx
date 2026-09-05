import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { usePayments } from "../../hooks/usePayments";
import type { InvoiceStatus, Payment, PaymentFields, PaymentMethod } from "../../lib/tauri/types";

/**
 * ui-ux.md §1/§7 — "Record Payment (panel, not a full screen)", opened from
 * the Invoice Editor's `EditIssued` footer, not a route of its own
 * (features/payments/ per ui-ux.md §7). Scoped to one invoice: shows its
 * payment history (editable/deletable regardless of the invoice's own
 * status, per database-schema.md §11) plus a "record a new payment" form,
 * hidden once the invoice is CANCELLED (user-flows.md §6 / database-schema.md §3).
 */

const METHOD_LABELS: Record<PaymentMethod, string> = {
  CASH: "Cash",
  BANK_TRANSFER: "Bank transfer",
  UPI: "UPI",
  CHEQUE: "Cheque",
  OTHER: "Other",
};

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function PaymentRow({
  payment,
  onSave,
  onDelete,
}: {
  payment: Payment;
  onSave: (fields: PaymentFields) => Promise<void>;
  onDelete: () => Promise<void>;
}) {
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const [editing, setEditing] = useState(false);
  const [amountStr, setAmountStr] = useState(formatMinor(payment.amount_minor));
  const [method, setMethod] = useState<PaymentMethod>(payment.method);
  const [paidOn, setPaidOn] = useState(payment.paid_on);
  const [reference, setReference] = useState(payment.reference ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);

  if (!editing) {
    return (
      <tr className="border-t border-zinc-200 dark:border-zinc-800">
        <td className="py-2">{payment.paid_on}</td>
        <td className="py-2">{symbol}{formatMinor(payment.amount_minor)}</td>
        <td className="py-2 text-zinc-500 dark:text-zinc-400">{METHOD_LABELS[payment.method]}</td>
        <td className="py-2 text-zinc-500 dark:text-zinc-400">{payment.reference || "—"}</td>
        <td className="py-2 text-right">
          <button onClick={() => setEditing(true)} className="mr-3 text-blue-600 dark:text-blue-400 transition-colors hover:underline">
            Edit
          </button>
          <button onClick={() => void onDelete()} className="text-red-600 dark:text-red-400 transition-colors hover:underline">
            Delete
          </button>
        </td>
      </tr>
    );
  }

  const submit = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave({
        amount_minor: parseToMinor(amountStr),
        method,
        paid_on: paidOn,
        reference: reference.trim() === "" ? null : reference.trim(),
      });
      setEditing(false);
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <tr className="border-t border-zinc-200 dark:border-zinc-800">
      <td colSpan={5} className="py-2">
        <ErrorBanner error={error} />
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="date"
            value={paidOn}
            onChange={(e) => setPaidOn(e.target.value)}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <input
            value={amountStr}
            onChange={(e) => setAmountStr(e.target.value)}
            placeholder={`Amount ${symbol}`}
            className="w-24 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <select
            value={method}
            onChange={(e) => setMethod(e.target.value as PaymentMethod)}
            className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          >
            {Object.entries(METHOD_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
          <input
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            placeholder="Reference (optional)"
            className="flex-1 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <button onClick={() => void submit()} disabled={saving} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1 text-sm disabled:opacity-50">
            {saving ? "Saving…" : "Save"}
          </button>
          <button onClick={() => setEditing(false)} className="text-sm text-zinc-500 dark:text-zinc-400 transition-colors hover:underline">
            Cancel
          </button>
        </div>
      </td>
    </tr>
  );
}

export function PaymentPanel({
  invoiceId,
  invoiceStatus,
  totalMinor,
  onChanged,
}: {
  invoiceId: number;
  invoiceStatus: InvoiceStatus;
  totalMinor: number;
  onChanged: () => void;
}) {
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const { payments, error, record, update, remove } = usePayments(invoiceId);
  const [amountStr, setAmountStr] = useState("");
  const [method, setMethod] = useState<PaymentMethod>("CASH");
  const [paidOn, setPaidOn] = useState(today());
  const [reference, setReference] = useState("");
  const [recording, setRecording] = useState(false);
  const [recordError, setRecordError] = useState<unknown>(null);

  const amountPaidMinor = (payments ?? []).reduce((sum, p) => sum + p.amount_minor, 0);
  const remainingMinor = totalMinor - amountPaidMinor;
  const canRecordNew = invoiceStatus !== "CANCELLED";

  const startAmount = () => (amountStr === "" ? Math.max(remainingMinor, 0) : parseToMinor(amountStr));
  const isOverpaying = amountStr !== "" && startAmount() > remainingMinor && remainingMinor > 0;

  const submitNew = async () => {
    setRecording(true);
    setRecordError(null);
    try {
      await record({
        invoice_id: invoiceId,
        amount_minor: amountStr === "" ? Math.max(remainingMinor, 0) : parseToMinor(amountStr),
        method,
        paid_on: paidOn,
        reference: reference.trim() === "" ? null : reference.trim(),
      });
      setAmountStr("");
      setReference("");
      onChanged();
    } catch (err) {
      setRecordError(err);
    } finally {
      setRecording(false);
    }
  };

  return (
    <div className="space-y-3 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">Payments</h2>
        <p className="text-sm text-zinc-500 dark:text-zinc-400">
          Paid {symbol}{formatMinor(amountPaidMinor)} of {symbol}{formatMinor(totalMinor)}
          {remainingMinor > 0 && <> · {symbol}{formatMinor(remainingMinor)} remaining</>}
          {remainingMinor < 0 && <span className="text-amber-600 dark:text-amber-400"> · overpaid by {symbol}{formatMinor(-remainingMinor)}</span>}
        </p>
      </div>

      <ErrorBanner error={error} />

      {payments && payments.length > 0 && (
        <table className="w-full text-left text-sm">
          <thead className="text-zinc-500 dark:text-zinc-400">
            <tr>
              <th className="pb-1">Date</th>
              <th className="pb-1">Amount</th>
              <th className="pb-1">Method</th>
              <th className="pb-1">Reference</th>
              <th className="pb-1"></th>
            </tr>
          </thead>
          <tbody>
            {payments.map((p) => (
              <PaymentRow
                key={p.id}
                payment={p}
                onSave={async (fields) => {
                  await update(p.id, fields);
                  onChanged();
                }}
                onDelete={async () => {
                  await remove(p.id);
                  onChanged();
                }}
              />
            ))}
          </tbody>
        </table>
      )}
      {payments && payments.length === 0 && <p className="text-sm text-zinc-400 dark:text-zinc-500">No payments recorded yet.</p>}

      {canRecordNew && (
        <div className="space-y-2 border-t border-zinc-200 dark:border-zinc-800 pt-3">
          <ErrorBanner error={recordError} />
          <div className="flex flex-wrap items-center gap-2">
            <input
              type="date"
              value={paidOn}
              onChange={(e) => setPaidOn(e.target.value)}
              className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
            <input
              value={amountStr}
              onChange={(e) => setAmountStr(e.target.value)}
              placeholder={`Amount ${symbol} (default ${formatMinor(Math.max(remainingMinor, 0))})`}
              className="w-40 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
            <select
              value={method}
              onChange={(e) => setMethod(e.target.value as PaymentMethod)}
              className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            >
              {Object.entries(METHOD_LABELS).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
            <input
              value={reference}
              onChange={(e) => setReference(e.target.value)}
              placeholder="Reference (optional)"
              className="flex-1 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
            />
            <button
              onClick={() => void submitNew()}
              disabled={recording}
              className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1 text-sm font-medium disabled:opacity-50"
            >
              {recording ? "Recording…" : "Record Payment"}
            </button>
          </div>
          {isOverpaying && (
            <p className="text-xs text-amber-600 dark:text-amber-400">
              This is more than the remaining balance — it will be recorded in full and flagged as an overpayment, not clamped.
            </p>
          )}
        </div>
      )}
    </div>
  );
}
