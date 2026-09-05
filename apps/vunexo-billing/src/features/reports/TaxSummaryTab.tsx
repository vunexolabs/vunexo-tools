import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { chooseSavePath } from "../../lib/tauri/client";
import { generateTaxSummaryReport, writeExportFile } from "../../lib/tauri/commands";
import type { TaxSummaryResult } from "../../lib/tauri/types";
import { currentMonthRange, toExclusiveEnd } from "../../lib/dateRanges";
import { csvRow } from "../../lib/exportFormat";

/**
 * ui-ux-v2.md §6 — Tax Summary: date range, a table, Export (CSV/JSON). The
 * regime column only appears when the range actually spans more than one
 * `tax_regime_snapshot` value (database-schema-v2.md §7's mixed-regime edge
 * case) — with today's single-regime reality, that's every range, so this
 * is dormant until a second regime exists, not untested-by-construction.
 */
export function TaxSummaryTab() {
  const { symbol, formatMinor, code } = useCurrency();
  const initial = currentMonthRange();
  const [from, setFrom] = useState(initial.from);
  const [to, setTo] = useState(initial.to);
  const [result, setResult] = useState<TaxSummaryResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState<"CSV" | "JSON" | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const run = async () => {
    setError(null);
    setDone(null);
    setBusy(true);
    try {
      setResult(await generateTaxSummaryReport(from, toExclusiveEnd(to)));
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  const showRegimeColumn = (result?.by_regime.length ?? 0) > 1;

  const handleExport = async (format: "CSV" | "JSON") => {
    if (!result) return;
    setError(null);
    setDone(null);
    setExporting(format);
    try {
      const path = await chooseSavePath({
        defaultPath: `vunexo-tax-summary.${format.toLowerCase()}`,
        filters: [{ name: format, extensions: [format.toLowerCase()] }],
      });
      if (!path) return;
      const contents =
        format === "JSON"
          ? JSON.stringify(result, null, 2)
          : csvRow(["Tax Regime", "Tax Amount"]) +
            result.by_regime.map((r) => csvRow([r.tax_regime, formatMinor(r.tax_amount_minor)])).join("") +
            csvRow(["Total", formatMinor(result.total_tax_minor)]);
      await writeExportFile(path, contents);
      setDone(`Saved to ${path}`);
    } catch (err) {
      setError(err);
    } finally {
      setExporting(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">
          From
          <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
        <label className="flex flex-col gap-1">
          To
          <input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-2 py-1 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
        <button onClick={() => void run()} disabled={busy} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-1.5 font-medium disabled:opacity-50">
          {busy ? "Running…" : "Run Report"}
        </button>
      </div>

      <ErrorBanner error={error} />
      {done && <p className="text-sm text-green-600 dark:text-green-400">{done}</p>}

      {result && (
        <>
          <div className="flex items-center justify-between">
            <p className="text-sm text-zinc-500 dark:text-zinc-400">
              Total tax ({code}): <span className="font-semibold text-zinc-900 dark:text-zinc-100">{symbol}{formatMinor(result.total_tax_minor)}</span>
            </p>
            <div className="flex gap-2">
              <button onClick={() => void handleExport("CSV")} disabled={exporting !== null} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-1.5 text-sm disabled:opacity-50">
                {exporting === "CSV" ? "Exporting…" : "Export CSV"}
              </button>
              <button onClick={() => void handleExport("JSON")} disabled={exporting !== null} className="rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-1.5 text-sm disabled:opacity-50">
                {exporting === "JSON" ? "Exporting…" : "Export JSON"}
              </button>
            </div>
          </div>

          {showRegimeColumn && (
            <table className="w-full text-left text-sm">
              <thead className="text-zinc-500 dark:text-zinc-400">
                <tr>
                  <th className="pb-2">Tax Regime</th>
                  <th className="pb-2">Tax Amount</th>
                </tr>
              </thead>
              <tbody>
                {result.by_regime.map((row) => (
                  <tr key={row.tax_regime} className="border-t border-zinc-200 dark:border-zinc-800">
                    <td className="py-2">{row.tax_regime}</td>
                    <td className="py-2 text-zinc-500 dark:text-zinc-400">{symbol}{formatMinor(row.tax_amount_minor)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </>
      )}
    </div>
  );
}
