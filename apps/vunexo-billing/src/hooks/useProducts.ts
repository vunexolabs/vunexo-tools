import { useCallback, useEffect, useState } from "react";
import {
  archiveProduct,
  createProduct,
  deleteProduct,
  listProducts,
  restoreProduct,
  updateProduct,
} from "../lib/tauri/commands";
import type { ProductFields, ProductListItem } from "../lib/tauri/types";

/** Mirrors hooks/useCustomers.ts exactly — see ui-ux.md §5. */
export function useProducts(includeArchived: boolean) {
  const [products, setProducts] = useState<ProductListItem[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listProducts({ include_archived: includeArchived })
      .then(setProducts)
      .catch((err: unknown) => setError(err));
  }, [includeArchived]);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (fields: ProductFields) => {
      const created = await createProduct(fields);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: ProductFields) => {
      const updated = await updateProduct(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  const archive = useCallback(
    async (id: number) => {
      await archiveProduct(id);
      reload();
    },
    [reload],
  );

  const restore = useCallback(
    async (id: number) => {
      await restoreProduct(id);
      reload();
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteProduct(id);
      reload();
    },
    [reload],
  );

  return { products, error, create, update, archive, restore, remove, reload };
}
