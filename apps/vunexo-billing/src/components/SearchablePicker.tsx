import { useEffect, useRef, useState } from "react";

interface PickerItem {
  id: number;
  label: string;
}

/**
 * ui-ux.md §3 — "customer and product selection ... share one component —
 * type-to-filter over list_customers/list_products (active only), with a
 * persistent '+ Create new…' row at the bottom that opens the same
 * Customer/Product Detail form inline" (user-flows.md §3/§4's dual
 * entry-point rule). Generic over the item shape so both pickers reuse it.
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
        className="w-full rounded border border-slate-700 bg-slate-950 px-3 py-2 text-sm"
      />
      {isOpen && (
        <div className="absolute z-10 mt-1 max-h-56 w-full overflow-auto rounded border border-slate-700 bg-slate-900 shadow-lg">
          {filtered.map((item) => (
            <button
              key={item.id}
              type="button"
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(item.id, item);
                setIsOpen(false);
              }}
              className="block w-full px-3 py-2 text-left text-sm hover:bg-slate-800"
            >
              {item.label}
            </button>
          ))}
          {filtered.length === 0 && <p className="px-3 py-2 text-sm text-slate-500">No matches.</p>}
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onCreateNew();
              setIsOpen(false);
            }}
            className="block w-full border-t border-slate-800 px-3 py-2 text-left text-sm text-sky-400 hover:bg-slate-800"
          >
            + {createLabel}
          </button>
        </div>
      )}
    </div>
  );
}
