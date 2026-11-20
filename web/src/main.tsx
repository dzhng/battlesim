import { BrowserRouter } from "react-router";
import { createRoot } from "react-dom/client";
import { Analytics } from "@vercel/analytics/react";
import { diagnostics } from "./diagnostics";
import { startWithMechanics } from "./mechanicsStartup";
import { isChromium } from "./browserSupport";
import type { HudTheme } from "./battle/present/hudTheme";
import "./lab.css";
import "./menu.css";
import "./hud.css";
import "./armory.css";

/** Whether this is a Vercel build (`vite.config.ts`). */
declare const __ON_VERCEL__: boolean;

// Before anything else, so a failure while starting is in the report.
diagnostics.start();
if (!isChromium(navigator.userAgent)) void showUnsupportedBrowser();
else
  void startWithMechanics(import.meta.env.DEV, async () => {
    // Camera and HUD consumers can be evaluated while their shared
    // presentation settings are still awaiting WebAssembly initialization.
    await import("./presentationSettings");
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

/** A browser the game doesn't run in gets the notice in its place, in the
 *  HUD's look but with none of the game behind it (`browserSupport.ts`). */
async function showUnsupportedBrowser() {
  const [{ UnsupportedBrowser }, { hudProperties, applyHudProperties }, { default: presentation }] =
    await Promise.all([
      import("@apps/battle-lab/src/UnsupportedBrowser"),
      import("./battle/present/hudCss"),
      import("../../fixtures/presentation.json"),
    ]);
  const { hud, overlay } = presentation;
  applyHudProperties(
    hudProperties(hud as unknown as HudTheme, overlay.orders.selected, overlay.glow.callouts),
  );
  createRoot(document.getElementById("root")!).render(<UnsupportedBrowser />);
}
