import type { ReactNode } from "react";
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
