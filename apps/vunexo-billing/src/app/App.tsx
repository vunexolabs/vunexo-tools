import { useState } from "react";
import { ThemeToggle } from "../components/ThemeToggle";
import { BusinessSetup } from "../features/settings/BusinessSetup";
import { CustomerDetail } from "../features/customers/CustomerDetail";
import { CustomersList } from "../features/customers/CustomersList";
import { Dashboard } from "../features/dashboard/Dashboard";
import { InvoiceEditor } from "../features/invoices/InvoiceEditor";
import { InvoicesList, type FilterOption } from "../features/invoices/InvoicesList";
import { ProductsList } from "../features/products/ProductsList";
import { QuoteEditor } from "../features/quotes/QuoteEditor";
import { QuotesList } from "../features/quotes/QuotesList";
import { ReportsScreen } from "../features/reports/ReportsScreen";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { useBusiness } from "../hooks/useBusiness";

// ui-ux-v2.md §2 — Quotes and Reports are new top-level sections.
type Section = "dashboard" | "invoices" | "quotes" | "customers" | "products" | "reports" | "settings";

const SECTIONS: { id: Section; label: string; implemented: boolean }[] = [
  { id: "dashboard", label: "Dashboard", implemented: true },
  { id: "invoices", label: "Invoices", implemented: true },
  { id: "quotes", label: "Quotes", implemented: true },
  { id: "customers", label: "Customers", implemented: true },
  { id: "products", label: "Products", implemented: true },
  { id: "reports", label: "Reports", implemented: true },
  { id: "settings", label: "Settings", implemented: true },
];

// One tiny stroke icon per section, inlined rather than pulling in an icon
// library for seven glyphs — kept purely decorative (labels alone are the
// accessible name), same minimal-dependency stance as the rest of the app.
const SECTION_ICON_PATHS: Record<Section, string> = {
  dashboard: "M3 13h4v8H3v-8Zm7-9h4v17h-4V4Zm7 5h4v12h-4V9Z",
  invoices: "M6 3h9l3 3v15H6V3Zm0 6h9M6 12h9M6 15h6",
  quotes: "M4 6h16M4 12h16M4 18h10",
  customers: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-7 8a7 7 0 0 1 14 0",
  products: "M21 8 12 3 3 8l9 5 9-5Zm0 0v8l-9 5-9-5V8m9 5v8",
  reports: "M4 19V9m6 10V5m6 14v-7",
  settings:
    "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82A1.65 1.65 0 0 0 3 13.09H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z",
};

function SectionIcon({ section }: { section: Section }) {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="shrink-0">
      <path d={SECTION_ICON_PATHS[section]} />
    </svg>
  );
}

/**
 * ui-ux.md §2 / ui-ux-v2.md §2 — top-level sections, flat, no nested menus.
 * Business Setup is a full-screen gate shown instead of the shell when no
 * business profile exists yet (user-flows.md §1), not a sidebar destination.
 */
export function App() {
  const { business, create } = useBusiness();
  const [section, setSection] = useState<Section>("dashboard");
  const [openInvoiceId, setOpenInvoiceId] = useState<number | null>(null);
  const [invoiceFilter, setInvoiceFilter] = useState<FilterOption>(null);
  const [openQuoteId, setOpenQuoteId] = useState<number | null>(null);
  const [openCustomerId, setOpenCustomerId] = useState<number | null>(null);

  if (business === undefined) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-zinc-50 text-zinc-400 dark:bg-zinc-950 dark:text-zinc-500">
        Loading…
      </main>
    );
  }

  if (business === null) {
    return <BusinessSetup onCreated={create} />;
  }

  const goToSection = (s: Section) => {
    setSection(s);
    setOpenInvoiceId(null);
    setInvoiceFilter(null);
    setOpenQuoteId(null);
    setOpenCustomerId(null);
  };

  return (
    <div className="flex min-h-screen bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <nav className="flex w-56 shrink-0 flex-col border-r border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-900">
        <div className="mb-6 flex items-center gap-2 px-2">
          <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-blue-600 text-sm font-semibold text-white dark:bg-blue-500">
            {business.name.trim().charAt(0).toUpperCase() || "V"}
          </span>
          <span className="min-w-0 flex-1 truncate text-sm font-semibold text-zinc-900 dark:text-zinc-100">{business.name}</span>
          <ThemeToggle />
        </div>
        <ul className="flex-1 space-y-0.5">
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <button
                onClick={() => s.implemented && goToSection(s.id)}
                disabled={!s.implemented}
                className={`relative flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm font-medium transition-colors ${
                  section === s.id
                    ? "bg-blue-50 text-blue-700 dark:bg-blue-500/10 dark:text-blue-400"
                    : "text-zinc-600 hover:bg-zinc-100 dark:text-zinc-400 dark:hover:bg-zinc-800"
                } ${!s.implemented ? "cursor-not-allowed text-zinc-300 hover:bg-transparent dark:text-zinc-700 dark:hover:bg-transparent" : ""}`}
              >
                {section === s.id && (
                  <span className="absolute -left-4 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-blue-600 dark:bg-blue-400" />
                )}
                <SectionIcon section={s.id} />
                {s.label}
                {!s.implemented && <span className="ml-auto text-xs text-zinc-400 dark:text-zinc-600">soon</span>}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <main className="flex-1 overflow-y-auto p-8">
        {section === "dashboard" && (
          <Dashboard
            onOpenInvoice={(id) => {
              setSection("invoices");
              setOpenInvoiceId(id);
            }}
            onOpenOverdueInvoices={() => {
              setSection("invoices");
              setOpenInvoiceId(null);
              setInvoiceFilter("OVERDUE");
            }}
          />
        )}
        {section === "invoices" &&
          (openInvoiceId === null ? (
            <InvoicesList onOpen={setOpenInvoiceId} filter={invoiceFilter} onFilterChange={setInvoiceFilter} />
          ) : (
            <InvoiceEditor invoiceId={openInvoiceId} onBack={() => setOpenInvoiceId(null)} onOpenInvoice={setOpenInvoiceId} />
          ))}
        {section === "quotes" &&
          (openQuoteId === null ? (
            <QuotesList onOpen={setOpenQuoteId} />
          ) : (
            <QuoteEditor
              quoteId={openQuoteId}
              onBack={() => setOpenQuoteId(null)}
              onOpenQuote={setOpenQuoteId}
              onConverted={(invoiceId) => {
                setSection("invoices");
                setOpenQuoteId(null);
                setOpenInvoiceId(invoiceId);
              }}
            />
          ))}
        {section === "customers" &&
          (openCustomerId === null ? (
            <CustomersList onOpen={setOpenCustomerId} />
          ) : (
            <CustomerDetail customerId={openCustomerId} onBack={() => setOpenCustomerId(null)} />
          ))}
        {section === "products" && <ProductsList />}
        {section === "reports" && <ReportsScreen />}
        {section === "settings" && <SettingsScreen />}
      </main>
    </div>
  );
}
