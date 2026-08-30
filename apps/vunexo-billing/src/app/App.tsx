import { useState } from "react";
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
      <main className="flex min-h-screen items-center justify-center bg-slate-950 text-slate-400">
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
    <div className="flex min-h-screen bg-slate-950 text-slate-100">
      <nav className="w-48 shrink-0 border-r border-slate-800 p-4">
        <div className="mb-6 text-lg font-semibold">{business.name}</div>
        <ul className="space-y-1">
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <button
                onClick={() => s.implemented && goToSection(s.id)}
                disabled={!s.implemented}
                className={`w-full rounded px-3 py-2 text-left text-sm ${
                  section === s.id ? "bg-slate-800" : "hover:bg-slate-900"
                } ${!s.implemented ? "cursor-not-allowed text-slate-600" : ""}`}
              >
                {s.label}
                {!s.implemented && <span className="ml-2 text-xs text-slate-700">soon</span>}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <main className="flex-1 p-8">
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
