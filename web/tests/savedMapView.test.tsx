import { createElement } from "react";
import { render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import Geometry from "@apps/battle-lab/src/routes/geometry";

// Acquisition is the edge: the real route must ask for the selected id.
vi.mock("@apps/battle-lab/src/savedMaps", () => ({
  SavedMap: ({ id }: { id: string }) =>
    createElement("output", { "aria-label": "Requested map" }, id),
}));
afterEach(() => window.history.replaceState(null, "", "/"));

test("the generic geometry viewer asks for its catalogue id instead of the default arena", () => {
  window.history.replaceState(null, "", "/lab/geometry?map=river");
  const view = render(createElement(Geometry));
  expect(view.getByLabelText("Requested map").textContent).toBe("river");
  view.unmount();
});
