import { useState } from "react";
import { BusinessSetup } from "../features/settings/BusinessSetup";
import { CustomersList } from "../features/customers/CustomersList";
import { InvoiceEditor } from "../features/invoices/InvoiceEditor";
import { InvoicesList } from "../features/invoices/InvoicesList";
import { ProductsList } from "../features/products/ProductsList";
import { useBusiness } from "../hooks/useBusiness";

type Section = "dashboard" | "invoices" | "customers" | "products" | "settings";

const SECTIONS: { id: Section; label: string; implemented: boolean }[] = [
  { id: "dashboard", label: "Dashboard", implemented: false },
  { id: "invoices", label: "Invoices", implemented: true },
  { id: "customers", label: "Customers", implemented: true },
  { id: "products", label: "Products", implemented: true },
  { id: "settings", label: "Settings", implemented: false },
];

/**
 * ui-ux.md §2 — five top-level sections, flat, no nested menus. Business
 * Setup is a full-screen gate shown instead of the shell when no business
 * profile exists yet (user-flows.md §1), not a sidebar destination.
 */
export function App() {
  const { business, create } = useBusiness();
  const [section, setSection] = useState<Section>("invoices");
  const [openInvoiceId, setOpenInvoiceId] = useState<number | null>(null);

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
        {section === "invoices" &&
          (openInvoiceId === null ? (
            <InvoicesList onOpen={setOpenInvoiceId} />
          ) : (
            <InvoiceEditor invoiceId={openInvoiceId} onBack={() => setOpenInvoiceId(null)} />
          ))}
        {section === "customers" && <CustomersList />}
        {section === "products" && <ProductsList />}
      </main>
    </div>
  );
}
