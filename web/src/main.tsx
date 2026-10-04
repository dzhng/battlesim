import { BrowserRouter } from "react-router";
import { createRoot } from "react-dom/client";
import { startWithMechanics } from "./mechanicsStartup";
import "./lab.css";
import "./menu.css";
import "./hud.css";

void startWithMechanics(import.meta.env.DEV, async () => {
  const [{ LabRouter }, { applyHudTheme }] = await Promise.all([
    import("@apps/battle-lab/src/router"),
    import("./battle/present/hudTheme"),
  ]);
  applyHudTheme();
  createRoot(document.getElementById("root")!).render(
    <BrowserRouter unstable_useTransitions={false}>
      <LabRouter />
    </BrowserRouter>,
  );
}).catch((error: Error) => {
  document.getElementById("root")!.textContent = error.message;
});
