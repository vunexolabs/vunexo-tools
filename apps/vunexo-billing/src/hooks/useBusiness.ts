import { useCallback, useEffect, useState } from "react";
import { createBusiness, getBusiness, updateBusiness } from "../lib/tauri/commands";
import type { Business } from "../lib/tauri/types";

/**
 * user-flows.md §1/§2 — `business === null` (after the initial load) is the
 * first-run signal; the app never re-checks this once a business exists
 * (application-architecture.md §3b's `BusinessRepository::get` note).
 */
export function useBusiness() {
  const [business, setBusiness] = useState<Business | null | undefined>(undefined);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    getBusiness()
      .then(setBusiness)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(async (fields: Business) => {
    const created = await createBusiness(fields);
    setBusiness(created);
    return created;
  }, []);

  const update = useCallback(async (fields: Business) => {
    const updated = await updateBusiness(fields);
    setBusiness(updated);
    return updated;
  }, []);

  return { business, loading: business === undefined, error, create, update, reload };
}
