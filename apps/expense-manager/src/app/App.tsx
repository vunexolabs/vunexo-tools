import { useState } from "react";
import {
  CategoriesIcon,
  DashboardIcon,
  ExpensesIcon,
  MoonIcon,
  ReportsIcon,
  SettingsIcon,
  SunIcon,
  VendorsIcon,
} from "../components/icons";
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
import { useTheme } from "../hooks/useTheme";
import { createExpense, updateExpense } from "../lib/tauri/commands";
import type { Expense, ExpenseFilter } from "../lib/tauri/types";

// ui-ux.md §2 — five top-level sections, flat, no nested menus.
type Section = "dashboard" | "expenses" | "vendors" | "categories" | "reports" | "settings";

const SECTIONS: { id: Section; label: string; icon: (props: { className?: string }) => JSX.Element }[] = [
  { id: "dashboard", label: "Dashboard", icon: DashboardIcon },
  { id: "expenses", label: "Expenses", icon: ExpensesIcon },
  { id: "vendors", label: "Vendors", icon: VendorsIcon },
  { id: "categories", label: "Categories", icon: CategoriesIcon },
  { id: "reports", label: "Reports", icon: ReportsIcon },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

/**
 * ui-ux.md §2 — top-level sections, flat, no nested menus. Business Setup is
 * a full-screen gate shown instead of the shell when `get_business` returns
 * nothing (user-flows.md §1), exactly like Billing.
 */
export function App() {
  const { business, create } = useBusiness();
  const { theme, toggle } = useTheme();
  const [section, setSection] = useState<Section>("dashboard");
  const [openExpense, setOpenExpense] = useState<Expense | "new" | null>(null);
  const [expenseFilter, setExpenseFilter] = useState<ExpenseFilter | undefined>(undefined);
  const [openVendorId, setOpenVendorId] = useState<number | null>(null);

  if (business === undefined) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background text-text-muted">Loading…</main>
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
    <div className="flex min-h-screen bg-background text-text-primary">
      <nav className="flex w-56 shrink-0 flex-col border-r border-border bg-surface p-4">
        <div className="mb-6 flex items-center gap-2 px-2">
          <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-accent text-sm font-semibold text-white">
            {business.name.trim().charAt(0).toUpperCase() || "V"}
          </span>
          <span className="truncate text-sm font-semibold">{business.name}</span>
        </div>
        <ul className="flex-1 space-y-0.5">
          {SECTIONS.map((s) => {
            const Icon = s.icon;
            const active = section === s.id;
            return (
              <li key={s.id}>
                <button
                  onClick={() => goToSection(s.id)}
                  className={`relative flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors ${
                    active
                      ? "bg-accent/10 font-medium text-accent"
                      : "text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                  }`}
                >
                  {active && <span className="absolute -left-4 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-accent" />}
                  <Icon className="h-4 w-4 shrink-0" />
                  {s.label}
                </button>
              </li>
            );
          })}
        </ul>
        <button
          onClick={toggle}
          className="mt-4 flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm text-text-secondary transition-colors hover:bg-surface-hover hover:text-text-primary"
        >
          {theme === "dark" ? <SunIcon className="h-4 w-4" /> : <MoonIcon className="h-4 w-4" />}
          {theme === "dark" ? "Light mode" : "Dark mode"}
        </button>
      </nav>
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-5xl px-8 py-8">
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
              <ExpensesList onOpen={setOpenExpense} onNew={() => setOpenExpense("new")} initialFilter={expenseFilter} />
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
        </div>
      </main>
    </div>
  );
}
