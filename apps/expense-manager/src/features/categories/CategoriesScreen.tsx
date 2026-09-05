import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
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
      <tr className="border-t border-slate-800">
        <td className="py-2">{item.name}</td>
        <td className="py-2 text-slate-400">{item.default_deductible ? "Deductible" : "Non-deductible"}</td>
        <td className="py-2 text-right">
          <div className="flex justify-end gap-2">
            <button onClick={() => setEditing(true)} className="text-sky-400 hover:underline">
              Edit
            </button>
            {item.has_expenses ? (
              <span className="text-slate-600" title="Has expenses recorded — can't be deleted">
                Delete
              </span>
            ) : (
              <button onClick={onDelete} className="text-red-400 hover:underline">
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
    <tr className="border-t border-slate-800">
      <td colSpan={3} className="py-2">
        <ErrorBanner error={error} />
        <div className="flex items-center gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm"
          />
          <label className="flex items-center gap-1 text-sm text-slate-400">
            <input
              type="checkbox"
              checked={defaultDeductible}
              onChange={(e) => setDefaultDeductible(e.target.checked)}
            />
            Default deductible
          </label>
          <button onClick={() => void submit()} disabled={saving} className="rounded bg-emerald-600 px-3 py-1 text-sm disabled:opacity-50">
            {saving ? "Saving…" : "Save"}
          </button>
          <button onClick={() => setEditing(false)} className="text-sm text-slate-400 hover:underline">
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
      <h1 className="text-xl font-semibold">Categories</h1>
      <ErrorBanner error={error} />
      <ErrorBanner error={rowError} />

      <table className="w-full text-left text-sm">
        <thead className="text-slate-400">
          <tr>
            <th className="pb-2">Name</th>
            <th className="pb-2">Default</th>
            <th className="pb-2"></th>
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
      {categories && categories.length === 0 && <p className="text-sm text-slate-500">No categories yet — add one below.</p>}

      <div className="space-y-2 border-t border-slate-800 pt-3">
        <ErrorBanner error={createError} />
        <div className="flex items-center gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. Equipment"
            className="flex-1 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm"
          />
          <label className="flex items-center gap-1 text-sm text-slate-400">
            <input type="checkbox" checked={defaultDeductible} onChange={(e) => setDefaultDeductible(e.target.checked)} />
            Default deductible
          </label>
          <button
            onClick={() => void submitNew()}
            disabled={creating || name.trim() === ""}
            className="rounded bg-emerald-600 px-3 py-1 text-sm font-medium disabled:opacity-50"
          >
            {creating ? "Adding…" : "+ Add"}
          </button>
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
