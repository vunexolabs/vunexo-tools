// database-schema.md §5 — single currency per install, a plain display
// string (`business.currency_symbol`), not an ISO code table. V1 doesn't
// model per-currency decimal places either (unlike Billing's `CURRENCIES`
// table) — every amount here is assumed to have 2 minor-unit decimal places,
// matching the product's stated user base (paise, cents) — a disclosed
// simplification, not a locked decision from the six Expense Manager docs.

/** `123450` -> `"1,234.50"`. Never touches the value with floating point. */
export function formatMinor(minor: number): string {
  const sign = minor < 0 ? "-" : "";
  const abs = Math.abs(Math.trunc(minor));
  const rupees = Math.floor(abs / 100);
  const paise = abs % 100;
  return `${sign}${rupees.toLocaleString("en-IN")}.${String(paise).padStart(2, "0")}`;
}

/** `"1,234.50"` -> `123450`. Parses to a plain number, then rounds once —
 * this is the one and only place user text becomes minor units; nothing
 * downstream of this ever re-parses or re-derives an amount. */
export function parseAmountToMinor(input: string): number {
  const cleaned = input.replace(/,/g, "").trim();
  if (cleaned === "") return 0;
  const value = Number(cleaned);
  if (!Number.isFinite(value)) return 0;
  return Math.round(value * 100);
}
