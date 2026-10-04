import { createRoot } from "react-dom/client";
import { startWithMechanics } from "./mechanicsStartup";
import "./lab.css";
import "./menu.css";
import "./hud.css";

const path = window.location.pathname.replace(/\/$/, "") || "/";

void startWithMechanics(import.meta.env.DEV, async () => {
  const [{ LabRouter }, { applyHudTheme }] = await Promise.all([
    import("@apps/battle-lab/src/router"),
    import("./battle/present/hudTheme"),
  ]);
  applyHudTheme();
  createRoot(document.getElementById("root")!).render(<LabRouter path={path} />);
}).catch((error: Error) => {
  document.getElementById("root")!.textContent = error.message;
});
