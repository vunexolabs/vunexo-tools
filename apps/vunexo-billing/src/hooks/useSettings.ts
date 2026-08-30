import { useCallback, useEffect, useState } from "react";
import { getSettings, updateSettings } from "../lib/tauri/commands";
import type { Settings, SettingsFields } from "../lib/tauri/types";

/** ui-ux.md §1/§6 — Settings → Invoicing. */
export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<unknown>(null);

  const reload = useCallback(() => {
    getSettings()
      .then(setSettings)
      .catch((err: unknown) => setError(err));
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  const update = useCallback(async (fields: SettingsFields) => {
    const updated = await updateSettings(fields);
    setSettings(updated);
    return updated;
  }, []);

  return { settings, error, update, reload };
}
