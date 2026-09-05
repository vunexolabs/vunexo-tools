import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import type { Vendor, VendorFields } from "../../lib/tauri/types";

const EMPTY: VendorFields = { name: "", contact: null, notes: null };

/** user-flows.md §3 — the same form for standalone create and inline quick-add from the Expense Editor's picker. */
export function VendorForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial?: Vendor;
  onSubmit: (fields: VendorFields) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [fields, setFields] = useState<VendorFields>(
    initial ? { name: initial.name, contact: initial.contact, notes: initial.notes } : EMPTY,
  );
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const set = (key: "contact" | "notes") => (e: React.ChangeEvent<HTMLInputElement>) =>
    setFields((f) => ({ ...f, [key]: e.target.value || null }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit(fields);
    } catch (err) {
      setError(err);
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3 rounded border border-slate-700 bg-slate-900 p-4">
      <ErrorBanner error={error} />
      <label className="block text-sm">
        Name *
        <input
          required
          value={fields.name}
          onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="block text-sm">
        Contact
        <input
          defaultValue={fields.contact ?? ""}
          onChange={set("contact")}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="block text-sm">
        Notes
        <input
          defaultValue={fields.notes ?? ""}
          onChange={set("notes")}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting || fields.name.trim() === ""}
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
