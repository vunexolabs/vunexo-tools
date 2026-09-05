import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useBusiness } from "../../hooks/useBusiness";
import { useCurrency } from "../../hooks/useCurrency";
import type { Business } from "../../lib/tauri/types";

/** ui-ux.md §2 — Settings → Business Profile. */
export function BusinessProfileTab() {
  const { business, update } = useBusiness();
  const { refresh } = useCurrency();
  const [fields, setFields] = useState<Business | null>(business ?? null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [saved, setSaved] = useState(false);

  const current = fields ?? business;
  if (!current) return null;

  const set = <K extends keyof Business>(key: K, value: Business[K]) =>
    setFields((f) => ({ ...(f ?? business!), [key]: value }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaved(false);
    setSubmitting(true);
    try {
      await update(current);
      refresh();
      setSaved(true);
    } catch (err) {
      setError(err);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="card max-w-md space-y-4 p-5">
      <ErrorBanner error={error} />
      {saved && <p className="text-sm text-success">Saved.</p>}

      <div>
        <label className="label">Business name *</label>
        <input required value={current.name} onChange={(e) => set("name", e.target.value)} className="input mt-1" />
      </div>
      <div>
        <label className="label">Address</label>
        <input
          value={current.address ?? ""}
          onChange={(e) => set("address", e.target.value || null)}
          className="input mt-1"
        />
      </div>
      <div>
        <label className="label">Tax info</label>
        <input
          value={current.tax_info ?? ""}
          onChange={(e) => set("tax_info", e.target.value || null)}
          className="input mt-1"
        />
      </div>
      <div>
        <label className="label">Currency symbol</label>
        <input
          value={current.currency_symbol}
          onChange={(e) => set("currency_symbol", e.target.value)}
          className="input mt-1 w-24"
        />
      </div>
      <button type="submit" disabled={submitting || current.name.trim() === ""} className="btn-primary">
        {submitting ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
