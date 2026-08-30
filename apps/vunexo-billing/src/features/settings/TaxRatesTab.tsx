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
      <tr className="border-t border-slate-800">
        <td className="py-2">{taxRate.name}</td>
        <td className="py-2 text-slate-400">{formatBasisPointsAsPercent(taxRate.rate_basis_points)}%</td>
        <td className="py-2 text-right">
          <button onClick={() => setEditing(true)} className="text-sky-400 hover:underline">
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
    <tr className="border-t border-slate-800">
      <td colSpan={3} className="py-2">
        <ErrorBanner error={error} />
        <div className="flex items-center gap-2">
          <input value={name} onChange={(e) => setName(e.target.value)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm" />
          <input value={rateStr} onChange={(e) => setRateStr(e.target.value)} className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm" />
          <span className="text-sm text-slate-500">%</span>
          <button onClick={() => void submit()} disabled={saving} className="rounded bg-sky-600 px-3 py-1 text-sm disabled:opacity-50">
            {saving ? "Saving…" : "Save"}
          </button>
          <button onClick={() => setEditing(false)} className="text-sm text-slate-400 hover:underline">
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
      <table className="w-full text-left text-sm">
        <thead className="text-slate-400">
          <tr>
            <th className="pb-2">Name</th>
            <th className="pb-2">Rate</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {taxRates?.map((rate) => (
            <TaxRateRow key={rate.id} taxRate={rate} onSave={(fields) => update(rate.id, fields).then(() => undefined)} />
          ))}
        </tbody>
      </table>
      {taxRates && taxRates.length === 0 && <p className="text-sm text-slate-500">No tax rates yet — add one below.</p>}

      <div className="space-y-2 border-t border-slate-800 pt-3">
        <ErrorBanner error={createError} />
        <div className="flex items-center gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. GST 18%"
            className="flex-1 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm"
          />
          <input
            value={rateStr}
            onChange={(e) => setRateStr(e.target.value)}
            placeholder="18"
            className="w-20 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm"
          />
          <span className="text-sm text-slate-500">%</span>
          <button
            onClick={() => void submitNew()}
            disabled={creating || name.trim() === ""}
            className="rounded bg-sky-600 px-3 py-1 text-sm font-medium disabled:opacity-50"
          >
            {creating ? "Adding…" : "+ Add"}
          </button>
        </div>
      </div>
    </div>
  );
}
