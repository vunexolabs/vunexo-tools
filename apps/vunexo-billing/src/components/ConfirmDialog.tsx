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
      <div className="space-y-3 rounded border border-slate-700 bg-slate-900 p-4">
        <h2 className="text-base font-semibold">{title}</h2>
        <p className="text-sm text-slate-400">{message}</p>
        <ErrorBanner error={error} />
        {children}
        <div className="flex gap-2 pt-2">
          <button
            onClick={() => void handleConfirm()}
            disabled={busy}
            className={`rounded px-4 py-2 text-sm font-medium text-white disabled:opacity-50 ${danger ? "bg-red-600" : "bg-sky-600"}`}
          >
            {busy ? "Working…" : confirmLabel}
          </button>
          <button onClick={onCancel} disabled={busy} className="rounded border border-slate-700 px-4 py-2 text-sm disabled:opacity-50">
            Cancel
          </button>
        </div>
      </div>
    </Modal>
  );
}
