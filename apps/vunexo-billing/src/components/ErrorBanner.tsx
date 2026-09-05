import { errorMessage } from "../lib/tauri/types";

/** ui-ux.md §3 — one shared error rendering, not one-off per screen. */
export function ErrorBanner({ error }: { error: unknown }) {
  if (!error) return null;
  return (
    <div className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
      {errorMessage(error)}
    </div>
  );
}
