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
    <form onSubmit={handleSubmit} className="card space-y-3 p-4">
      <ErrorBanner error={error} />
      <div>
        <label className="label">Name *</label>
        <input
          required
          value={fields.name}
          onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
          className="input mt-1"
        />
      </div>
      <div>
        <label className="label">Contact</label>
        <input defaultValue={fields.contact ?? ""} onChange={set("contact")} className="input mt-1" />
      </div>
      <div>
        <label className="label">Notes</label>
        <input defaultValue={fields.notes ?? ""} onChange={set("notes")} className="input mt-1" />
      </div>
      <div className="flex gap-2">
        <button type="submit" disabled={submitting || fields.name.trim() === ""} className="btn-primary">
          {submitting ? "Saving…" : "Save"}
        </button>
        <button type="button" onClick={onCancel} className="btn-secondary">
          Cancel
        </button>
      </div>
    </form>
  );
}
