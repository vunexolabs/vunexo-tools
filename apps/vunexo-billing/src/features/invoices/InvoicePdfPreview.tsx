/**
 * The Preview from user-flows.md §5 step 5 — "a live, read-only rendering of
 * the eventual PDF, reachable at any point without leaving the draft".
 *
 * It shows the *actual* rendered PDF rather than an HTML lookalike: a second
 * template maintained in React would drift from the Rust one, and the whole
 * point of a preview is that it is what will print. The webview's own PDF
 * viewer supplies scrolling, zoom, and its print button.
 */
export function InvoicePdfPreview({
  url,
  title,
  onClose,
  onSave,
  saving,
}: {
  url: string;
  title: string;
  onClose: () => void;
  onSave: () => void;
  saving: boolean;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex flex-col bg-black/70 p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="mx-auto flex h-full w-full max-w-4xl flex-col overflow-hidden rounded-lg border border-slate-700 bg-slate-900">
        <div className="flex items-center justify-between border-b border-slate-700 px-4 py-3">
          <h2 className="text-sm font-medium text-slate-200">{title}</h2>
          <div className="flex items-center gap-2">
            <button
              onClick={onSave}
              disabled={saving}
              className="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
            >
              {saving ? "Saving…" : "Save PDF"}
            </button>
            <button onClick={onClose} className="rounded border border-slate-700 px-3 py-1.5 text-sm">
              Close
            </button>
          </div>
        </div>
        <iframe src={url} title={title} className="h-full w-full flex-1 bg-white" />
      </div>
    </div>
  );
}
