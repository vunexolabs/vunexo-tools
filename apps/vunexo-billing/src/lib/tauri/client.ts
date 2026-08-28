// The only file in `src/` that imports `@tauri-apps/api`. Everything else
// calls through `./commands.ts`. See docs/vunexo-billing/architecture.md.
import { invoke } from "@tauri-apps/api/core";

export function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}
