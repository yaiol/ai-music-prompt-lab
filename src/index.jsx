import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import pkg from "../package.json";
import { loadLang } from "./i18n-gen";
import "./assets/ui-colors.css"; // canonical color-token palette (light default + [data-theme=dark]) — MUST be first (synced from .common/shared)
import "./assets/ui-listview.css"; // shared canonical list-within-a-view styling (synced from .common/shared)
import "./assets/ui-app.css"; // shared canonical header-bar + button chrome (synced from .common/shared)

// ⚠ CLAUDE: the active language chunk is awaited BEFORE the first render, so a non-English
// user never sees a flash of English. `.finally` (not `.then`) so a failed or unknown language
// still renders, in English. src/i18n-gen/ is GENERATED — see scripts/i18n-split.mjs.
const root = ReactDOM.createRoot(document.getElementById("root"));
loadLang(localStorage.getItem(`${pkg.storagePrefix}-lang`) || "en")
  .finally(() => root.render(<App />));
