// Date-range helpers for the Statement/Reports filter bars. Every screen
// here shows the user an inclusive "From"/"To" pair (the natural way to pick
// a range), but `generate_customer_statement`/`generate_sales_report`/
// `generate_tax_summary_report` all take `range_end` **exclusive**
// (`date(...) >= range_start AND date(...) < range_end`, see
// `sqlite_report_repository.rs`/`sqlite_statement_repository.rs`) — so every
// call site converts with `toExclusiveEnd` right before calling the command,
// never storing the exclusive form in UI state.

function toIsoDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

/** The next calendar day, as an ISO date string — turns an inclusive "To" into the exclusive end the backend expects. */
export function toExclusiveEnd(isoDate: string): string {
  const d = new Date(`${isoDate}T00:00:00`);
  d.setDate(d.getDate() + 1);
  return toIsoDate(d);
}

/** Current calendar quarter, both ends inclusive — ui-ux-v2.md §5's Statement tab default. */
export function currentQuarterRange(): { from: string; to: string } {
  const now = new Date();
  const quarterStartMonth = Math.floor(now.getMonth() / 3) * 3;
  const from = new Date(now.getFullYear(), quarterStartMonth, 1);
  const to = new Date(now.getFullYear(), quarterStartMonth + 3, 0);
  return { from: toIsoDate(from), to: toIsoDate(to) };
}

/** Current calendar month, both ends inclusive — the Reports screens' default. */
export function currentMonthRange(): { from: string; to: string } {
  const now = new Date();
  const from = new Date(now.getFullYear(), now.getMonth(), 1);
  const to = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  return { from: toIsoDate(from), to: toIsoDate(to) };
}
