import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
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
    <main className="flex min-h-screen items-center justify-center bg-slate-950 p-8 text-slate-100">
      <form onSubmit={handleSubmit} className="w-full max-w-md space-y-4">
        <div>
          <h1 className="text-2xl font-semibold">Set up your business</h1>
          <p className="text-sm text-slate-400">Only the business name is required — everything else can be added later in Settings.</p>
        </div>

        <ErrorBanner error={error} />

        <label className="block text-sm">
          Business name *
          <input
            required
            value={fields.name}
            onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
          />
        </label>

        <label className="block text-sm">
          Address
          <input onChange={set("address")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>

        <div className="grid grid-cols-2 gap-3">
          <label className="block text-sm">
            Phone
            <input onChange={set("phone")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
          </label>
          <label className="block text-sm">
            Email
            <input onChange={set("email")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
          </label>
        </div>

        <label className="block text-sm">
          GSTIN
          <input onChange={set("gstin")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>

        <button
          type="submit"
          disabled={submitting || fields.name.trim() === ""}
          className="w-full rounded bg-sky-600 px-4 py-2 font-medium disabled:opacity-50"
        >
          {submitting ? "Saving…" : "Save and continue"}
        </button>
      </form>
    </main>
  );
}
