import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { getSettings } from "../lib/tauri/commands";
import { currencyMeta } from "../lib/currency";
import { formatMinorAsAmount, parseAmountToMinor } from "../lib/tauri/types";

interface CurrencyState {
  code: string;
  symbol: string;
  decimals: number;
}

const DEFAULT_STATE: CurrencyState = { code: "INR", symbol: "₹", decimals: 2 };

const CurrencyContext = createContext<{ state: CurrencyState; refresh: () => void } | null>(null);

/**
 * The one legitimate cross-cutting exception to "no global client-state
 * store" (ui-ux.md §7) — the active currency is read by nearly every screen
 * that shows money, so prop-drilling it through the whole tree would be far
 * more invasive than the "no store" rule was written to prevent (that rule
 * targets feature data like invoices/customers, not read-mostly app config).
 * Mounted once at the app root; `refresh()` re-reads Settings after the
 * Invoicing tab changes `currency_code`, so the change applies app-wide
 * without a restart.
 */
export function CurrencyProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<CurrencyState>(DEFAULT_STATE);

  const refresh = useCallback(() => {
    getSettings()
      .then((settings) => {
        const meta = currencyMeta(settings.currency_code);
        setState({ code: settings.currency_code, symbol: meta.symbol, decimals: meta.decimals });
      })
      .catch(() => {
        // Business/settings aren't guaranteed to exist yet (first-run gate) — keep the default.
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return <CurrencyContext.Provider value={{ state, refresh }}>{children}</CurrencyContext.Provider>;
}

export function useCurrency() {
  const ctx = useContext(CurrencyContext);
  if (!ctx) throw new Error("useCurrency must be used within a CurrencyProvider");
  const { decimals } = ctx.state;
  // Stable identities (tied only to `decimals`) so effects that legitimately
  // depend on these — e.g. InvoiceEditor's initial-load effect — can list
  // them without either an exhaustive-deps warning or a re-run on every
  // unrelated render.
  const formatMinor = useCallback((minor: number) => formatMinorAsAmount(minor, decimals), [decimals]);
  const parseToMinor = useCallback((input: string) => parseAmountToMinor(input, decimals), [decimals]);
  return {
    code: ctx.state.code,
    symbol: ctx.state.symbol,
    decimals,
    formatMinor,
    parseToMinor,
    refresh: ctx.refresh,
  };
}
