import { errorMessage } from "../lib/tauri/types";

/** ui-ux.md §3 — one shared error rendering, not one-off per screen. */
export function ErrorBanner({ error }: { error: unknown }) {
  if (!error) return null;
  return (
    <div className="rounded border border-red-800 bg-red-950 px-3 py-2 text-sm text-red-200">
      {errorMessage(error)}
    </div>
  );
}
