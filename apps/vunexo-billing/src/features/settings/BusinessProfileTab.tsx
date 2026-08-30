import { FormEvent, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { chooseOpenPath } from "../../lib/tauri/client";
import { useBusiness } from "../../hooks/useBusiness";
import type { Business } from "../../lib/tauri/types";

/** ui-ux.md §1/§6 — Settings → Business Profile. Every field but name stays optional past initial setup. */
export function BusinessProfileTab() {
  const { business, update } = useBusiness();

  if (!business) {
    return <p className="text-sm text-slate-500">Loading…</p>;
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

  const set = (key: keyof Business) => (e: React.ChangeEvent<HTMLInputElement>) => {
    setSaved(false);
    setFields((f) => ({ ...f, [key]: e.target.value || null }));
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);
    try {
      await onSave(fields);
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
          className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
        />
      </label>

      <label className="block text-sm">
        Address
        <input value={fields.address ?? ""} onChange={set("address")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
      </label>

      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Phone
          <input value={fields.phone ?? ""} onChange={set("phone")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>
        <label className="block text-sm">
          Email
          <input value={fields.email ?? ""} onChange={set("email")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>
      </div>

      {/* The logo is a *path*, not an upload — the app is offline-first and
          the file stays where the user keeps it. The invoice PDF prints it in
          the letterhead, and silently omits it if the file has since moved. */}
      <div className="block text-sm">
        Logo
        <div className="mt-1 flex items-center gap-2">
          <input
            readOnly
            value={fields.logo_path ?? ""}
            placeholder="No logo chosen"
            className="w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 text-slate-400"
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
            className="shrink-0 rounded border border-slate-700 px-3 py-2"
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
              className="shrink-0 rounded border border-slate-700 px-3 py-2 text-slate-400"
            >
              Remove
            </button>
          )}
        </div>
      </div>

      <label className="block text-sm">
        GSTIN
        <input value={fields.gstin ?? ""} onChange={set("gstin")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
      </label>

      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Bank details
          <input value={fields.bank_details ?? ""} onChange={set("bank_details")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>
        <label className="block text-sm">
          UPI ID
          <input value={fields.upi_id ?? ""} onChange={set("upi_id")} className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2" />
        </label>
      </div>

      <div className="flex items-center gap-3">
        <button
          type="submit"
          disabled={saving || fields.name.trim() === ""}
          className="rounded bg-sky-600 px-4 py-2 font-medium disabled:opacity-50"
        >
          {saving ? "Saving…" : "Save"}
        </button>
        {saved && <span className="text-sm text-emerald-400">Saved.</span>}
      </div>
    </form>
  );
}
