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
    <form onSubmit={handleSubmit} className="max-w-md space-y-3">
      <ErrorBanner error={error} />
      {saved && <p className="text-sm text-emerald-400">Saved.</p>}

      <label className="block text-sm">
        Business name *
        <input
          required
          value={current.name}
          onChange={(e) => set("name", e.target.value)}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="block text-sm">
        Address
        <input
          value={current.address ?? ""}
          onChange={(e) => set("address", e.target.value || null)}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="block text-sm">
        Tax info
        <input
          value={current.tax_info ?? ""}
          onChange={(e) => set("tax_info", e.target.value || null)}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <label className="block text-sm">
        Currency symbol
        <input
          value={current.currency_symbol}
          onChange={(e) => set("currency_symbol", e.target.value)}
          className="mt-1 w-24 rounded border border-slate-700 bg-slate-950 px-3 py-2"
        />
      </label>
      <button type="submit" disabled={submitting || current.name.trim() === ""} className="rounded bg-emerald-600 px-4 py-2 font-medium disabled:opacity-50">
        {submitting ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
