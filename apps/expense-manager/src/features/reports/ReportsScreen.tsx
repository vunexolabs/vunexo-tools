import { useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { currentMonthRange, toExclusiveEnd } from "../../lib/dateRanges";
import { csvRow } from "../../lib/exportFormat";
import { chooseSavePath } from "../../lib/tauri/client";
import {
  generateCategorySummary,
  generateDeductibleSummary,
  generatePeriodSummary,
  generateTaxItcSummary,
  generateTopVendors,
  writeExportFile,
} from "../../lib/tauri/commands";
import type {
  CategorySummaryResult,
  DeductibleSummaryResult,
  PeriodSummaryResult,
  TaxItcSummaryResult,
  TopVendorsResult,
} from "../../lib/tauri/types";

type ReportKind = "CATEGORY" | "PERIOD" | "DEDUCTIBLE" | "TAX_ITC" | "TOP_VENDORS";

const REPORTS: { id: ReportKind; label: string }[] = [
  { id: "CATEGORY", label: "Category Summary" },
  { id: "PERIOD", label: "Period Summary" },
  { id: "DEDUCTIBLE", label: "Deductible / Non-Deductible" },
  { id: "TAX_ITC", label: "Tax / ITC Summary" },
  { id: "TOP_VENDORS", label: "Top Vendors" },
];

type AnyResult =
  | { kind: "CATEGORY"; data: CategorySummaryResult }
  | { kind: "PERIOD"; data: PeriodSummaryResult }
  | { kind: "DEDUCTIBLE"; data: DeductibleSummaryResult }
  | { kind: "TAX_ITC"; data: TaxItcSummaryResult }
  | { kind: "TOP_VENDORS"; data: TopVendorsResult };

/**
 * ui-ux.md §7 — a report picker (5 kinds), date range, result table, export
 * button (CSV via `write_export_file` — the same generic "frontend renders
 * the export text" pattern Billing's Reports screens use).
 */
export function ReportsScreen() {
  const { symbol, formatMinor } = useCurrency();
  const initial = currentMonthRange();
  const [kind, setKind] = useState<ReportKind>("CATEGORY");
  const [from, setFrom] = useState(initial.from);
  const [to, setTo] = useState(initial.to);
  const [result, setResult] = useState<AnyResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [done, setDone] = useState<string | null>(null);

  const run = async () => {
    setError(null);
    setDone(null);
    setBusy(true);
    try {
      const rangeEnd = toExclusiveEnd(to);
      switch (kind) {
        case "CATEGORY":
          setResult({ kind: "CATEGORY", data: await generateCategorySummary(from, rangeEnd) });
          break;
        case "PERIOD":
          setResult({ kind: "PERIOD", data: await generatePeriodSummary(from, rangeEnd) });
          break;
        case "DEDUCTIBLE":
          setResult({ kind: "DEDUCTIBLE", data: await generateDeductibleSummary(from, rangeEnd) });
          break;
        case "TAX_ITC":
          setResult({ kind: "TAX_ITC", data: await generateTaxItcSummary(from, rangeEnd) });
          break;
        case "TOP_VENDORS":
          setResult({ kind: "TOP_VENDORS", data: await generateTopVendors(from, rangeEnd) });
          break;
      }
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void run();
    // Re-run whenever the report kind or range changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind]);

  const buildCsv = (): string => {
    if (!result) return "";
    switch (result.kind) {
      case "CATEGORY":
        return (
          csvRow(["Category", "Total"]) +
          result.data.rows.map((r) => csvRow([r.category_name, formatMinor(r.total_minor)])).join("") +
          csvRow(["Total", formatMinor(result.data.total_minor)])
        );
      case "PERIOD":
        return (
          csvRow(["Period", "Total"]) +
          result.data.rows.map((r) => csvRow([r.period, formatMinor(r.total_minor)])).join("") +
          csvRow(["Total", formatMinor(result.data.total_minor)])
        );
      case "DEDUCTIBLE":
        return (
          csvRow(["Deductible", "Non-Deductible"]) +
          csvRow([formatMinor(result.data.deductible_minor), formatMinor(result.data.non_deductible_minor)])
        );
      case "TAX_ITC":
        return (
          csvRow(["Tax Paid", "ITC-Eligible"]) +
          csvRow([formatMinor(result.data.tax_paid_minor), formatMinor(result.data.itc_eligible_minor)])
        );
      case "TOP_VENDORS":
        return (
          csvRow(["Vendor", "Total"]) +
          result.data.rows.map((r) => csvRow([r.vendor_name_snapshot, formatMinor(r.total_minor)])).join("")
        );
    }
  };

  const handleExport = async () => {
    if (!result) return;
    setError(null);
    setDone(null);
    setExporting(true);
    try {
      const path = await chooseSavePath({
        defaultPath: `expense-manager-report-${kind.toLowerCase()}-${from}-to-${to}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!path) return;
      await writeExportFile(path, buildCsv());
      setDone(`Saved to ${path}`);
    } catch (err) {
      setError(err);
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Reports</h1>

      <div className="flex flex-wrap gap-2 border-b border-slate-800 pb-3 text-sm">
        {REPORTS.map((r) => (
          <button
            key={r.id}
            onClick={() => setKind(r.id)}
            className={`rounded px-3 py-1.5 ${kind === r.id ? "bg-slate-800" : "text-slate-400 hover:bg-slate-900"}`}
          >
            {r.label}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap items-end gap-3 text-sm">
        <label className="flex flex-col gap-1">
          From
          <input type="date" value={from} onChange={(e) => setFrom(e.target.value)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1" />
        </label>
        <label className="flex flex-col gap-1">
          To
          <input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="rounded border border-slate-700 bg-slate-950 px-2 py-1" />
        </label>
        <button onClick={() => void run()} disabled={busy} className="rounded bg-emerald-600 px-4 py-1.5 font-medium disabled:opacity-50">
          {busy ? "Running…" : "Refresh"}
        </button>
        {result && (
          <button onClick={() => void handleExport()} disabled={exporting} className="rounded border border-slate-700 px-3 py-1.5 disabled:opacity-50">
            {exporting ? "Exporting…" : "Export CSV"}
          </button>
        )}
      </div>

      <ErrorBanner error={error} />
      {done && <p className="text-sm text-emerald-400">{done}</p>}
      <p className="text-xs text-slate-500">
        This is what you recorded, not a statutory determination — Expense Manager does not decide legal tax deductibility or
        ITC eligibility.
      </p>

      {result?.kind === "CATEGORY" && (
        <ReportTable
          rows={result.data.rows.map((r) => [r.category_name, `${symbol}${formatMinor(r.total_minor)}`])}
          headers={["Category", "Total"]}
          footer={["Total", `${symbol}${formatMinor(result.data.total_minor)}`]}
        />
      )}
      {result?.kind === "PERIOD" && (
        <ReportTable
          rows={result.data.rows.map((r) => [r.period, `${symbol}${formatMinor(r.total_minor)}`])}
          headers={["Period", "Total"]}
          footer={["Total", `${symbol}${formatMinor(result.data.total_minor)}`]}
        />
      )}
      {result?.kind === "DEDUCTIBLE" && (
        <ReportTable
          rows={[
            ["Deductible", `${symbol}${formatMinor(result.data.deductible_minor)}`],
            ["Non-Deductible", `${symbol}${formatMinor(result.data.non_deductible_minor)}`],
          ]}
          headers={["Classification", "Total"]}
        />
      )}
      {result?.kind === "TAX_ITC" && (
        <ReportTable
          rows={[
            ["Tax Paid", `${symbol}${formatMinor(result.data.tax_paid_minor)}`],
            ["ITC-Eligible", `${symbol}${formatMinor(result.data.itc_eligible_minor)}`],
          ]}
          headers={["Measure", "Total"]}
        />
      )}
      {result?.kind === "TOP_VENDORS" && (
        <ReportTable
          rows={result.data.rows.map((r) => [r.vendor_name_snapshot, `${symbol}${formatMinor(r.total_minor)}`])}
          headers={["Vendor", "Total"]}
        />
      )}
    </div>
  );
}

function ReportTable({ headers, rows, footer }: { headers: string[]; rows: string[][]; footer?: string[] }) {
  return (
    <table className="w-full text-left text-sm">
      <thead className="text-slate-400">
        <tr>
          {headers.map((h) => (
            <th key={h} className="pb-2">
              {h}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((row, i) => (
          <tr key={i} className="border-t border-slate-800">
            {row.map((cell, j) => (
              <td key={j} className="py-2">
                {cell}
              </td>
            ))}
          </tr>
        ))}
        {rows.length === 0 && (
          <tr>
            <td colSpan={headers.length} className="py-4 text-center text-slate-500">
              No data in this range.
            </td>
          </tr>
        )}
      </tbody>
      {footer && (
        <tfoot>
          <tr className="border-t border-slate-700 font-semibold">
            {footer.map((cell, j) => (
              <td key={j} className="py-2">
                {cell}
              </td>
            ))}
          </tr>
        </tfoot>
      )}
    </table>
  );
}
