// The only file in `src/` that imports `@tauri-apps/api` (or a Tauri plugin).
// Everything else calls through `./commands.ts`. See
// docs/vunexo-billing/architecture.md.
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

/** OS "save as" dialog. Resolves to `null` when the user dismisses it. */
export function chooseSavePath(options: {
  defaultPath?: string;
  filters?: { name: string; extensions: string[] }[];
}): Promise<string | null> {
  return save(options);
}

/** OS "open file" dialog, single selection. Resolves to `null` when dismissed. */
export async function chooseOpenPath(options: {
  filters?: { name: string; extensions: string[] }[];
}): Promise<string | null> {
  const selected = await open({ multiple: false, directory: false, ...options });
  return typeof selected === "string" ? selected : null;
}
