import { useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { chooseSavePath } from "../../lib/tauri/client";
import { generateCustomerStatement, getCustomer, updateCustomer, writeExportFile } from "../../lib/tauri/commands";
import type { Customer, StatementResult } from "../../lib/tauri/types";
import { currentQuarterRange, toExclusiveEnd } from "../../lib/dateRanges";
import { csvRow } from "../../lib/exportFormat";
import { CustomerForm } from "./CustomerForm";

type DetailTab = "OVERVIEW" | "STATEMENT";

/**
 * ui-ux-v2.md §5 — Customer Detail gains a Statement tab. V1 never actually
 * built a standalone Customer Detail screen (CustomersList's inline edit
 * form covered everything V1 needed), so this is the first one — scoped to
 * what the Statement work needs: Overview (the existing edit form) and
 * Statement. It does not add the Invoices/Payments tabs the original
 * ui-ux.md sketch describes; those already exist as filterable views
 * elsewhere and weren't part of this slice.
 */
export function CustomerDetail({ customerId, onBack }: { customerId: number; onBack: () => void }) {
  const [customer, setCustomer] = useState<Customer | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [tab, setTab] = useState<DetailTab>("OVERVIEW");

  useEffect(() => {
    getCustomer(customerId)
      .then(setCustomer)
      .catch((err: unknown) => setError(err));
  }, [customerId]);

  return (
    <div className="space-y-4">
      <button onClick={onBack} className="text-sm text-slate-400 hover:underline">
        ← Back to Customers
      </button>

      <ErrorBanner error={error} />

      {customer && (
        <>
          <h1 className="text-xl font-semibold">{customer.name}</h1>
          <div className="flex gap-2 text-sm">
            {(["OVERVIEW", "STATEMENT"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                className={`rounded px-3 py-1.5 ${tab === t ? "bg-slate-800" : "text-slate-400 hover:bg-slate-900"}`}
              >
                {t === "OVERVIEW" ? "Overview" : "Statement"}
              </button>
            ))}
          </div>

          {tab === "OVERVIEW" && (
            <OverviewTab customer={customer} onSaved={setCustomer} />
          )}
          {tab === "STATEMENT" && <StatementTab customer={customer} />}
        </>
      )}
    </div>
  );
}

function OverviewTab({ customer, onSaved }: { customer: Customer; onSaved: (c: Customer) => void }) {
  const [editing, setEditing] = useState(false);

  if (editing) {
    return (
      <CustomerForm
        initial={customer}
        onCancel={() => setEditing(false)}
        onSubmit={async (fields) => {
          onSaved(await updateCustomer(customer.id, fields));
          setEditing(false);
        }}
      />
    );
  }

  return (
    <div className="max-w-md space-y-2 text-sm">
      <Field label="Phone" value={customer.phone} />
      <Field label="Email" value={customer.email} />
      <Field label="Address" value={customer.address} />
      <Field label="GSTIN" value={customer.gstin} />
      <Field label="Status" value={customer.status} />
      <button onClick={() => setEditing(true)} className="mt-2 rounded border border-slate-700 px-4 py-2 text-sm font-medium">
        Edit
      </button>
    </div>
  );
}

function Field({ label, value }: { label: string; value: string | null }) {
  return (
    <div className="flex justify-between border-b border-slate-800 py-1">
      <span className="text-slate-400">{label}</span>
      <span>{value ?? "—"}</span>
    </div>
  );
}

function StatementTab({ customer }: { customer: Customer }) {
  const { symbol, formatMinor } = useCurrency();
  const initial = currentQuarterRange();
  const [from, setFrom] = useState(initial.from);
  const [to, setTo] = useState(initial.to);
  const [result, setResult] = useState<StatementResult | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [done, setDone] = useState<string | null>(null);

  const run = async () => {
    setError(null);
    setDone(null);
    setBusy(true);
    try {
      setResult(await generateCustomerStatement(customer.id, from, toExclusiveEnd(to)));
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void run();
    // Only on first mount — the default quarter range, for this customer.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [customer.id]);

  const handleExport = async () => {
    if (!result) return;
    setError(null);
    setDone(null);
    setExporting(true);
    try {
      const path = await chooseSavePath({
        defaultPath: `vunexo-statement-${customer.name.replace(/[^a-z0-9]+/gi, "-").toLowerCase()}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!path) return;
      const contents =
        csvRow(["Opening Balance", "", "", formatMinor(result.opening_balance_minor)]) +
        csvRow(["Date", "Type", "Reference", "Amount"]) +
        result.entries.map((e) => csvRow([e.date, e.kind, e.reference ?? "", formatMinor(e.amount_minor)])).join("") +
        csvRow(["Closing Balance", "", "", formatMinor(result.closing_balance_minor)]);
      await writeExportFile(path, contents);
      setDone(`Saved to ${path}`);
    } catch (err) {
      setError(err);
    } finally {
      setExporting(false);
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
        <button onClick={() => void run()} disabled={busy} className="rounded bg-sky-600 px-4 py-1.5 font-medium disabled:opacity-50">
          {busy ? "Running…" : "Refresh"}
        </button>
      </div>

      <ErrorBanner error={error} />
      {done && <p className="text-sm text-emerald-400">{done}</p>}

      {result && (
        <>
          <div className="flex items-center justify-between">
            <p className="text-sm text-slate-400">
              Opening balance: <span className="font-semibold text-slate-200">{symbol}{formatMinor(result.opening_balance_minor)}</span>
            </p>
            <button onClick={() => void handleExport()} disabled={exporting} className="rounded border border-slate-700 px-3 py-1.5 text-sm disabled:opacity-50">
              {exporting ? "Exporting…" : "Export CSV"}
            </button>
          </div>

          <table className="w-full text-left text-sm">
            <thead className="text-slate-400">
              <tr>
                <th className="pb-2">Date</th>
                <th className="pb-2">Type</th>
                <th className="pb-2">Reference</th>
                <th className="pb-2">Amount</th>
              </tr>
            </thead>
            <tbody>
              {result.entries.map((entry, i) => (
                <tr key={i} className="border-t border-slate-800">
                  <td className="py-2">{entry.date}</td>
                  <td className="py-2 text-slate-400">{entry.kind === "INVOICE" ? "Invoice" : "Payment"}</td>
                  <td className="py-2 text-slate-400">{entry.reference ?? "—"}</td>
                  <td className="py-2 text-slate-400">
                    {entry.kind === "PAYMENT" ? "-" : ""}
                    {symbol}
                    {formatMinor(entry.amount_minor)}
                  </td>
                </tr>
              ))}
              {result.entries.length === 0 && (
                <tr>
                  <td colSpan={4} className="py-4 text-center text-slate-500">
                    No activity in this range.
                  </td>
                </tr>
              )}
            </tbody>
          </table>

          <div className="flex justify-between border-t border-slate-700 pt-2 text-base font-semibold">
            <span>Closing Balance</span>
            <span>{symbol}{formatMinor(result.closing_balance_minor)}</span>
          </div>
        </>
      )}
    </div>
  );
}
