import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { PlusIcon, TrashIcon } from "../../components/icons";
import { useCategories } from "../../hooks/useCategories";
import type { CategoryListItem } from "../../lib/tauri/types";

/**
 * ui-ux.md §6 — "Single inline-edit table (name + default-deductible toggle
 * per row + delete action), matching Billing's Tax Rates screen exactly."
 * Editing a row's name/flag never touches an existing expense's own stored
 * deductibility (user-flows.md §4) — that's enforced entirely by
 * `application::expenses::CreateExpense` never re-reading this table after
 * an expense is saved, not by anything here.
 */
function CategoryRow({
  item,
  onSave,
  onDelete,
}: {
  item: CategoryListItem;
  onSave: (fields: { name: string; default_deductible: boolean }) => Promise<void>;
  onDelete: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(item.name);
  const [defaultDeductible, setDefaultDeductible] = useState(item.default_deductible);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);

  if (!editing) {
    return (
      <tr className="is-hoverable">
        <td className="pl-4">{item.name}</td>
        <td>
          <span className={item.default_deductible ? "badge-success" : "badge-neutral"}>
            {item.default_deductible ? "Deductible" : "Non-deductible"}
          </span>
        </td>
        <td className="pr-4 text-right">
          <div className="flex justify-end gap-3">
            <button onClick={() => setEditing(true)} className="link">
              Edit
            </button>
            {item.has_expenses ? (
              <span className="text-text-muted" title="Has expenses recorded — can't be deleted">
                Delete
              </span>
            ) : (
              <button onClick={onDelete} className="inline-flex items-center gap-1 text-sm text-danger hover:underline">
                <TrashIcon className="h-3.5 w-3.5" />
                Delete
              </button>
            )}
          </div>
        </td>
      </tr>
    );
  }

  const submit = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave({ name, default_deductible: defaultDeductible });
      setEditing(false);
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <tr className="is-hoverable">
      <td colSpan={3} className="px-4 py-3">
        <ErrorBanner error={error} />
        <div className="mt-2 flex items-center gap-3">
          <input value={name} onChange={(e) => setName(e.target.value)} className="input w-48" />
          <label className="flex items-center gap-2 text-sm text-text-secondary">
            <input
              type="checkbox"
              checked={defaultDeductible}
              onChange={(e) => setDefaultDeductible(e.target.checked)}
              className="h-4 w-4 rounded border-border text-accent focus:ring-accent/50"
            />
            Default deductible
          </label>
          <button onClick={() => void submit()} disabled={saving} className="btn-primary btn-sm">
            {saving ? "Saving…" : "Save"}
          </button>
          <button onClick={() => setEditing(false)} className="btn-ghost btn-sm">
            Cancel
          </button>
        </div>
      </td>
    </tr>
  );
}

export function CategoriesScreen() {
  const { categories, error, create, update, remove } = useCategories();
  const [name, setName] = useState("");
  const [defaultDeductible, setDefaultDeductible] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<unknown>(null);
  const [deleteTarget, setDeleteTarget] = useState<CategoryListItem | null>(null);
  const [rowError, setRowError] = useState<unknown>(null);

  const submitNew = async () => {
    setCreating(true);
    setCreateError(null);
    try {
      await create({ name, default_deductible: defaultDeductible });
      setName("");
      setDefaultDeductible(false);
    } catch (err) {
      setCreateError(err);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="max-w-2xl space-y-4">
      <div className="page-header">
        <h1 className="text-xl font-semibold">Categories</h1>
      </div>
      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <div className="card overflow-hidden">
        <table className="table-base">
          <thead>
            <tr>
              <th className="pl-4">Name</th>
              <th>Default</th>
              <th className="pr-4"></th>
            </tr>
          </thead>
          <tbody>
            {categories?.map((item) => (
              <CategoryRow
                key={item.id}
                item={item}
                onSave={(fields) => update(item.id, fields).then(() => undefined)}
                onDelete={() => setDeleteTarget(item)}
              />
            ))}
          </tbody>
        </table>
        {categories && categories.length === 0 && (
          <p className="px-4 py-6 text-center text-sm text-text-muted">No categories yet — add one below.</p>
        )}

        <div className="space-y-2 border-t border-border p-4">
          <ErrorBanner error={createError} />
          <div className="flex items-center gap-3">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Equipment"
              className="input flex-1"
            />
            <label className="flex items-center gap-2 whitespace-nowrap text-sm text-text-secondary">
              <input
                type="checkbox"
                checked={defaultDeductible}
                onChange={(e) => setDefaultDeductible(e.target.checked)}
                className="h-4 w-4 rounded border-border text-accent focus:ring-accent/50"
              />
              Default deductible
            </label>
            <button onClick={() => void submitNew()} disabled={creating || name.trim() === ""} className="btn-primary btn-sm">
              <PlusIcon className="h-3.5 w-3.5" />
              {creating ? "Adding…" : "Add"}
            </button>
          </div>
        </div>
      </div>

      {deleteTarget && (
        <ConfirmDialog
          title="Delete this category?"
          message={`"${deleteTarget.name}" has no expenses recorded, so this permanently removes the record. This can't be undone.`}
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteTarget(null)}
          onConfirm={async () => {
            setRowError(null);
            try {
              await remove(deleteTarget.id);
            } catch (err) {
              setRowError(err);
            }
            setDeleteTarget(null);
          }}
        />
      )}
    </div>
  );
}
