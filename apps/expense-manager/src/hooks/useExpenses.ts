import { useCallback, useEffect, useState } from "react";
import {
  attachReceipt,
  createExpense,
  deleteExpense,
  listExpenses,
  removeReceipt,
  replaceReceipt,
  updateExpense,
} from "../lib/tauri/commands";
import type { Expense, ExpenseFilter, ExpenseInput } from "../lib/tauri/types";

/**
 * ui-ux.md §5 — Expenses List, filterable by category/vendor/date-range
 * (also what the dashboard's category-row click-through drives).
 */
export function useExpenses(filter: ExpenseFilter) {
  const [expenses, setExpenses] = useState<Expense[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const filterKey = JSON.stringify(filter);

  const reload = useCallback(() => {
    listExpenses(filter)
      .then(setExpenses)
      .catch((err: unknown) => setError(err));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterKey]);

  useEffect(() => {
    reload();
  }, [reload]);

  const create = useCallback(
    async (input: ExpenseInput) => {
      const created = await createExpense(input);
      reload();
      return created;
    },
    [reload],
  );

  const update = useCallback(
    async (id: number, input: ExpenseInput) => {
      const updated = await updateExpense(id, input);
      reload();
      return updated;
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteExpense(id);
      reload();
    },
    [reload],
  );

  const attach = useCallback(
    async (id: number, path: string) => {
      const updated = await attachReceipt(id, path);
      reload();
      return updated;
    },
    [reload],
  );

  const replace = useCallback(
    async (id: number, path: string) => {
      const updated = await replaceReceipt(id, path);
      reload();
      return updated;
    },
    [reload],
  );

  const removeReceiptFile = useCallback(
    async (id: number) => {
      const updated = await removeReceipt(id);
      reload();
      return updated;
    },
    [reload],
  );

  return { expenses, error, create, update, remove, attach, replace, removeReceiptFile, reload };
}
