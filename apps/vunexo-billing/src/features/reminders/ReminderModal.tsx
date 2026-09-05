import { useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { Modal } from "../../components/Modal";
import { generateReminderMessage } from "../../lib/tauri/commands";

/**
 * ui-ux-v2.md §7 — one `generate_reminder_message` call on open, editable
 * before either action, no send button (locked no-delivery-mechanism
 * decision) — closing discards edits, nothing is persisted. "Print / Save
 * PDF" is the OS print dialog scoped to just this text via `@media print`
 * (below) rather than a generated PDF file — there's no reminder PDF
 * renderer in the backend, and the OS dialog's own "Save as PDF" already
 * covers the "Save PDF" half of the one button the design names.
 */
export function ReminderModal({ invoiceId, invoiceNumber, onClose }: { invoiceId: number; invoiceNumber: string | null; onClose: () => void }) {
  const [message, setMessage] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    generateReminderMessage(invoiceId)
      .then((msg) => {
        setMessage(msg);
        setLoading(false);
      })
      .catch((err: unknown) => {
        setError(err);
        setLoading(false);
      });
  }, [invoiceId]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Modal onClose={onClose}>
      <style>{`
        .reminder-print-only { display: none; }
        @media print {
          body * { visibility: hidden; }
          #reminder-print-portal, #reminder-print-portal * { visibility: visible; }
          #reminder-print-portal {
            position: absolute; top: 0; left: 0; width: 100%;
            background: white; padding: 2rem;
          }
          #reminder-print-portal > *:not(.reminder-print-only) { display: none; }
          .reminder-print-only {
            display: block; color: black; white-space: pre-wrap;
            font-family: sans-serif;
          }
        }
      `}</style>
      <div id="reminder-print-portal" className="w-full max-w-lg space-y-3 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
        <h2 className="text-base font-semibold">Payment Reminder{invoiceNumber ? ` — ${invoiceNumber}` : ""}</h2>
        <ErrorBanner error={error} />
        {loading ? (
          <p className="text-sm text-zinc-400 dark:text-zinc-500">Loading…</p>
        ) : (
          <textarea
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            rows={12}
            className="w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
        )}
        <pre className="reminder-print-only">{message}</pre>
        <div className="flex flex-wrap gap-2 pt-2">
          <button onClick={() => void handleCopy()} disabled={loading} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 text-sm font-medium disabled:opacity-50">
            {copied ? "Copied!" : "Copy to Clipboard"}
          </button>
          <button onClick={() => window.print()} disabled={loading} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 text-sm disabled:opacity-50">
            Print / Save PDF
          </button>
          <button onClick={onClose} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2 text-sm">
            Close
          </button>
        </div>
      </div>
    </Modal>
  );
}
