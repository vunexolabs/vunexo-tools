import { FormEvent, useEffect, useState } from "react";
import { ErrorBanner } from "../../components/ErrorBanner";
import { useCurrency } from "../../hooks/useCurrency";
import { useSettings } from "../../hooks/useSettings";
import { useTaxRates } from "../../hooks/useTaxRates";
import { COUNTRIES, CURRENCIES } from "../../lib/currency";
import type { SettingsFields } from "../../lib/tauri/types";

/**
 * ui-ux.md §1/§6 — Settings → Invoicing. `invoice_number_format` becomes
 * read-only once the first invoice has been issued
 * (database-schema.md §7) — enforced by the backend, surfaced here as the
 * ordinary Conflict banner (ui-ux.md §3's error-mapping rule) rather than a
 * separate "is it locked yet" query.
 */
export function InvoicingTab() {
  const { settings, error, update } = useSettings();
  const { taxRates } = useTaxRates();
  const { refresh: refreshCurrency } = useCurrency();
  const [fields, setFields] = useState<SettingsFields | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<unknown>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (settings && !fields) setFields(settings);
  }, [settings, fields]);

  if (!fields) {
    return (
      <>
        <ErrorBanner error={error} />
        {!error && <p className="text-sm text-slate-500">Loading…</p>}
      </>
    );
  }

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setSaveError(null);
    setSaving(true);
    try {
      const updated = await update(fields);
      setFields(updated);
      setSaved(true);
      refreshCurrency();
    } catch (err) {
      setSaveError(err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="max-w-md space-y-4">
      <ErrorBanner error={saveError} />

      <label className="block text-sm">
        Invoice number format
        <input
          value={fields.invoice_number_format}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => f && { ...f, invoice_number_format: e.target.value });
          }}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
        />
        <span className="text-xs text-slate-500">
          Supports {"{year}"} and {"{seq}"}/{"{seq:04d}"} — locked once the first invoice has been issued.
        </span>
      </label>

      <label className="block text-sm">
        Default due days
        <input
          type="number"
          min={0}
          value={fields.default_due_days}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => f && { ...f, default_due_days: Number(e.target.value) });
          }}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
        />
      </label>

      <label className="block text-sm">
        Default tax rate
        <select
          value={fields.default_tax_rate_id ?? ""}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => f && { ...f, default_tax_rate_id: e.target.value ? Number(e.target.value) : null });
          }}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
        >
          <option value="">None</option>
          {taxRates?.map((rate) => (
            <option key={rate.id} value={rate.id}>
              {rate.name}
            </option>
          ))}
        </select>
      </label>

      <div className="grid grid-cols-2 gap-3">
        <label className="block text-sm">
          Country
          <select
            value={fields.country_code}
            onChange={(e) => {
              setSaved(false);
              const country = COUNTRIES.find((c) => c.code === e.target.value);
              setFields((f) => f && { ...f, country_code: e.target.value, currency_code: country?.currencyCode ?? f.currency_code });
            }}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
          >
            {!COUNTRIES.some((c) => c.code === fields.country_code) && (
              <option value={fields.country_code}>{fields.country_code}</option>
            )}
            {COUNTRIES.map((c) => (
              <option key={c.code} value={c.code}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
        <label className="block text-sm">
          Currency
          <select
            value={fields.currency_code}
            onChange={(e) => {
              setSaved(false);
              setFields((f) => f && { ...f, currency_code: e.target.value });
            }}
            className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
          >
            {!(fields.currency_code in CURRENCIES) && <option value={fields.currency_code}>{fields.currency_code}</option>}
            {Object.entries(CURRENCIES).map(([code, meta]) => (
              <option key={code} value={code}>
                {code} — {meta.name} ({meta.symbol})
              </option>
            ))}
          </select>
        </label>
      </div>
      <p className="text-xs text-slate-500">
        Only India's GST (CGST/SGST/IGST) tax model is implemented today — other countries' tax rules are on the roadmap. Currency
        symbol and decimal formatting apply everywhere immediately; set the tax rates under the Tax Rates tab to match your region.
      </p>

      <label className="block text-sm">
        Date format
        <input
          value={fields.date_format}
          onChange={(e) => {
            setSaved(false);
            setFields((f) => f && { ...f, date_format: e.target.value });
          }}
          className="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-3 py-2"
        />
      </label>

      <div className="flex items-center gap-3">
        <button type="submit" disabled={saving} className="rounded bg-sky-600 px-4 py-2 font-medium disabled:opacity-50">
          {saving ? "Saving…" : "Save"}
        </button>
        {saved && <span className="text-sm text-emerald-400">Saved.</span>}
      </div>
    </form>
  );
}
