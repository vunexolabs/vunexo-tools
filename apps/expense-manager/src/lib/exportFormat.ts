// Client-side CSV building for the Reports export (user-flows.md §7) — the
// figures themselves are never recomputed here, only re-stringified.
// Mirrors `vunexo-billing`'s `lib/exportFormat.ts`.

/** RFC 4180 escaping. */
export function csvRow(fields: (string | number)[]): string {
  return fields.map(csvField).join(",") + "\r\n";
}

function csvField(value: string | number): string {
  const s = String(value);
  if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}
