import { useState } from "react";
import { Modal } from "./Modal";
import { ErrorBanner } from "./ErrorBanner";

/**
 * ui-ux.md §3/§7 — the shared confirmation dialog named alongside status
 * badge / searchable picker / error banner as a cross-cutting component,
 * required before `CancelInvoice` (optional reason, via `children`),
 * `DeleteDraftInvoice`, and `restore_backup`. Also used here for the
 * customer/product hard-delete actions — irreversible and one misclick
 * away otherwise, even though the locked spec doesn't name them explicitly.
 */
export function ConfirmDialog({
  title,
  message,
  confirmLabel = "Confirm",
  danger = false,
  onConfirm,
  onCancel,
  children,
}: {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => Promise<unknown>;
  onCancel: () => void;
  children?: React.ReactNode;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const handleConfirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await onConfirm();
    } catch (err) {
      setError(err);
      setBusy(false);
    }
  };

  return (
    <Modal onClose={onCancel}>
      <div className="space-y-3 rounded-lg border border-zinc-200 bg-white p-4 shadow-lg dark:border-zinc-800 dark:bg-zinc-900">
        <h2 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">{title}</h2>
        <p className="text-sm text-zinc-500 dark:text-zinc-400">{message}</p>
        <ErrorBanner error={error} />
        {children}
        <div className="flex gap-2 pt-2">
          <button
            onClick={() => void handleConfirm()}
            disabled={busy}
            className={`rounded-md px-4 py-2 text-sm font-medium text-white transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 dark:focus:ring-offset-zinc-900 ${
              danger
                ? "bg-red-600 hover:bg-red-700 focus:ring-red-500"
                : "bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600"
            }`}
          >
            {busy ? "Working…" : confirmLabel}
          </button>
          <button
            onClick={onCancel}
            disabled={busy}
            className="rounded-md border border-zinc-300 px-4 py-2 text-sm text-zinc-700 transition-colors hover:bg-zinc-50 disabled:opacity-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
          >
            Cancel
          </button>
        </div>
      </div>
    </Modal>
  );
}
