import { useState } from "react";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { chooseOpenPath, chooseSavePath } from "../../lib/tauri/client";
import {
  backupDatabase,
  exportData,
  inspectBackup,
  restoreBackup,
  suggestedBackupFileName,
  suggestedExportFileName,
} from "../../lib/tauri/commands";
import type { BackupMetadata, ExportEntity } from "../../lib/tauri/types";

const EXPORTS: { entity: ExportEntity; label: string }[] = [
  { entity: "CUSTOMERS", label: "Export Customers (CSV)" },
  { entity: "PRODUCTS", label: "Export Products (CSV)" },
  { entity: "INVOICES", label: "Export Invoices (CSV)" },
];

/**
 * ui-ux.md §6 — Settings → Data. Backup and restore (user-flows.md §9), the
 * three CSV exports, and the JSON "everything" export, as one row of buttons
 * each. No options, no filters, no scheduling: that's the locked V1 scope.
 */
export function DataTab() {
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  // Held between "pick a file" and "yes, replace everything": the metadata is
  // read first so the confirmation can name what's about to overwrite the
  // user's data, and so an unreadable archive never reaches the dialog.
  const [pendingRestore, setPendingRestore] = useState<{ path: string; metadata: BackupMetadata } | null>(null);

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
        filters: [{ name: "Vunexo Backup", extensions: ["vbx"] }],
      });
      if (!path) return null;
      await backupDatabase(path);
      return `Backed up to ${path}`;
    });

  const handleChooseRestore = () =>
    run("restore", async () => {
      const path = await chooseOpenPath({
        filters: [{ name: "Vunexo Backup", extensions: ["vbx"] }],
      });
      if (!path) return null;
      // Throws — and so surfaces in the banner — if this build can't read it.
      const metadata = await inspectBackup(path);
      setPendingRestore({ path, metadata });
      return null;
    });

  const handleExport = (entity: ExportEntity, label: string) =>
    run(entity, async () => {
      const path = await chooseSavePath({
        defaultPath: await suggestedExportFileName(entity),
        filters: [
          entity === "ALL"
            ? { name: "JSON", extensions: ["json"] }
            : { name: "CSV", extensions: ["csv"] },
        ],
      });
      if (!path) return null;
      await exportData(entity, path);
      return `${label} saved to ${path}`;
    });

  return (
    <div className="max-w-2xl space-y-8">
      <ErrorBanner error={error} />
      {done && <p className="text-sm text-emerald-400">{done}</p>}

      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-slate-200">Backup &amp; restore</h2>
        <p className="text-sm text-slate-500">
          A backup is a single <code className="text-slate-400">.vbx</code> file holding your database and your logo. Keep it
          somewhere other than this computer — it's the only copy of your data that isn't on this machine.
        </p>
        <div className="flex gap-2 pt-1">
          <button
            onClick={() => void handleBackup()}
            disabled={busy !== null}
            className="rounded bg-sky-600 px-4 py-2 text-sm font-medium disabled:opacity-50"
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
          Exports are read-only — nothing in your data changes. Amounts are written as plain numbers so a spreadsheet reads them
          as numbers.
        </p>
        <div className="flex flex-wrap gap-2 pt-1">
          {EXPORTS.map(({ entity, label }) => (
            <button
              key={entity}
              onClick={() => void handleExport(entity, label)}
              disabled={busy !== null}
              className="rounded border border-slate-700 px-4 py-2 text-sm font-medium disabled:opacity-50"
            >
              {busy === entity ? "Exporting…" : label}
            </button>
          ))}
          <button
            onClick={() => void handleExport("ALL", "Export All Data (JSON)")}
            disabled={busy !== null}
            className="rounded border border-slate-700 px-4 py-2 text-sm font-medium disabled:opacity-50"
          >
            {busy === "ALL" ? "Exporting…" : "Export All Data (JSON)"}
          </button>
        </div>
      </section>

      {pendingRestore && (
        <ConfirmDialog
          title="Restore this backup?"
          message={`This replaces all current data — every customer, product, invoice and payment on this computer — with the contents of the backup made on ${new Date(pendingRestore.metadata.created_at).toLocaleString()} (app version ${pendingRestore.metadata.app_version}). This can't be undone. Vunexo Billing will restart when it's done.`}
          confirmLabel="Replace All Data"
          danger
          onCancel={() => setPendingRestore(null)}
          onConfirm={async () => {
            // Never resolves when it works: the app restarts.
            await restoreBackup(pendingRestore.path);
          }}
        />
      )}
    </div>
  );
}
