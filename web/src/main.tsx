import { createRoot } from "react-dom/client";
import { LabRouter } from "@apps/battle-lab/src/router";
import { applyHudTheme } from "./battle/present/hudTheme";
import "./lab.css";
import "./menu.css";
import "./hud.css";

const path = window.location.pathname.replace(/\/$/, "") || "/";

applyHudTheme();
createRoot(document.getElementById("root")!).render(<LabRouter path={path} />);
