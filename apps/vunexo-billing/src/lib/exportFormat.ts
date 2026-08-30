// Client-side CSV building for the Statement/Reports exports (ui-ux-v2.md
// §5/§6) — those are parameterized read models (a date range, a group-by),
// not a fixed shape `ExportEntity` enumerates, so there's no backend command
// to extend the way `export_data` covers Customers/Products/Invoices. The
// figures themselves are never recomputed here, only re-stringified — the
// same category of transform `formatMinorAsAmount` already does for on-screen
// display, mirroring `domain::export::export_amount`'s plain-decimal,
// no-grouping, no-symbol convention exactly so a report's CSV and an
// invoice's CSV read the same way in a spreadsheet.

/** RFC 4180 escaping, matching `domain::export::csv_record` field-for-field. */
export function csvRow(fields: (string | number)[]): string {
  return fields.map(csvField).join(",") + "\r\n";
}

function csvField(value: string | number): string {
  const s = String(value);
  if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}
