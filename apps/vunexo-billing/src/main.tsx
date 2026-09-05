import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app/App";
import { CurrencyProvider } from "./hooks/useCurrency";
import { ThemeProvider } from "./hooks/useTheme";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <CurrencyProvider>
        <App />
      </CurrencyProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
