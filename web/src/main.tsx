import { BrowserRouter } from "react-router";
import { createRoot } from "react-dom/client";
import { Analytics } from "@vercel/analytics/react";
import { diagnostics } from "./diagnostics";
import { startWithMechanics } from "./mechanicsStartup";
import "./lab.css";
import "./menu.css";
import "./hud.css";

/** Whether this is a Vercel build (`vite.config.ts`). */
declare const __ON_VERCEL__: boolean;

// Before anything else, so a failure while starting is in the report.
diagnostics.start();
void startWithMechanics(import.meta.env.DEV, async () => {
  const [{ AppShell }, { applyHudTheme }] = await Promise.all([
    import("@apps/battle-lab/src/AppShell"),
    import("./battle/present/hudTheme"),
  ]);
  applyHudTheme();
  createRoot(document.getElementById("root")!).render(
    <>
      <BrowserRouter unstable_useTransitions={false}>
        <AppShell />
      </BrowserRouter>
      {/* Vercel's builds only: its edge serves the script. In development the
          debug script is cross-origin, which the page's cross-origin isolation
          blocks, and a local production build has no such route (a 404); both
          are page errors to every browser scene. */}
      {__ON_VERCEL__ && <Analytics />}
    </>,
  );
}).catch((error: Error) => {
  document.getElementById("root")!.textContent = error.message;
});
