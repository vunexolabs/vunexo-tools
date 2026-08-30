import { useCallback, useEffect, useState } from "react";
import { deletePayment, listPaymentsForInvoice, recordPayment, updatePayment } from "../lib/tauri/commands";
import type { NewPayment, Payment, PaymentFields } from "../lib/tauri/types";

/**
 * ui-ux.md §1/§7 — backs the Record Payment panel, scoped to one invoice.
 * user-flows.md §6 — every mutation here also changes the parent invoice's
 * `status` as a side effect the panel itself doesn't return inline, so the
 * caller (InvoiceEditor) refetches the invoice after each one (ui-ux.md §3's
 * "mutations that affect status refetch the invoice" rule).
 */
export function usePayments(invoiceId: number) {
  const [payments, setPayments] = useState<Payment[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    listPaymentsForInvoice(invoiceId)
      .then(setPayments)
      .catch((err: unknown) => setError(err));
  }, [invoiceId]);

  useEffect(() => {
    reload();
  }, [reload]);

  const record = useCallback(
    async (payment: NewPayment) => {
      const created = await recordPayment(payment);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, fields: PaymentFields) => {
      const updated = await updatePayment(id, fields);
      reload();
      return updated;
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deletePayment(id);
      reload();
    },
    [reload],
  );

  return { payments, error, record, update, remove, reload };
}
