import { FormEvent, useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { chooseOpenPath } from "../../lib/tauri/client";
import { probeBusinessLogo } from "../../lib/tauri/commands";
import type { LogoProbe } from "../../lib/tauri/types";
import { useBusiness } from "../../hooks/useBusiness";
import { useTaxRegimeFields } from "../../hooks/useTaxRegimeFields";
import type { Business } from "../../lib/tauri/types";

/** ui-ux.md §1/§6 — Settings → Business Profile. Every field but name stays optional past initial setup. */
export function BusinessProfileTab() {
  const { business, update } = useBusiness();

  if (!business) {
    return <p className="text-sm text-zinc-400 dark:text-zinc-500">Loading…</p>;
  }

  return <BusinessProfileForm business={business} onSave={update} />;
}

function BusinessProfileForm({
  business,
  onSave,
}: {
  business: Business;
  onSave: (fields: Business) => Promise<Business>;
}) {
  const [fields, setFields] = useState<Business>(business);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [saved, setSaved] = useState(false);
  // The PDF renderer skips a logo it can't load rather than failing the
  // invoice — right for a render, but it means a moved or unsupported file
  // shows up only as a logo-less invoice. So say it here, where it's fixable.
  const [logoProbe, setLogoProbe] = useState<LogoProbe | null>(null);
  const taxFields = useTaxRegimeFields(fields.tax_regime_code);

  const logoPath = fields.logo_path;
  useEffect(() => {
    if (!logoPath) {
      setLogoProbe(null);
      return;
    }
    let stale = false;
    probeBusinessLogo(logoPath)
      .then((probe) => {
        if (!stale) setLogoProbe(probe);
      })
      .catch(() => {
        if (!stale) setLogoProbe(null);
      });
    return () => {
      stale = true;
    };
  }, [logoPath]);

  const set = (key: keyof Business) => (e: React.ChangeEvent<HTMLInputElement>) => {
    setSaved(false);
    setFields((f) => ({ ...f, [key]: e.target.value || null }));
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);
    try {
      // The backend imports a freshly chosen logo into its own data
      // directory and rewrites `logo_path` to the resulting managed
      // (relative) path — sync the form to what was actually saved, or the
      // input would keep showing the original absolute path and every
      // later save would re-import the same file again.
      const saved = await onSave(fields);
      setFields(saved);
      setSaved(true);
    } catch (err) {
      setError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="max-w-md space-y-4">
      <ErrorBanner error={error} />

      <label className="block text-sm">
        Business name *
        <input
          required
          value={fields.name}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => ({ ...f, name: e.target.value }));
          }}
          className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        />
      </label>

      <label className="block text-sm">
        Address
        <input value={fields.address ?? ""} onChange={set("address")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
      </label>

      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Phone
          <input value={fields.phone ?? ""} onChange={set("phone")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
        <label className="block text-sm">
          Email
          <input value={fields.email ?? ""} onChange={set("email")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
      </div>

      {/* Picking a file here copies it into the app's own data directory
          (`import_logo_if_chosen`, backend) rather than leaving it wherever
          the user found it — a bare reference to some arbitrary path
          wouldn't survive a backup/restore onto a different machine
          (database-schema.md §9). The invoice PDF prints the imported copy,
          and silently omits it if that copy has somehow gone missing. */}
      <div className="block text-sm">
        Logo
        <div className="mt-1 flex items-center gap-2">
          <input
            readOnly
            value={fields.logo_path ?? ""}
            placeholder="No logo chosen"
            className="w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 text-zinc-500 dark:text-zinc-400 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
          />
          <button
            type="button"
            onClick={async () => {
              const path = await chooseOpenPath({
                filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg"] }],
              });
              if (!path) return;
              setSaved(false);
              setFields((f) => ({ ...f, logo_path: path }));
            }}
            className="shrink-0 rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-2"
          >
            Choose…
          </button>
          {fields.logo_path && (
            <button
              type="button"
              onClick={() => {
                setSaved(false);
                setFields((f) => ({ ...f, logo_path: null }));
              }}
              className="shrink-0 rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-2 text-zinc-500 dark:text-zinc-400"
            >
              Remove
            </button>
          )}
        </div>
        {logoProbe?.status === "OK" && (
          <p className="mt-1 text-xs text-green-600 dark:text-green-400">
            Ready to print — {logoProbe.width_px} × {logoProbe.height_px} px.
          </p>
        )}
        {logoProbe?.status === "NOT_FOUND" && (
          <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
            That file isn't there any more — it was moved, renamed, or is on a drive that isn't connected. Choose it again.
          </p>
        )}
        {logoProbe?.status === "UNREADABLE" && (
          <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">That file can't be read as an image. PNG and JPEG print correctly.</p>
        )}
        <p className="mt-1 text-xs text-zinc-400 dark:text-zinc-500">
          Invoices already issued keep the logo they were issued with — they're a frozen record. Opening one and pressing
          "Save Changes" re-snapshots it, and every new invoice picks this up automatically.
        </p>
      </div>

      <label className="block text-sm">
        Tax regime
        <select
          value={fields.tax_regime_code}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => ({ ...f, tax_regime_code: e.target.value as Business["tax_regime_code"] }));
          }}
          className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="IN_GST">India (GST)</option>
          <option value="VAT_STANDARD">Standard VAT</option>
        </select>
      </label>
      {/* ui-ux-v2.md §3 — a plain confirmation, not a migration wizard: no
          recalculation preview, no per-draft opt-in, no affected-count. */}
      {fields.tax_regime_code !== business.tax_regime_code && (
        <p className="rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-950 px-3 py-2 text-xs text-zinc-500 dark:text-zinc-400">
          <span className="font-medium text-zinc-900 dark:text-zinc-100">Tax regime changed.</span> Existing issued documents won&apos;t be
          affected. Any Draft documents will use the new regime the next time you save them.
        </p>
      )}

      {taxFields.has("gstin") && (
        <label className="block text-sm">
          GSTIN
          <input value={fields.gstin ?? ""} onChange={set("gstin")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
      )}

      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Bank details
          <input value={fields.bank_details ?? ""} onChange={set("bank_details")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
        <label className="block text-sm">
          UPI ID
          <input value={fields.upi_id ?? ""} onChange={set("upi_id")} className="mt-1 w-full rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500" />
        </label>
      </div>

      <div className="flex items-center gap-3">
        <button
          type="submit"
          disabled={saving || fields.name.trim() === ""}
          className="rounded-md bg-blue-600 transition-colors hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-blue-500 dark:hover:bg-blue-600 dark:focus:ring-offset-zinc-900 px-4 py-2 font-medium disabled:opacity-50"
        >
          {saving ? "Saving…" : "Save"}
        </button>
        {saved && <span className="text-sm text-green-600 dark:text-green-400">Saved.</span>}
      </div>
    </form>
  );
}
