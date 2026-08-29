import type { InvoiceStatus } from "../lib/tauri/types";

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
