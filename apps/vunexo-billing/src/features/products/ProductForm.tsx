import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { useTaxRates } from "../../hooks/useTaxRates";
import type { Product, ProductFields } from "../../lib/tauri/types";

const EMPTY: ProductFields = {
  name: "",
  sku: null,
  description: null,
  unit: "",
  price_minor: 0,
  tax_rate_id: null,
  hsn_sac_code: null,
};

/** user-flows.md §4 — same form for standalone create and (later) inline create-from-invoice-picker. */
export function ProductForm({
  initial,
  onSubmit,
  onCancel,
}: {
  initial?: Product;
  onSubmit: (fields: ProductFields) => Promise<unknown>;
  onCancel: () => void;
}) {
  const [fields, setFields] = useState<ProductFields>(
    initial
      ? {
          name: initial.name,
          sku: initial.sku,
          description: initial.description,
          unit: initial.unit,
          price_minor: initial.price_minor,
          tax_rate_id: initial.tax_rate_id,
          hsn_sac_code: initial.hsn_sac_code,
        }
      : EMPTY,
  );
  const { symbol, formatMinor, parseToMinor } = useCurrency();
  const [priceInput, setPriceInput] = useState(initial ? formatMinor(initial.price_minor) : "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const { taxRates } = useTaxRates();

  const set = (key: "sku" | "description" | "hsn_sac_code") => (e: React.ChangeEvent<HTMLInputElement>) =>
    setFields((f) => ({ ...f, [key]: e.target.value || null }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await onSubmit({ ...fields, price_minor: parseToMinor(priceInput) });
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
      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Unit * <span className="text-slate-500">(pcs, hr, kg…)</span>
          <input
            required
            value={fields.unit}
            onChange={(e) => setFields((f) => ({ ...f, unit: e.target.value }))}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
          />
        </label>
        <label className="block text-sm">
          Price ({symbol}) *
          <input
            required
            inputMode="decimal"
            value={priceInput}
            onChange={(e) => setPriceInput(e.target.value)}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
          />
        </label>
      </div>
      <label className="block text-sm">
        SKU
        <input defaultValue={fields.sku ?? ""} onChange={set("sku")} className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2" />
      </label>
      <label className="block text-sm">
        Description
        <input defaultValue={fields.description ?? ""} onChange={set("description")} className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2" />
      </label>
      <label className="block text-sm">
        HSN/SAC code
        <input defaultValue={fields.hsn_sac_code ?? ""} onChange={set("hsn_sac_code")} className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2" />
      </label>
      <label className="block text-sm">
        Tax rate
        <select
          value={fields.tax_rate_id ?? ""}
          onChange={(e) => setFields((f) => ({ ...f, tax_rate_id: e.target.value ? Number(e.target.value) : null }))}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-3 py-2"
        >
          <option value="">None</option>
          {taxRates?.map((rate) => (
            <option key={rate.id} value={rate.id}>
              {rate.name}
            </option>
          ))}
        </select>
      </label>
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting || fields.name.trim() === "" || fields.unit.trim() === ""}
          className="rounded bg-sky-600 px-4 py-2 font-medium disabled:opacity-50"
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
