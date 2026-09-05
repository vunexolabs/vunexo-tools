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
        className="w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 transition-colors focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
      />
      {isOpen && (
        <div className="absolute z-10 mt-1 max-h-56 w-full overflow-auto rounded-md border border-zinc-200 bg-white shadow-lg dark:border-zinc-800 dark:bg-zinc-900">
          {filtered.map((item) => (
            <button
              key={item.id}
              type="button"
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(item.id, item);
                setIsOpen(false);
              }}
              className="block w-full px-3 py-2 text-left text-sm text-zinc-900 hover:bg-zinc-100 dark:text-zinc-100 dark:hover:bg-zinc-800"
            >
              {item.label}
            </button>
          ))}
          {filtered.length === 0 && <p className="px-3 py-2 text-sm text-zinc-500 dark:text-zinc-500">No matches.</p>}
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onCreateNew();
              setIsOpen(false);
            }}
            className="block w-full border-t border-zinc-200 px-3 py-2 text-left text-sm text-blue-600 hover:bg-zinc-100 dark:border-zinc-800 dark:text-blue-400 dark:hover:bg-zinc-800"
          >
            + {createLabel}
          </button>
        </div>
      )}
    </div>
  );
}
