import { useCallback, useEffect, useState } from "react";
import {
  archiveCustomer,
  createCustomer,
  deleteCustomer,
  listCustomers,
  restoreCustomer,
  updateCustomer,
} from "../lib/tauri/commands";
import type { CustomerFields, CustomerListItem } from "../lib/tauri/types";

/** ui-ux.md §5 — Customers List, including the Archived filter toggle. */
export function useCustomers(includeArchived: boolean) {
  const [customers, setCustomers] = useState<CustomerListItem[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listCustomers({ include_archived: includeArchived })
      .then(setCustomers)
      .catch((err: unknown) => setError(err));
  }, [includeArchived]);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (fields: CustomerFields) => {
      const created = await createCustomer(fields);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: CustomerFields) => {
      const updated = await updateCustomer(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  // ui-ux.md §3 — archive/restore/delete decided by `has_invoices`, not by
  // attempting a delete and catching the resulting Conflict.
  const archive = useCallback(
    async (id: number) => {
      await archiveCustomer(id);
      reload();
    },
    [reload],
  );

  const restore = useCallback(
    async (id: number) => {
      await restoreCustomer(id);
      reload();
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteCustomer(id);
      reload();
    },
    [reload],
  );

  return { customers, error, create, update, archive, restore, remove, reload };
}
