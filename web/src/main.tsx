import { createRoot } from "react-dom/client";
import { LabRouter } from "@apps/battle-lab/src/router";
import "./lab.css";
import "./menu.css";

const path = window.location.pathname.replace(/\/$/, "") || "/";

createRoot(document.getElementById("root")!).render(<LabRouter path={path} />);
