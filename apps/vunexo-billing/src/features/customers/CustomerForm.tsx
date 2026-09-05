import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useBusiness } from "../../hooks/useBusiness";
import { useTaxRegimeFields } from "../../hooks/useTaxRegimeFields";
import type { Customer, CustomerFields } from "../../lib/tauri/types";

const EMPTY: CustomerFields = { name: "", phone: null, email: null, address: null, gstin: null };

/**
 * user-flows.md §3 — the same form for standalone create and inline
 * create-from-invoice-picker (the picker isn't built yet, but the form
 * itself doesn't need to know which entry point opened it).
 */
export function CustomerForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial?: Customer;
  onSubmit: (fields: CustomerFields) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [fields, setFields] = useState<CustomerFields>(
    initial
      ? { name: initial.name, phone: initial.phone, email: initial.email, address: initial.address, gstin: initial.gstin }
      : EMPTY,
  );
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const { business } = useBusiness();
  const taxFields = useTaxRegimeFields(business?.tax_regime_code ?? "IN_GST");

  const set = (key: keyof CustomerFields) => (e: React.ChangeEvent<HTMLInputElement>) =>
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
    <form onSubmit={handleSubmit} className="space-y-3 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
      <ErrorBanner error={error} />
      <label className="block text-sm">
        Name *
        <input
          required
          value={fields.name}
          onChange={(e) => setFields((f) => ({ ...f, name: e.target.value }))}
          className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        />
      </label>
      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Phone
          <input defaultValue={fields.phone ?? ""} onChange={set("phone")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
        <label className="block text-sm">
          Email
          <input defaultValue={fields.email ?? ""} onChange={set("email")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
      </div>
      <label className="block text-sm">
        Address
        <input defaultValue={fields.address ?? ""} onChange={set("address")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
      </label>
      {taxFields.has("gstin") && (
        <label className="block text-sm">
          GSTIN
          <input defaultValue={fields.gstin ?? ""} onChange={set("gstin")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
      )}
      <div className="flex gap-2">
        <button type="submit" disabled={submitting || fields.name.trim() === ""} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50">
          {submitting ? "Saving…" : "Save"}
        </button>
        <button type="button" onClick={onCancel} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-4 py-2">
          Cancel
        </button>
      </div>
    </form>
  );
}
