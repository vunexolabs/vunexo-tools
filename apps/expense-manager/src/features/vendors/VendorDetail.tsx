import { useState } from "react";
import { ChevronLeftIcon, PencilIcon } from "../../components/icons";
import { useVendors } from "../../hooks/useVendors";
import { VendorForm } from "./VendorForm";

/**
 * ui-ux.md §5 — Vendor Detail: view/edit a single vendor. No history/balance
 * concept (user-flows.md §3 — expense manager tracks outgoing spend, not
 * receivables, so there's no statement-style tab the way Billing's Customer
 * Detail has).
 */
export function VendorDetail({ vendorId, onBack }: { vendorId: number; onBack: () => void }) {
  const { vendors, update } = useVendors();
  const [editing, setEditing] = useState(false);
  const vendor = vendors?.find((v) => v.id === vendorId) ?? null;

  return (
    <div className="space-y-4">
      <button onClick={onBack} className="inline-flex items-center gap-1 text-sm text-text-secondary hover:text-text-primary">
        <ChevronLeftIcon className="h-4 w-4" />
        Back to Vendors
      </button>

      {vendor && (
        <>
          <h1 className="text-xl font-semibold">{vendor.name}</h1>

          {editing ? (
            <VendorForm
              initial={vendor}
              onCancel={() => setEditing(false)}
              onSubmit={async (fields) => {
                await update(vendor.id, fields);
                setEditing(false);
              }}
            />
          ) : (
            <div className="card max-w-md space-y-1 p-5">
              <Field label="Contact" value={vendor.contact} />
              <Field label="Notes" value={vendor.notes} />
              <Field label="Has expenses" value={vendor.has_expenses ? "Yes" : "No"} />
              <button onClick={() => setEditing(true)} className="btn-secondary btn-sm mt-3">
                <PencilIcon className="h-3.5 w-3.5" />
                Edit
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function Field({ label, value }: { label: string; value: string | null }) {
  return (
    <div className="flex justify-between border-b border-border py-2 text-sm last:border-0">
      <span className="text-text-secondary">{label}</span>
      <span>{value ?? "—"}</span>
    </div>
  );
}
