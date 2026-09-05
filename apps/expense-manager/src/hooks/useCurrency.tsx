import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { getBusiness } from "../lib/tauri/commands";
import { formatMinor as formatMinorUnits, parseAmountToMinor } from "../lib/currency";

const DEFAULT_SYMBOL = "₹";

const CurrencyContext = createContext<{ symbol: string; refresh: () => void } | null>(null);

/**
 * The one legitimate cross-cutting exception to prop-drilling money
 * formatting through the whole tree (mirrors Billing's `CurrencyProvider`) —
 * `business.currency_symbol` (database-schema.md §5) is read by nearly every
 * screen that shows money. Mounted once at the app root.
 */
export function CurrencyProvider({ children }: { children: ReactNode }) {
  const [symbol, setSymbol] = useState(DEFAULT_SYMBOL);

  const refresh = useCallback(() => {
    getBusiness()
      .then((business) => setSymbol(business?.currency_symbol ?? DEFAULT_SYMBOL))
      .catch(() => {
        // No business yet (first-run gate) — keep the default.
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return <CurrencyContext.Provider value={{ symbol, refresh }}>{children}</CurrencyContext.Provider>;
}

export function useCurrency() {
  const ctx = useContext(CurrencyContext);
  if (!ctx) throw new Error("useCurrency must be used within a CurrencyProvider");
  return {
    symbol: ctx.symbol,
    formatMinor: formatMinorUnits,
    parseToMinor: parseAmountToMinor,
    refresh: ctx.refresh,
  };
}
