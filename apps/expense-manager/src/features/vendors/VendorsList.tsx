import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { PlusIcon, TrashIcon } from "../../components/icons";
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
      <div className="page-header">
        <h1 className="text-xl font-semibold">Vendors</h1>
        <button onClick={() => setEditing("new")} className="btn-primary">
          <PlusIcon className="h-4 w-4" />
          New Vendor
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

      <div className="card overflow-hidden">
        <table className="table-base">
          <thead>
            <tr>
              <th className="pl-4">Name</th>
              <th>Contact</th>
              <th>Notes</th>
              <th className="pr-4"></th>
            </tr>
          </thead>
          <tbody>
            {vendors?.map((v) => (
              <tr key={v.id} className="is-hoverable">
                <td className="pl-4">{v.name}</td>
                <td className="text-text-secondary">{v.contact ?? "—"}</td>
                <td className="text-text-secondary">{v.notes ?? "—"}</td>
                <td className="pr-4 text-right">
                  <div className="flex justify-end gap-3">
                    <button onClick={() => onOpen(v.id)} className="link">
                      View
                    </button>
                    <button onClick={() => setEditing(v)} className="link">
                      Edit
                    </button>
                    {v.has_expenses ? (
                      <span className="text-text-muted" title="Has expenses recorded — can't be deleted">
                        Delete
                      </span>
                    ) : (
                      <button
                        onClick={() => setDeleteTarget(v)}
                        className="inline-flex items-center gap-1 text-sm text-danger hover:underline"
                      >
                        <TrashIcon className="h-3.5 w-3.5" />
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
          <p className="px-4 py-6 text-center text-sm text-text-muted">No vendors yet — click "New Vendor" to add one.</p>
        )}
      </div>

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
