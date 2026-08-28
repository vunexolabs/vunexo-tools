import { useState } from "react";
import { greet } from "../lib/tauri/commands";

export function App() {
  const [status, setStatus] = useState<string>("checking foundation...");

  const checkFoundation = () => {
    greet("Vunexo")
      .then(setStatus)
      .catch((error: unknown) => setStatus(`command failed: ${String(error)}`));
  };

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 bg-slate-950 p-8 text-slate-100">
      <h1 className="text-2xl font-semibold">Vunexo Billing</h1>
      <p className="text-slate-400">Round 1 foundation — no product features yet.</p>
      <button
        type="button"
        onClick={checkFoundation}
        className="rounded bg-slate-800 px-4 py-2 hover:bg-slate-700"
      >
        Check React → Tauri → Rust
      </button>
      <p className="text-sm text-slate-400">{status}</p>
    </main>
  );
}
