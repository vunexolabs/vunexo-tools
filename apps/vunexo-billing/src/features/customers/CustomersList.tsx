import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCustomers } from "../../hooks/useCustomers";
import type { Customer, CustomerListItem } from "../../lib/tauri/types";
import { CustomerForm } from "./CustomerForm";

/**
 * ui-ux.md §5 — filter/search bar (search omitted for this first slice),
 * table, "+ New" action. Row actions follow §3's has_invoices-driven rule:
 * archive/restore/delete are decided from data already in hand, never by
 * attempting a delete and catching the resulting Conflict.
 */
export function CustomersList() {
  const [includeArchived, setIncludeArchived] = useState(false);
  const { customers, error, create, update, archive, restore, remove } = useCustomers(includeArchived);
  const [editing, setEditing] = useState<Customer | "new" | null>(null);
  const [rowError, setRowError] = useState<unknown>(null);
  const [deleteTarget, setDeleteTarget] = useState<Customer | null>(null);

  const runRowAction = async (action: () => Promise<void>) => {
    setRowError(null);
    try {
      await action();
    } catch (err) {
      setRowError(err);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Customers</h1>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-slate-400">
            <input type="checkbox" checked={includeArchived} onChange={(e) => setIncludeArchived(e.target.checked)} />
            Show archived
          </label>
          <button onClick={() => setEditing("new")} className="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium">
            + New Customer
          </button>
        </div>
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      {editing !== null && (
        <CustomerForm
          initial={editing === "new" ? undefined : editing}
          onCancel={() => setEditing(null)}
          onSubmit={async (fields) => {
            if (editing === "new") {
              await create(fields);
            } else {
              await update(editing.id, fields);
            }
            setEditing(null);
          }}
        />
      )}

      <table className="w-full text-left text-sm">
        <thead className="text-slate-400">
          <tr>
            <th className="pb-2">Name</th>
            <th className="pb-2">Phone</th>
            <th className="pb-2">Email</th>
            <th className="pb-2">Status</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {customers?.map((c: CustomerListItem) => (
            <tr key={c.id} className="border-t border-slate-800">
              <td className="py-2">{c.name}</td>
              <td className="py-2 text-slate-400">{c.phone ?? "—"}</td>
              <td className="py-2 text-slate-400">{c.email ?? "—"}</td>
              <td className="py-2">
                <span className={c.status === "ARCHIVED" ? "text-slate-500" : "text-emerald-400"}>{c.status}</span>
              </td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  <button onClick={() => setEditing(c)} className="text-sky-400 hover:underline">
                    Edit
                  </button>
                  {c.status === "ACTIVE" ? (
                    <button onClick={() => runRowAction(() => archive(c.id))} className="text-amber-400 hover:underline">
                      Archive
                    </button>
                  ) : (
                    <button onClick={() => runRowAction(() => restore(c.id))} className="text-emerald-400 hover:underline">
                      Restore
                    </button>
                  )}
                  {!c.has_invoices && (
                    <button onClick={() => setDeleteTarget(c)} className="text-red-400 hover:underline">
                      Delete
                    </button>
                  )}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {customers !== null && customers.length === 0 && (
        <p className="text-sm text-slate-500">No customers yet — click "+ New Customer" to add one.</p>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="Delete this customer?"
          message={`"${deleteTarget.name}" has no invoice history, so this permanently removes the record. This can't be undone.`}
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={async () => {
            await runRowAction(() => remove(deleteTarget.id));
            setDeleteTarget(null);
          }}
        />
      )}
    </div>
  );
}
