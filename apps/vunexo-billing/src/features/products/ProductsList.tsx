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
          <label className="flex items-center gap-2 text-sm text-zinc-500 dark:text-zinc-400">
            <input type="checkbox" checked={includeArchived} onChange={(e) => setIncludeArchived(e.target.checked)} />
            Show archived
          </label>
          <button onClick={() => setEditing("new")} className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-3 py-1.5 text-sm font-medium">
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

      <div className="overflow-hidden rounded-lg border border-zinc-200 dark:border-zinc-800">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-zinc-200 bg-zinc-50 text-zinc-500 dark:border-zinc-800 dark:bg-zinc-950/50 dark:text-zinc-400">
            <tr>
              <th className="px-4 py-2.5 font-medium">Name</th>
              <th className="px-4 py-2.5 font-medium">Unit</th>
              <th className="px-4 py-2.5 font-medium">Price</th>
              <th className="px-4 py-2.5 font-medium">Status</th>
              <th className="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody className="bg-white dark:bg-zinc-900">
            {products?.map((p: ProductListItem) => (
              <tr key={p.id} className="border-t border-zinc-200 dark:border-zinc-800">
                <td className="px-4 py-2.5">{p.name}</td>
                <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{p.unit}</td>
                <td className="px-4 py-2.5 text-zinc-500 dark:text-zinc-400">{symbol}{formatMinor(p.price_minor)}</td>
                <td className="px-4 py-2.5">
                  <span className={p.status === "ARCHIVED" ? "text-zinc-400 dark:text-zinc-500" : "text-green-600 dark:text-green-400"}>{p.status}</span>
                </td>
                <td className="px-4 py-2.5 text-right">
                  <div className="flex flex-wrap justify-end gap-1">
                    <button
                      onClick={() => setEditing(p)}
                      className="rounded-md px-2 py-1 text-xs font-medium text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-500/10"
                    >
                      Edit
                    </button>
                    {p.status === "ACTIVE" ? (
                      <button
                        onClick={() => runRowAction(() => archive(p.id))}
                        className="rounded-md px-2 py-1 text-xs font-medium text-amber-600 transition-colors hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-500/10"
                      >
                        Archive
                      </button>
                    ) : (
                      <button
                        onClick={() => runRowAction(() => restore(p.id))}
                        className="rounded-md px-2 py-1 text-xs font-medium text-green-600 transition-colors hover:bg-green-50 dark:text-green-400 dark:hover:bg-green-500/10"
                      >
                        Restore
                      </button>
                    )}
                    {!p.has_invoices && (
                      <button
                        onClick={() => setDeleteTarget(p)}
                        className="rounded-md px-2 py-1 text-xs font-medium text-red-600 transition-colors hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"
                      >
                        Delete
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {products !== null && products.length === 0 && (
        <p className="text-sm text-zinc-400 dark:text-zinc-500">No products yet — click "+ New Product" to add one.</p>
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
