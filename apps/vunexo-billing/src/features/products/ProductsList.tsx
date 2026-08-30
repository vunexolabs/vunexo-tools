import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { useProducts } from "../../hooks/useProducts";
import type { Product, ProductListItem } from "../../lib/tauri/types";
import { ProductForm } from "./ProductForm";

/** Mirrors features/customers/CustomersList.tsx exactly — see ui-ux.md §5. */
export function ProductsList() {
  const { symbol, formatMinor } = useCurrency();
  const [includeArchived, setIncludeArchived] = useState(false);
  const { products, error, create, update, archive, restore, remove } = useProducts(includeArchived);
  const [editing, setEditing] = useState<Product | "new" | null>(null);
  const [rowError, setRowError] = useState<unknown>(null);
  const [deleteTarget, setDeleteTarget] = useState<Product | null>(null);

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
        <h1 className="text-xl font-semibold">Products / Services</h1>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-slate-400">
            <input type="checkbox" checked={includeArchived} onChange={(e) => setIncludeArchived(e.target.checked)} />
            Show archived
          </label>
          <button onClick={() => setEditing("new")} className="rounded bg-sky-600 px-3 py-1.5 text-sm font-medium">
            + New Product
          </button>
        </div>
      </div>

      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      {editing !== null && (
        <ProductForm
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
            <th className="pb-2">Unit</th>
            <th className="pb-2">Price</th>
            <th className="pb-2">Status</th>
            <th className="pb-2"></th>
          </tr>
        </thead>
        <tbody>
          {products?.map((p: ProductListItem) => (
            <tr key={p.id} className="border-t border-slate-800">
              <td className="py-2">{p.name}</td>
              <td className="py-2 text-slate-400">{p.unit}</td>
              <td className="py-2 text-slate-400">{symbol}{formatMinor(p.price_minor)}</td>
              <td className="py-2">
                <span className={p.status === "ARCHIVED" ? "text-slate-500" : "text-emerald-400"}>{p.status}</span>
              </td>
              <td className="py-2 text-right">
                <div className="flex justify-end gap-2">
                  <button onClick={() => setEditing(p)} className="text-sky-400 hover:underline">
                    Edit
                  </button>
                  {p.status === "ACTIVE" ? (
                    <button onClick={() => runRowAction(() => archive(p.id))} className="text-amber-400 hover:underline">
                      Archive
                    </button>
                  ) : (
                    <button onClick={() => runRowAction(() => restore(p.id))} className="text-emerald-400 hover:underline">
                      Restore
                    </button>
                  )}
                  {!p.has_invoices && (
                    <button onClick={() => setDeleteTarget(p)} className="text-red-400 hover:underline">
                      Delete
                    </button>
                  )}
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {products !== null && products.length === 0 && (
        <p className="text-sm text-slate-500">No products yet — click "+ New Product" to add one.</p>
      )}

      {deleteTarget && (
        <ConfirmDialog
          title="Delete this product?"
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
