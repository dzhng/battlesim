// The gallery's finished match, read from either seat through the real
// result component. Kept browser-side so Vite resolves React normally.
import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { MatchResultOverlay } from "../../apps/battle-lab/src/MatchResultOverlay.tsx";
import { reportSpecimen } from "../../apps/battle-lab/src/reportSpecimens.ts";
import { SessionCatalogProvider } from "../src/battle/catalog/context.tsx";
import { catalogSet } from "../src/battle/catalog/sets.ts";

export async function showResult(viewer) {
  const specimen = reportSpecimen("h", "h", "red");
  const mount = document.createElement("div");
  document.body.replaceChildren(mount);
  createRoot(mount).render(
    createElement(
      SessionCatalogProvider,
      { catalog: await catalogSet("test") },
      createElement(MatchResultOverlay, { ...specimen, viewer }),
    ),
  );
}
