import { useCallback, useEffect, useState } from "react";
import { createCategory, deleteCategory, listCategories, updateCategory } from "../lib/tauri/commands";
import type { CategoryFields, CategoryListItem } from "../lib/tauri/types";

/** ui-ux.md §6 — the Categories inline-edit table. */
export function useCategories() {
  const [categories, setCategories] = useState<CategoryListItem[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listCategories()
      .then(setCategories)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (fields: CategoryFields) => {
      const created = await createCategory(fields);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: CategoryFields) => {
      const updated = await updateCategory(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteCategory(id);
      reload();
    },
    [reload],
  );

  return { categories, error, create, update, remove, reload };
}
