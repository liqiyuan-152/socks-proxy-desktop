import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { syncSystemTheme } from "@/lib/system-theme";
import "./index.css";

syncSystemTheme();

createRoot(document.getElementById("app")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
