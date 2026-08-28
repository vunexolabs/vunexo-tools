// Typed call signatures for Tauri commands. Add one entry here per command
// exposed in src-tauri/src/commands/mod.rs.
import { callCommand } from "./client";

/** Round 1 technical spike: proves the React -> Tauri -> Rust round trip. */
export function greet(name: string): Promise<string> {
  return callCommand<string>("greet", { name });
}
