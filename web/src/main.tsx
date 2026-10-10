import { BrowserRouter } from "react-router";
import { createRoot } from "react-dom/client";
import { diagnostics } from "./diagnostics";
import { startWithMechanics } from "./mechanicsStartup";
import "./lab.css";
import "./menu.css";
import "./hud.css";

// Before anything else, so a failure while starting is in the report.
diagnostics.start();
void startWithMechanics(import.meta.env.DEV, async () => {
  const [{ AppShell }, { applyHudTheme }] = await Promise.all([
    import("@apps/battle-lab/src/AppShell"),
    import("./battle/present/hudTheme"),
  ]);
  applyHudTheme();
  createRoot(document.getElementById("root")!).render(
    <BrowserRouter unstable_useTransitions={false}>
      <AppShell />
    </BrowserRouter>,
  );
}).catch((error: Error) => {
  document.getElementById("root")!.textContent = error.message;
});
