import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./assets/ui-colors.css"; // canonical color-token palette (light default + [data-theme=dark]) — MUST be first (synced from .common/shared)
import "./assets/ui-listview.css"; // shared canonical list-within-a-view styling (synced from .common/shared)
import "./assets/ui-app.css"; // shared canonical header-bar + button chrome (synced from .common/shared)

const root = ReactDOM.createRoot(document.getElementById("root"));
root.render(<App />);
