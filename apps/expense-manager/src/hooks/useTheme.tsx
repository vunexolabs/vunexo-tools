import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

const STORAGE_KEY = "expense-manager:theme";

type Theme = "light" | "dark";

function prefersDark(): boolean {
  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  } catch {
    return false;
  }
}

function readStoredTheme(): Theme | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored === "light" || stored === "dark" ? stored : null;
  } catch {
    return null;
  }
}

function writeStoredTheme(theme: Theme): void {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // Best-effort only — a webview that blocks storage still gets a working
    // theme for the session, it just won't persist across restarts.
  }
}

function applyThemeClass(theme: Theme): void {
  document.documentElement.classList.toggle("dark", theme === "dark");
}

const ThemeContext = createContext<{ theme: Theme; toggle: () => void } | null>(null);

/**
 * Mirrors `CurrencyProvider`'s exact context pattern. Reads the persisted
 * choice on mount, falling back to the OS preference; applies/removes the
 * `dark` class on `<html>` so every `dark:` utility and the CSS-variable
 * token overrides in `index.css` respond consistently app-wide.
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(() => readStoredTheme() ?? (prefersDark() ? "dark" : "light"));

  useEffect(() => {
    applyThemeClass(theme);
  }, [theme]);

  const toggle = useCallback(() => {
    setTheme((current) => {
      const next: Theme = current === "dark" ? "light" : "dark";
      writeStoredTheme(next);
      return next;
    });
  }, []);

  return <ThemeContext.Provider value={{ theme, toggle }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within a ThemeProvider");
  return ctx;
}
