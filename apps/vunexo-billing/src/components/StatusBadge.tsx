import type { InvoiceStatus, QuoteStatus } from "../lib/tauri/types";

const COLORS: Record<InvoiceStatus, string> = {
  DRAFT: "text-slate-400",
  ISSUED: "text-sky-400",
  PARTIALLY_PAID: "text-amber-400",
  PAID: "text-emerald-400",
  CANCELLED: "text-slate-600 line-through",
};

/**
 * ui-ux.md §3 — one fixed color per status, plus the derived OVERDUE badge
 * rendered *alongside* the stored status, never replacing it
 * (database-schema.md §8 — is_overdue is never a stored status).
 */
export function StatusBadge({ status, isOverdue }: { status: InvoiceStatus; isOverdue: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={COLORS[status]}>{status}</span>
      {isOverdue && <span className="rounded bg-red-950 px-1.5 py-0.5 text-xs text-red-300">OVERDUE</span>}
    </span>
  );
}

// ui-ux-v2.md §3 — same fixed-color-per-status rule, plus the derived
// EXPIRED badge (database-schema-v2.md §3). CONVERTED gets its own violet
// color rather than reusing PAID's green — a converted quote isn't "paid,"
// it became a different document.
const QUOTE_COLORS: Record<QuoteStatus, string> = {
  DRAFT: "text-slate-400",
  ISSUED: "text-sky-400",
  ACCEPTED: "text-emerald-400",
  DECLINED: "text-red-400",
  CONVERTED: "text-violet-400",
  CANCELLED: "text-slate-600 line-through",
};

export function QuoteStatusBadge({ status, isExpired }: { status: QuoteStatus; isExpired: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className={QUOTE_COLORS[status]}>{status}</span>
      {isExpired && <span className="rounded bg-amber-950 px-1.5 py-0.5 text-xs text-amber-300">EXPIRED</span>}
    </span>
  );
}
