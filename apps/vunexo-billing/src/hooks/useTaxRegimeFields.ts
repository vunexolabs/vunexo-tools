import type { TaxRegimeCode } from "../lib/tauri/types";

/**
 * ui-ux-v2.md §3/§8 — the one switch point for every regime-conditional
 * field (GSTIN, HSN/SAC, the interstate toggle, and by extension the
 * CGST/SGST/IGST-vs-VAT totals breakdown these same screens render).
 * Callers branch on the *document's* `tax_regime_snapshot` when viewing an
 * issued Invoice/Quote, or `business.tax_regime_code` when editing a Draft
 * (application-architecture-v2.md §4d) — never on `country_code`, which is a
 * separate, currency-display-only setting.
 *
 * Adding a third regime later means adding one entry to this table, not
 * hunting down conditional branches across the Invoice Editor, Quote Editor,
 * Customer/Product forms, and the PDF preview independently.
 */
export type TaxRegimeField = "gstin" | "hsn_sac" | "is_interstate";

const TAX_REGIME_FIELDS: Record<TaxRegimeCode, ReadonlySet<TaxRegimeField>> = {
  IN_GST: new Set(["gstin", "hsn_sac", "is_interstate"]),
  VAT_STANDARD: new Set(),
};

export function useTaxRegimeFields(regime: TaxRegimeCode): ReadonlySet<TaxRegimeField> {
  return TAX_REGIME_FIELDS[regime];
}
