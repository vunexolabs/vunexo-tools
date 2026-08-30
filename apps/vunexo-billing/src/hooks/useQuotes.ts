import { useCallback, useEffect, useState } from "react";
import { cancelQuote, deleteDraftQuote, duplicateQuote, listQuotes } from "../lib/tauri/commands";
import type { QuoteFilter, QuoteStatus, QuoteSummary } from "../lib/tauri/types";

/** ui-ux-v2.md §2 — Quotes List, mirrors useInvoices.ts. */
export function useQuotes(status: QuoteStatus | null) {
  const [quotes, setQuotes] = useState<QuoteSummary[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const filter: QuoteFilter = { status };

  const reload = useCallback(() => {
    listQuotes(filter)
      .then(setQuotes)
      .catch((err: unknown) => setError(err));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status]);

  useEffect(() => {
    reload();
  }, [reload]);

  const cancel = useCallback(
    async (id: number, reason: string | null) => {
      await cancelQuote(id, reason);
      reload();
    },
    [reload],
  );

  const remove = useCallback(
    async (id: number) => {
      await deleteDraftQuote(id);
      reload();
    },
    [reload],
  );

  const duplicate = useCallback(
    async (id: number) => {
      const created = await duplicateQuote(id);
      reload();
      return created;
    },
    [reload],
  );

  return { quotes, error, cancel, remove, duplicate, reload };
}
