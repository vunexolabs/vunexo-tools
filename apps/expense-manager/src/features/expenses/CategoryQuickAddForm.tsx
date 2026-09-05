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
    <form onSubmit={handleSubmit} className="card space-y-3 p-4">
      <ErrorBanner error={error} />
      <div>
        <label className="label">Category name *</label>
        <input required value={name} onChange={(e) => setName(e.target.value)} className="input mt-1" />
      </div>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={defaultDeductible}
          onChange={(e) => setDefaultDeductible(e.target.checked)}
          className="h-4 w-4 rounded border-border text-accent focus:ring-accent/50"
        />
        Default deductible
      </label>
      <div className="flex gap-2">
        <button type="submit" disabled={submitting || name.trim() === ""} className="btn-primary">
          {submitting ? "Saving…" : "Save"}
        </button>
        <button type="button" onClick={onCancel} className="btn-secondary">
          Cancel
        </button>
      </div>
    </form>
  );
}
