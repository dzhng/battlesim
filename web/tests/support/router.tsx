import type { ReactNode } from "react";
import {
  act,
  render as renderView,
  type RenderOptions,
  type RenderResult,
} from "@testing-library/react";
import { BrowserVisit } from "./browserVisit";

export function renderInRouter(ui: ReactNode) {
  return renderView(ui, { wrapper: BrowserVisit });
}

/** Render `ui` once the catalog sets its pages read have resolved. A page
 *  under `SessionCatalogScope` (every route, and the menu's backdrop) suspends
 *  on its set with `use`, and under a synchronous `act` React never retries
 *  it: the suspended part stays uncommitted, and so does every later update
 *  that re-renders it. Resolving the sets inside an awaited `act` lets those
 *  renders commit before the test interacts. */
export async function renderSettled(ui: ReactNode, options?: RenderOptions): Promise<RenderResult> {
  // Imported here, not at the top: a test that resets modules renders a
  // fresh router, whose set cache is the fresh module's.
  const { catalogSet } = await import("@web/battle/catalog/sets");
  let view!: RenderResult;
  await act(async () => {
    view = renderView(ui, options);
    await Promise.allSettled([catalogSet("game"), catalogSet("test"), catalogSet("menu")]);
  });
  return view;
}

/** `renderInRouter`, settled (`renderSettled`). */
export function visitInRouter(ui: ReactNode): Promise<RenderResult> {
  return renderSettled(ui, { wrapper: BrowserVisit });
}
