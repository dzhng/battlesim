// Isolated screens need the entry's styles without mounting its application.
import "../../src/lab.css";
import "../../src/menu.css";
import "../../src/hud.css";
import type { ReactNode } from "react";
export { createElement } from "react";
export { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { PageVisit } from "@apps/battle-lab/src/navigation";

/** The entry's browser history and visit wiring, for isolated screen checks. */
export function BrowserVisit({ children }: { children: ReactNode }) {
  return (
    <BrowserRouter unstable_useTransitions={false}>
      <PageVisit>{children}</PageVisit>
    </BrowserRouter>
  );
}
