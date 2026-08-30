import { useState } from "react";
import { BusinessProfileTab } from "./BusinessProfileTab";
import { InvoicingTab } from "./InvoicingTab";
import { TaxRatesTab } from "./TaxRatesTab";

type Tab = "business" | "tax_rates" | "invoicing";

const TABS: { id: Tab; label: string }[] = [
  { id: "business", label: "Business Profile" },
  { id: "tax_rates", label: "Tax Rates" },
  { id: "invoicing", label: "Invoicing" },
];

/**
 * ui-ux.md §2 — Settings has no nested route of its own; Business Profile /
 * Tax Rates / Invoicing are tabs within one screen. Data (backup/restore/
 * export, ui-ux.md §6) isn't built yet and is intentionally left off this
 * tab bar rather than shown as a dead tab.
 */
export function SettingsScreen() {
  const [tab, setTab] = useState<Tab>("business");

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold">Settings</h1>
      <div className="flex gap-2 border-b border-slate-800 text-sm">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`px-3 py-2 ${tab === t.id ? "border-b-2 border-sky-500 text-sky-400" : "text-slate-400 hover:text-slate-200"}`}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "business" && <BusinessProfileTab />}
      {tab === "tax_rates" && <TaxRatesTab />}
      {tab === "invoicing" && <InvoicingTab />}
    </div>
  );
}
