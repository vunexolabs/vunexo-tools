import { useCallback, useEffect, useState } from "react";
import { cancelInvoice, deleteDraftInvoice, duplicateInvoice, listInvoices } from "../lib/tauri/commands";
import type { InvoiceFilter, InvoiceStatus, InvoiceSummary } from "../lib/tauri/types";

/** ui-ux.md §5 — Invoices List. */
export function useInvoices(status: InvoiceStatus | null) {
  const [invoices, setInvoices] = useState<InvoiceSummary[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const filter: InvoiceFilter = { status };

  const reload = useCallback(() => {
    listInvoices(filter)
      .then(setInvoices)
      .catch((err: unknown) => setError(err));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status]);

  useEffect(() => {
    reload();
  }, [reload]);

  const cancel = useCallback(
    async (id: number, reason: string | null) => {
      await cancelInvoice(id, reason);
      reload();
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteDraftInvoice(id);
      reload();
    },
    [reload],
  );

  const duplicate = useCallback(
    async (id: number) => {
      const created = await duplicateInvoice(id);
      reload();
      return created;
    },
    [reload],
  );

  return { invoices, error, cancel, remove, duplicate, reload };
}
