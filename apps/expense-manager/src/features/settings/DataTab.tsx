import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { chooseOpenPath, chooseSavePath } from "../../lib/tauri/client";
import { backupData, restoreBackup, suggestedBackupFileName } from "../../lib/tauri/commands";

/**
 * ui-ux.md §2 — Settings → Data: backup/restore (user-flows.md §9) plus
 * export (covered per-screen by Reports' own CSV export — there is no
 * separate "export everything" command in this product's locked command
 * surface, unlike Billing's `export_data`/`ExportEntity`).
 *
 * No `inspect_backup` command exists here (application-architecture.md's
 * command surface names only `backup_data`/`restore_backup`), so — unlike
 * Billing's DataTab — this confirmation is generic rather than naming the
 * archive's own recorded date/app-version.
 */
export function DataTab() {
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [pendingRestorePath, setPendingRestorePath] = useState<string | null>(null);

  const run = async (label: string, action: () => Promise<string | null>) => {
    setError(null);
    setDone(null);
    setBusy(label);
    try {
      const result = await action();
      if (result) setDone(result);
    } catch (err) {
      setError(err);
    } finally {
      setBusy(null);
    }
  };

  const handleBackup = () =>
    run("backup", async () => {
      const path = await chooseSavePath({
        defaultPath: await suggestedBackupFileName(),
        filters: [{ name: "Vunexo Expense Manager Backup", extensions: ["vex"] }],
      });
      if (!path) return null;
      await backupData(path);
      return `Backed up to ${path}`;
    });

  const handleChooseRestore = () =>
    run("restore", async () => {
      const path = await chooseOpenPath({
        filters: [{ name: "Vunexo Expense Manager Backup", extensions: ["vex"] }],
      });
      if (!path) return null;
      setPendingRestorePath(path);
      return null;
    });

  return (
    <div className="max-w-2xl space-y-8">
      <ErrorBanner error={error} />
      {done && <p className="text-sm text-emerald-400">{done}</p>}

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-slate-200">Backup &amp; restore</h2>
        <p className="text-sm text-slate-500">
          A backup is a single <code className="text-slate-400">.vex</code> file holding your database and every receipt
          image you've attached. Keep it somewhere other than this computer — it's the only copy of your data that isn't on
          this machine.
        </p>
        <div className="flex gap-2 pt-1">
          <button
            onClick={() => void handleBackup()}
            disabled={busy !== null}
            className="rounded bg-emerald-600 px-4 py-2 text-sm font-medium disabled:opacity-50"
          >
            {busy === "backup" ? "Backing up…" : "Back Up Now"}
          </button>
          <button
            onClick={() => void handleChooseRestore()}
            disabled={busy !== null}
            className="rounded border border-amber-700 px-4 py-2 text-sm font-medium text-amber-400 disabled:opacity-50"
          >
            {busy === "restore" ? "Reading…" : "Restore from Backup…"}
          </button>
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-slate-200">Export</h2>
        <p className="text-sm text-slate-500">
          Export a report as CSV from the Reports screen — each report kind exports its own result, read-only, with amounts
          written as plain numbers.
        </p>
      </section>

      {pendingRestorePath && (
        <ConfirmDialog
          title="Restore this backup?"
          message="This replaces all current data — every vendor, category, and expense (plus every receipt image) on this computer — with the contents of the chosen backup. This can't be undone. Vunexo Expense Manager will restart when it's done."
          confirmLabel="Replace All Data"
          danger
          onCancel={() => setPendingRestorePath(null)}
          onConfirm={async () => {
            // Never resolves when it works: the app restarts.
            await restoreBackup(pendingRestorePath);
          }}
        />
      )}
    </div>
  );
}
