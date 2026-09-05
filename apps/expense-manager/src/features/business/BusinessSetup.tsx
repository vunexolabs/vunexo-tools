import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import type { Business } from "../../lib/tauri/types";

const EMPTY: Business = { name: "", address: null, tax_info: null, currency_symbol: "₹" };

/**
 * user-flows.md §1 — only business name is required; everything else is
 * optional and editable later from Settings. Shown instead of the app shell
 * whenever `get_business` returns null, and never reachable again once a
 * business profile exists (mirrors Billing's `BusinessSetup` exactly).
 */
export function BusinessSetup({ onCreated }: { onCreated: (business: Business) => Promise<Business> }) {
  const [fields, setFields] = useState<Business>(EMPTY);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);

  const set = (key: "address" | "tax_info") => (e: React.ChangeEvent<HTMLInputElement>) =>
    setFields((f) => ({ ...f, [key]: e.target.value || null }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onCreated(fields);
    } catch (err) {
      setError(err);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-8 text-text-primary">
      <div className="card w-full max-w-md p-8">
        <div className="mb-6 text-center">
          <span className="mx-auto mb-4 flex h-10 w-10 items-center justify-center rounded-md bg-accent text-lg font-semibold text-white">
            V
          </span>
          <h1 className="text-xl font-semibold">Set up your business</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Only the business name is required — everything else can be added later in Settings.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <ErrorBanner error={error} />

          <div>
            <label className="label" htmlFor="business-name">
              Business name *
            </label>
            <input
              id="business-name"
              required
              value={fields.name}
              onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
              className="input mt-1"
            />
          </div>

          <div>
            <label className="label" htmlFor="business-address">
              Address
            </label>
            <input id="business-address" onChange={set("address")} className="input mt-1" />
          </div>

          <div>
            <label className="label" htmlFor="business-tax-info">
              Tax info
            </label>
            <input
              id="business-tax-info"
              onChange={set("tax_info")}
              placeholder="e.g. GSTIN, VAT number"
              className="input mt-1"
            />
          </div>

          <div>
            <label className="label" htmlFor="business-currency">
              Currency symbol
            </label>
            <input
              id="business-currency"
              value={fields.currency_symbol}
              onChange={(e) => setFields((f) => ({ ...f, currency_symbol: e.target.value }))}
              className="input mt-1 w-24"
            />
          </div>

          <button type="submit" disabled={submitting || fields.name.trim() === ""} className="btn-primary w-full">
            {submitting ? "Saving…" : "Save and continue"}
          </button>
        </form>
      </div>
    </main>
  );
}
