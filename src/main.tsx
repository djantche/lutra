import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter";
import "@fontsource-variable/space-grotesk";
import App from "./App";
import { OverlayApp } from "./overlay/OverlayApp";
import "./i18n";
import "./styles/main.scss";
import { applyAccessibility } from "./lib/accessibility";

applyAccessibility();

// The in-game overlay is the same app opened in its own window (`?overlay=1`).
const overlay = new URLSearchParams(window.location.search).get("overlay") === "1";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{overlay ? <OverlayApp /> : <App />}</React.StrictMode>,
);
