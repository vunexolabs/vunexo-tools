import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import type { CategoryFields } from "../../lib/tauri/types";

/** ui-ux.md §3 — the quick-add form opened from the Expense Editor's category picker. */
export function CategoryQuickAddForm({
  onSubmit,
  onCancel,
}: {
  onSubmit: (fields: CategoryFields) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [name, setName] = useState("");
  const [defaultDeductible, setDefaultDeductible] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit({ name, default_deductible: defaultDeductible });
    } catch (err) {
      setError(err);
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3 rounded border border-slate-700 bg-slate-900 p-4">
      <ErrorBanner error={error} />
      <label className="block text-sm">
        Category name *
        <input
          required
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={defaultDeductible} onChange={(e) => setDefaultDeductible(e.target.checked)} />
        Default deductible
      </label>
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting || name.trim() === ""}
          className="rounded bg-emerald-600 px-4 py-2 font-medium disabled:opacity-50"
        >
          {submitting ? "Saving…" : "Save"}
        </button>
        <button type="button" onClick={onCancel} className="rounded border border-slate-700 px-4 py-2">
          Cancel
        </button>
      </div>
    </form>
  );
}
