import { useEffect, useRef, useState } from "react";
import { PlusIcon } from "./icons";

interface PickerItem {
  id: number;
  label: string;
}

/**
 * ui-ux.md §3 — "SearchablePicker + quick-add modal for picking a vendor or
 * category inline from the Expense Editor, same as Billing's customer/
 * product picker in the Invoice Editor." Mirrors Billing's component
 * verbatim; generic over the item shape so both pickers reuse it.
 */
export function SearchablePicker<T extends PickerItem>({
  items,
  value,
  onChange,
  placeholder,
  createLabel,
  onCreateNew,
  className,
}: {
  items: T[];
  value: number | null;
  onChange: (id: number, item: T) => void;
  placeholder: string;
  createLabel: string;
  onCreateNew: () => void;
  className?: string;
}) {
  const [query, setQuery] = useState("");
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selected = items.find((i) => i.id === value) ?? null;

  useEffect(() => {
    if (!isOpen) setQuery(selected?.label ?? "");
  }, [selected?.label, isOpen]);

  const filtered = items.filter((i) => i.label.toLowerCase().includes(query.toLowerCase()));

  return (
    <div ref={containerRef} className={`relative ${className ?? ""}`}>
      <input
        value={query}
        placeholder={placeholder}
        onFocus={() => {
          setIsOpen(true);
          setQuery("");
        }}
        onChange={(e) => setQuery(e.target.value)}
        onBlur={() => setTimeout(() => setIsOpen(false), 150)}
        className="input"
      />
      {isOpen && (
        <div className="card absolute z-10 mt-1 max-h-56 w-full overflow-auto shadow-sm">
          {filtered.map((item) => (
            <button
              key={item.id}
              type="button"
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(item.id, item);
                setIsOpen(false);
              }}
              className="block w-full px-3 py-2 text-left text-sm hover:bg-surface-hover"
            >
              {item.label}
            </button>
          ))}
          {filtered.length === 0 && <p className="px-3 py-2 text-sm text-text-muted">No matches.</p>}
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onCreateNew();
              setIsOpen(false);
            }}
            className="flex w-full items-center gap-1.5 border-t border-border px-3 py-2 text-left text-sm text-accent hover:bg-surface-hover"
          >
            <PlusIcon className="h-3.5 w-3.5" />
            {createLabel}
          </button>
        </div>
      )}
    </div>
  );
}
