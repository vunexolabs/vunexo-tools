import { useState } from "react";
import { SalesSummaryTab } from "./SalesSummaryTab";
import { TaxSummaryTab } from "./TaxSummaryTab";

type ReportTab = "SALES" | "TAX";

const TABS: { id: ReportTab; label: string }[] = [
  { id: "SALES", label: "Sales Summary" },
  { id: "TAX", label: "Tax Summary" },
];

/**
 * ui-ux-v2.md §6 — exactly two named reports, not a configurable report
 * builder. Each is its own filter bar + table + export, sharing only this
 * tab shell.
 */
export function ReportsScreen() {
  const [tab, setTab] = useState<ReportTab>("SALES");

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Reports</h1>
      <div className="flex gap-2 text-sm">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`rounded-md px-3 py-1.5 transition-colors ${tab === t.id ? "bg-blue-50 text-blue-700 dark:bg-blue-500/10 dark:text-blue-400" : "text-zinc-500 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800"}`}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "SALES" ? <SalesSummaryTab /> : <TaxSummaryTab />}
    </div>
  );
}
