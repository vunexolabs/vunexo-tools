import { useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { chooseSavePath } from "../../lib/tauri/client";
import { generateSalesReport, writeExportFile } from "../../lib/tauri/commands";
import type { SalesGrouping, SalesSummaryResult } from "../../lib/tauri/types";
import { currentMonthRange, toExclusiveEnd } from "../../lib/dateRanges";
import { csvRow } from "../../lib/exportFormat";

const GROUPINGS: { value: SalesGrouping; label: string }[] = [
  { value: "NONE", label: "None" },
  { value: "PRODUCT", label: "Product" },
  { value: "CUSTOMER", label: "Customer" },
];

/**
 * ui-ux-v2.md §6 — Sales Summary: date range + optional group-by, a table,
 * an Export (CSV/JSON) button. `generate_sales_report` is the one source of
 * truth for the totals shown here and in the export — never recomputed.
 */
export function SalesSummaryTab() {
  const { symbol, formatMinor, code } = useCurrency();
  const initial = currentMonthRange();
  const [from, setFrom] = useState(initial.from);
  const [to, setTo] = useState(initial.to);
  const [groupBy, setGroupBy] = useState<SalesGrouping>("NONE");
  const [result, setResult] = useState<SalesSummaryResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState<"CSV" | "JSON" | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const run = async () => {
    setError(null);
    setDone(null);
    setBusy(true);
    try {
      setResult(await generateSalesReport(from, toExclusiveEnd(to), groupBy));
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  const handleExport = async (format: "CSV" | "JSON") => {
    if (!result) return;
    setError(null);
    setDone(null);
    setExporting(format);
    try {
      const path = await chooseSavePath({
        defaultPath: `vunexo-sales-summary.${format.toLowerCase()}`,
        filters: [{ name: format, extensions: [format.toLowerCase()] }],
      });
      if (!path) return;
      const contents =
        format === "JSON"
          ? JSON.stringify(result, null, 2)
          : groupBy === "NONE"
            ? csvRow(["Total Sales"]) + csvRow([formatMinor(result.total_sales_minor)])
            : csvRow(["Label", "Sales"]) +
              result.rows.map((r) => csvRow([r.label, formatMinor(r.sales_minor)])).join("") +
              csvRow(["Total", formatMinor(result.total_sales_minor)]);
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
          <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1" />
        </label>
        <label className="flex flex-col gap-1">
          To
          <input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1" />
        </label>
        <label className="flex flex-col gap-1">
          Group by
          <select value={groupBy} onChange={(e) => setGroupBy(e.target.value as SalesGrouping)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1">
            {GROUPINGS.map((g) => (
              <option key={g.value} value={g.value}>
                {g.label}
              </option>
            ))}
          </select>
        </label>
        <button onClick={() => void run()} disabled={busy} className="rounded bg-sky-600 px-4 py-1.5 font-medium disabled:opacity-50">
          {busy ? "Running…" : "Run Report"}
        </button>
      </div>

      <ErrorBanner error={error} />
      {done && <p className="text-sm text-emerald-400">{done}</p>}

      {result && (
        <>
          <div className="flex items-center justify-between">
            <p className="text-sm text-slate-400">
              Total sales ({code}): <span className="font-semibold text-slate-200">{symbol}{formatMinor(result.total_sales_minor)}</span>
            </p>
            <div className="flex gap-2">
              <button onClick={() => void handleExport("CSV")} disabled={exporting !== null} className="rounded border border-slate-700 px-3 py-1.5 text-sm disabled:opacity-50">
                {exporting === "CSV" ? "Exporting…" : "Export CSV"}
              </button>
              <button onClick={() => void handleExport("JSON")} disabled={exporting !== null} className="rounded border border-slate-700 px-3 py-1.5 text-sm disabled:opacity-50">
                {exporting === "JSON" ? "Exporting…" : "Export JSON"}
              </button>
            </div>
          </div>

          {groupBy !== "NONE" && (
            <table className="w-full text-left text-sm">
              <thead className="text-slate-400">
                <tr>
                  <th className="pb-2">{groupBy === "PRODUCT" ? "Product" : "Customer"}</th>
                  <th className="pb-2">Sales</th>
                </tr>
              </thead>
              <tbody>
                {result.rows.map((row) => (
                  <tr key={row.label} className="border-t border-slate-800">
                    <td className="py-2">{row.label}</td>
                    <td className="py-2 text-slate-400">{symbol}{formatMinor(row.sales_minor)}</td>
                  </tr>
                ))}
                {result.rows.length === 0 && (
                  <tr>
                    <td colSpan={2} className="py-4 text-center text-slate-500">
                      No sales in this range.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          )}
        </>
      )}
    </div>
  );
}
