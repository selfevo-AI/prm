import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { SharedVault } from "./Permissions";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {window.location.pathname === "/shared" ? <SharedVault /> : <App />}
  </React.StrictMode>,
);
