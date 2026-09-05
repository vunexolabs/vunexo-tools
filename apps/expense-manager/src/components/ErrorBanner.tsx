import { errorMessage } from "../lib/tauri/types";

/** ui-ux.md §3 — one shared error rendering, not one-off per screen. */
export function ErrorBanner({ error }: { error: unknown }) {
  if (!error) return null;
  return (
    <div className="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
      {errorMessage(error)}
    </div>
  );
}
