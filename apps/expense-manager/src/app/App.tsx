import { useState } from "react";
import { BusinessSetup } from "../features/business/BusinessSetup";
import { CategoriesScreen } from "../features/categories/CategoriesScreen";
import { Dashboard } from "../features/dashboard/Dashboard";
import { ExpenseEditor } from "../features/expenses/ExpenseEditor";
import { ExpensesList } from "../features/expenses/ExpensesList";
import { ReportsScreen } from "../features/reports/ReportsScreen";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { VendorDetail } from "../features/vendors/VendorDetail";
import { VendorsList } from "../features/vendors/VendorsList";
import { useBusiness } from "../hooks/useBusiness";
import { createExpense, updateExpense } from "../lib/tauri/commands";
import type { Expense, ExpenseFilter } from "../lib/tauri/types";

// ui-ux.md §2 — five top-level sections, flat, no nested menus.
type Section = "dashboard" | "expenses" | "vendors" | "categories" | "reports" | "settings";

const SECTIONS: { id: Section; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "expenses", label: "Expenses" },
  { id: "vendors", label: "Vendors" },
  { id: "categories", label: "Categories" },
  { id: "reports", label: "Reports" },
  { id: "settings", label: "Settings" },
];

/**
 * ui-ux.md §2 — top-level sections, flat, no nested menus. Business Setup is
 * a full-screen gate shown instead of the shell when `get_business` returns
 * nothing (user-flows.md §1), exactly like Billing.
 */
export function App() {
  const { business, create } = useBusiness();
  const [section, setSection] = useState<Section>("dashboard");
  const [openExpense, setOpenExpense] = useState<Expense | "new" | null>(null);
  const [expenseFilter, setExpenseFilter] = useState<ExpenseFilter | undefined>(undefined);
  const [openVendorId, setOpenVendorId] = useState<number | null>(null);

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
    setOpenExpense(null);
    setExpenseFilter(undefined);
    setOpenVendorId(null);
  };

  return (
    <div className="flex min-h-screen bg-slate-950 text-slate-100">
      <nav className="w-48 shrink-0 border-r border-slate-800 p-4">
        <div className="mb-6 text-lg font-semibold">{business.name}</div>
        <ul className="space-y-1">
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <button
                onClick={() => goToSection(s.id)}
                className={`w-full rounded px-3 py-2 text-left text-sm ${
                  section === s.id ? "bg-slate-800" : "hover:bg-slate-900"
                }`}
              >
                {s.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <main className="flex-1 p-8">
        {section === "dashboard" && (
          <Dashboard
            onOpenCategory={(categoryId) => {
              setSection("expenses");
              setOpenExpense(null);
              setExpenseFilter({ category_id: categoryId });
            }}
            onOpenExpense={(expense) => {
              setSection("expenses");
              setOpenExpense(expense);
            }}
          />
        )}
        {section === "expenses" &&
          (openExpense === null ? (
            <ExpensesList
              onOpen={setOpenExpense}
              onNew={() => setOpenExpense("new")}
              initialFilter={expenseFilter}
            />
          ) : (
            <ExpenseEditor
              expense={openExpense === "new" ? null : openExpense}
              onSaved={setOpenExpense}
              onDeleted={() => setOpenExpense(null)}
              onBack={() => setOpenExpense(null)}
              createExpense={createExpense}
              updateExpense={updateExpense}
            />
          ))}
        {section === "vendors" &&
          (openVendorId === null ? (
            <VendorsList onOpen={setOpenVendorId} />
          ) : (
            <VendorDetail vendorId={openVendorId} onBack={() => setOpenVendorId(null)} />
          ))}
        {section === "categories" && <CategoriesScreen />}
        {section === "reports" && <ReportsScreen />}
        {section === "settings" && <SettingsScreen />}
      </main>
    </div>
  );
}
