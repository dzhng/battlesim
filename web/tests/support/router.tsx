import type { ReactNode } from "react";
import { render as renderView } from "@testing-library/react";
import { BrowserVisit } from "./browserVisit";

export function renderInRouter(ui: ReactNode) {
  return renderView(ui, { wrapper: BrowserVisit });
}
