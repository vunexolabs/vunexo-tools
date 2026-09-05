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
      <div className="mx-auto flex h-full w-full max-w-4xl flex-col overflow-hidden rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900">
        <div className="flex items-center justify-between border-b border-zinc-300 dark:border-zinc-700 px-4 py-3">
          <h2 className="text-sm font-medium text-zinc-900 dark:text-zinc-100">{title}</h2>
          <div className="flex items-center gap-2">
            <button
              onClick={onSave}
              disabled={saving}
              className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1.5 text-sm font-medium disabled:opacity-50"
            >
              {saving ? "Saving…" : "Save PDF"}
            </button>
            <button onClick={onClose} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-1.5 text-sm">
              Close
            </button>
          </div>
        </div>
        <iframe src={url} title={title} className="h-full w-full flex-1 bg-white" />
      </div>
    </div>
  );
}
