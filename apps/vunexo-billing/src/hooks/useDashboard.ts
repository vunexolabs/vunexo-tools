import { useCallback, useEffect, useState } from "react";
import { getDashboardMetrics } from "../lib/tauri/commands";
import type { DashboardMetrics } from "../lib/tauri/types";

/** ui-ux.md §1 — Dashboard, the default landing screen. */
export function useDashboard() {
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    getDashboardMetrics()
      .then(setMetrics)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  return { metrics, error, reload };
}
