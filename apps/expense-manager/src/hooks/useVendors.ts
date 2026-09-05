import { useCallback, useEffect, useState } from "react";
import { createVendor, deleteVendor, listVendors, updateVendor } from "../lib/tauri/commands";
import type { VendorFields, VendorListItem } from "../lib/tauri/types";

/** ui-ux.md §5 — Vendors List, including the blocked-delete `has_expenses` rule. */
export function useVendors() {
  const [vendors, setVendors] = useState<VendorListItem[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listVendors()
      .then(setVendors)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (fields: VendorFields) => {
      const created = await createVendor(fields);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: VendorFields) => {
      const updated = await updateVendor(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteVendor(id);
      reload();
    },
    [reload],
  );

  return { vendors, error, create, update, remove, reload };
}
