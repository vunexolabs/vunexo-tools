// Date-range helpers for the Reports filter bar. Every report screen shows
// the user an inclusive "From"/"To" pair, but every `generate_*` command
// takes `range_end` **exclusive** (`date >= range_start AND date <
// range_end`, see `sqlite_report_repository.rs`) — so the call site converts
// with `toExclusiveEnd` right before calling the command, never storing the
// exclusive form in UI state. Mirrors `vunexo-billing`'s `lib/dateRanges.ts`.

function toIsoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

/** The next calendar day, as an ISO date string. */
export function toExclusiveEnd(isoDate: string): string {
  const d = new Date(`${isoDate}T00:00:00`);
  d.setDate(d.getDate() + 1);
  return toIsoDate(d);
}

/** Current calendar month, both ends inclusive — the Reports screen's default. */
export function currentMonthRange(): { from: string; to: string } {
  const now = new Date();
  const from = new Date(now.getFullYear(), now.getMonth(), 1);
  const to = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  return { from: toIsoDate(from), to: toIsoDate(to) };
}
