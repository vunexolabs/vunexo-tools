import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useTaxRegimeFields } from "../../hooks/useTaxRegimeFields";
import type { Business } from "../../lib/tauri/types";

const EMPTY: Business = {
  name: "",
  logo_path: null,
  address: null,
  phone: null,
  email: null,
  gstin: null,
  bank_details: null,
  upi_id: null,
  tax_regime_code: "IN_GST",
};

/**
 * user-flows.md §1 — only business name is required; everything else is
 * optional and editable later from Settings. Shown instead of the app shell
 * whenever `get_business` returns null, and never reachable again once a
 * business profile exists.
 */
export function BusinessSetup({ onCreated }: { onCreated: (business: Business) => Promise<Business> }) {
  const [fields, setFields] = useState<Business>(EMPTY);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const taxFields = useTaxRegimeFields(fields.tax_regime_code);

  const set = (key: keyof Business) => (e: React.ChangeEvent<HTMLInputElement>) =>
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
    <main className="flex min-h-screen items-center justify-center bg-zinc-50 p-8 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <form onSubmit={handleSubmit} className="w-full max-w-md space-y-4">
        <div>
          <h1 className="text-2xl font-semibold">Set up your business</h1>
          <p className="text-sm text-zinc-500 dark:text-zinc-400">Only the business name is required — everything else can be added later in Settings.</p>
        </div>

        <ErrorBanner error={error} />

        <label className="block text-sm">
          Business name *
          <input
            required
            value={fields.name}
            onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
            className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
        </label>

        <label className="block text-sm">
          Address
          <input onChange={set("address")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>

        <div className="grid grid-cols-2 gap-3">
          <label className="block text-sm">
            Phone
            <input onChange={set("phone")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
          </label>
          <label className="block text-sm">
            Email
            <input onChange={set("email")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
          </label>
        </div>

        <label className="block text-sm">
          Tax regime
          <select
            value={fields.tax_regime_code}
            onChange={(e) => setFields((f) => ({ ...f, tax_regime_code: e.target.value as Business["tax_regime_code"] }))}
            className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="IN_GST">India (GST)</option>
            <option value="VAT_STANDARD">Standard VAT</option>
          </select>
          <p className="mt-1 text-xs text-zinc-400 dark:text-zinc-500">Changeable later in Settings — only India&apos;s GST is currently a fully modeled tax breakdown.</p>
        </label>

        {taxFields.has("gstin") && (
          <label className="block text-sm">
            GSTIN
            <input onChange={set("gstin")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
          </label>
        )}

        <button
          type="submit"
          disabled={submitting || fields.name.trim() === ""}
          className="w-full rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50"
        >
          {submitting ? "Saving…" : "Save and continue"}
        </button>
      </form>
    </main>
  );
}
