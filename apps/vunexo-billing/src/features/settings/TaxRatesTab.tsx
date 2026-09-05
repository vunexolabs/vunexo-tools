import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useTaxRates } from "../../hooks/useTaxRates";
import { formatBasisPointsAsPercent, parsePercentToBasisPoints, type TaxRate } from "../../lib/tauri/types";

/**
 * ui-ux.md §5 — "a simple list-and-inline-edit table under Settings (name +
 * rate%), not a dedicated master-detail flow" — small, low-cardinality
 * master data (a handful of GST slabs), no delete in V1
 * (application-architecture.md §3b).
 */

function TaxRateRow({ taxRate, onSave }: { taxRate: TaxRate; onSave: (fields: { name: string; rate_basis_points: number }) => Promise<void> }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(taxRate.name);
  const [rateStr, setRateStr] = useState(formatBasisPointsAsPercent(taxRate.rate_basis_points));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);

  if (!editing) {
    return (
      <tr className="border-t border-zinc-200 dark:border-zinc-800">
        <td className="px-4 py-2.5">{taxRate.name}</td>
        <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{formatBasisPointsAsPercent(taxRate.rate_basis_points)}%</td>
        <td className="px-4 py-2.5 text-right">
          <button
            onClick={() => setEditing(true)}
            className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-500/10"
          >
            Edit
          </button>
        </td>
      </tr>
    );
  }

  const submit = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave({ name, rate_basis_points: parsePercentToBasisPoints(rateStr) });
      setEditing(false);
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <tr className="border-t border-zinc-200 dark:border-zinc-800">
      <td colSpan={3} className="px-4 py-2.5">
        <ErrorBanner error={error} />
        <div className="flex items-center gap-2">
          <input value={name} onChange={(e) => setName(e.target.value)} className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
          <input value={rateStr} onChange={(e) => setRateStr(e.target.value)} className="w-20 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
          <span className="text-sm text-zinc-400 dark:text-zinc-500">%</span>
          <button onClick={() => void submit()} disabled={saving} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1 text-sm disabled:opacity-50">
            {saving ? "Saving…" : "Save"}
          </button>
          <button onClick={() => setEditing(false)} className="text-sm text-zinc-500 dark:text-zinc-400 transition-colors hover:underline">
            Cancel
          </button>
        </div>
      </td>
    </tr>
  );
}

export function TaxRatesTab() {
  const { taxRates, error, create, update } = useTaxRates();
  const [name, setName] = useState("");
  const [rateStr, setRateStr] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<unknown>(null);

  const submitNew = async () => {
    setCreating(true);
    setCreateError(null);
    try {
      await create({ name, rate_basis_points: parsePercentToBasisPoints(rateStr) });
      setName("");
      setRateStr("");
    } catch (err) {
      setCreateError(err);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="max-w-lg space-y-3">
      <ErrorBanner error={error} />
      <div className="overflow-hidden rounded-lg border border-zinc-200 dark:border-zinc-800">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 bg-zinc-50 text-zinc-500 dark:border-zinc-800 dark:bg-zinc-950/50 dark:text-zinc-400">
            <tr>
              <th className="px-4 py-2.5 font-medium">Name</th>
              <th className="px-4 py-2.5 font-medium">Rate</th>
              <th className="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody className="bg-white dark:bg-zinc-900">
            {taxRates?.map((rate) => (
              <TaxRateRow key={rate.id} taxRate={rate} onSave={(fields) => update(rate.id, fields).then(() => undefined)} />
            ))}
          </tbody>
        </table>
      </div>
      {taxRates && taxRates.length === 0 && <p className="text-sm text-zinc-400 dark:text-zinc-500">No tax rates yet — add one below.</p>}

      <div className="space-y-2 border-t border-zinc-200 dark:border-zinc-800 pt-3">
        <ErrorBanner error={createError} />
        <div className="flex items-center gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. GST 18%"
            className="flex-1 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <input
            value={rateStr}
            onChange={(e) => setRateStr(e.target.value)}
            placeholder="18"
            className="w-20 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <span className="text-sm text-zinc-400 dark:text-zinc-500">%</span>
          <button
            onClick={() => void submitNew()}
            disabled={creating || name.trim() === ""}
            className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1 text-sm font-medium disabled:opacity-50"
          >
            {creating ? "Adding…" : "+ Add"}
          </button>
        </div>
      </div>
    </div>
  );
}
