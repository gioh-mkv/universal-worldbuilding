import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

root.render(<p style={{ padding: 16 }}>React carregou. Importando o App…</p>);

import("./App")
  .then(({ default: App }) => {
    root.render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
  })
  .catch((e) => {
    root.render(
      <pre style={{ padding: 16, color: "crimson", whiteSpace: "pre-wrap" }}>
        {String(e?.stack ?? e)}
      </pre>,
    );
  });