import { useState } from "react";
import { BusinessProfileTab } from "./BusinessProfileTab";
import { DataTab } from "./DataTab";

type Tab = "business" | "data";

const TABS: { id: Tab; label: string }[] = [
  { id: "business", label: "Business Profile" },
  { id: "data", label: "Data" },
];

/** ui-ux.md §2 — Settings has no nested route of its own; Business Profile / Data are tabs within one screen. */
export function SettingsScreen() {
  const [tab, setTab] = useState<Tab>("business");

  return (
    <div className="space-y-4">
      <div className="page-header">
        <h1 className="text-xl font-semibold">Settings</h1>
      </div>
      <div className="flex gap-1 border-b border-border text-sm">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`relative px-3 py-2 transition-colors ${
              tab === t.id ? "font-medium text-accent" : "text-text-secondary hover:text-text-primary"
            }`}
          >
            {t.label}
            {tab === t.id && <span className="absolute inset-x-0 -bottom-px h-0.5 rounded-full bg-accent" />}
          </button>
        ))}
      </div>
      {tab === "business" && <BusinessProfileTab />}
      {tab === "data" && <DataTab />}
    </div>
  );
}
