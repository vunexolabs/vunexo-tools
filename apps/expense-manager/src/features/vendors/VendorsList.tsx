import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useVendors } from "../../hooks/useVendors";
import type { Vendor, VendorListItem } from "../../lib/tauri/types";
import { VendorForm } from "./VendorForm";

/**
 * ui-ux.md §5 — Vendors List: table, "+ New" action, row actions. Delete is
 * only ever offered when `has_expenses` is false — user-flows.md §3's
 * blocked-delete rule, decided from data already in hand rather than by
 * attempting a delete and catching the error.
 */
export function VendorsList({ onOpen }: { onOpen: (id: number) => void }) {
  const { vendors, error, create, update, remove } = useVendors();
  const [editing, setEditing] = useState<Vendor | "new" | null>(null);
  const [rowError, setRowError] = useState<unknown>(null);
  const [deleteTarget, setDeleteTarget] = useState<VendorListItem | null>(null);

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
        <h1 className="text-xl font-semibold">Vendors</h1>
        <button onClick={() => setEditing("new")} className="rounded bg-emerald-600 px-3 py-1.5 text-sm font-medium">
          + New Vendor
        </button>
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      {editing !== null && (
        <VendorForm
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
            <th className="pb-2">Contact</th>
            <th className="pb-2">Notes</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {vendors?.map((v) => (
            <tr key={v.id} className="border-t border-slate-800">
              <td className="py-2">{v.name}</td>
              <td className="py-2 text-slate-400">{v.contact ?? "—"}</td>
              <td className="py-2 text-slate-400">{v.notes ?? "—"}</td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  <button onClick={() => onOpen(v.id)} className="text-sky-400 hover:underline">
                    View
                  </button>
                  <button onClick={() => setEditing(v)} className="text-sky-400 hover:underline">
                    Edit
                  </button>
                  {v.has_expenses ? (
                    <span className="text-slate-600" title="Has expenses recorded — can't be deleted">
                      Delete
                    </span>
                  ) : (
                    <button onClick={() => setDeleteTarget(v)} className="text-red-400 hover:underline">
                      Delete
                    </button>
                  )}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {vendors !== null && vendors.length === 0 && (
        <p className="text-sm text-slate-500">No vendors yet — click "+ New Vendor" to add one.</p>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="Delete this vendor?"
          message={`"${deleteTarget.name}" has no expenses recorded, so this permanently removes the record. This can't be undone.`}
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
