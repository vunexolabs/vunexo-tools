import { useCallback, useEffect, useState } from "react";
import { createTaxRate, listTaxRates, updateTaxRate } from "../lib/tauri/commands";
import type { TaxRateFields } from "../lib/tauri/types";

/** ui-ux.md §5 — backs the Settings → Tax Rates list-and-inline-edit table. */
export function useTaxRates() {
  const [taxRates, setTaxRates] = useState<Awaited<ReturnType<typeof listTaxRates>> | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listTaxRates()
      .then(setTaxRates)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (fields: TaxRateFields) => {
      const created = await createTaxRate(fields);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: TaxRateFields) => {
      const updated = await updateTaxRate(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  return { taxRates, error, create, update, reload };
}
