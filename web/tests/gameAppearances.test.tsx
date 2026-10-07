import { renderInRouter as render } from "./support/router";
import { cleanup, fireEvent, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { TEST_CATALOG } from "./catalog";

/** A view under the test set, through the context module the freshly
 *  imported page modules read (`vi.resetModules` gives each test its own). */
async function underTestCatalog() {
  const { SessionCatalogProvider } = await import("@web/battle/catalog/context");
  return ({ children }: { children: ReactNode }) => (
    <SessionCatalogProvider catalog={TEST_CATALOG}>{children}</SessionCatalogProvider>
  );
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.resetModules();
});

test("required appearance rejection replaces loading with a recoverable refusal", async () => {
  vi.stubGlobal("fetch", async () => {
    throw new Error("Required appearance unavailable");
  });
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { useGameAppearances } = await import("@apps/battle-lab/src/gameAppearances");
  const Scope = await underTestCatalog();
  function World() {
    const appearances = useGameAppearances();
    return <div>{appearances ? "Battlefield ready" : "Loading appearance"}</div>;
  }
  const view = render(
    <AppResourceBoundary menu={false}>
      <Scope>
        <World />
      </Scope>
    </AppResourceBoundary>,
  );
  await waitFor(() => expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy());
  expect(view.queryByText("Loading appearance")).toBeNull();
  expect(view.getByRole("link", { name: "Back to the menu" }).getAttribute("href")).toBe("/");
  view.rerender(
    <AppResourceBoundary menu={true}>
      <div>Main menu remains available</div>
    </AppResourceBoundary>,
  );
  expect(view.getByText("Main menu remains available")).toBeTruthy();
});

test("a demanded building kit refuses play even after the shared catalog loaded", async () => {
  vi.stubGlobal("fetch", async () =>
    Response.json({ appearances: {}, skeletons: {}, sides: { blue: [0, 0, 1], red: [1, 0, 0] } }),
  );
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { gameAppearances, useGameAppearances } =
    await import("@apps/battle-lab/src/gameAppearances");
  const base = await gameAppearances(TEST_CATALOG.units);
  expect(base.appearances.size).toBe(0);
  const kits = new Set(["required-building-kit"]);
  const Scope = await underTestCatalog();
  function World() {
    useGameAppearances(kits);
    return <div>Loading required kit</div>;
  }
  const view = render(
    <AppResourceBoundary menu={false}>
      <Scope>
        <World />
      </Scope>
    </AppResourceBoundary>,
  );
  await waitFor(() => expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy());
  expect(view.queryByText("Loading required kit")).toBeNull();
});

test("document departure releases the active view and restored documents require manual reload", async () => {
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const view = render(
    <AppResourceBoundary menu={false}>
      <div>Active battle</div>
    </AppResourceBoundary>,
  );
  fireEvent(window, new Event("pagehide"));
  fireEvent(window, new Event("pageshow"));
  expect(view.queryByText("Active battle")).toBeNull();
  expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy();
  expect(view.getByRole("button", { name: "Reload" }).closest("main")).toBe(
    view.getByTestId("loading"),
  );
});

test("app boundary effect refresh leaves page resources usable until pagehide", async () => {
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const first = render(
    <AppResourceBoundary menu={false}>
      <div>First view</div>
    </AppResourceBoundary>,
  );
  first.unmount();
  const next = render(
    <AppResourceBoundary menu={false}>
      <div>Fresh view</div>
    </AppResourceBoundary>,
  );
  expect(next.getByText("Fresh view")).toBeTruthy();
  fireEvent(window, new Event("pagehide"));
  expect(next.queryByText("Fresh view")).toBeNull();
  expect(next.getByRole("heading", { name: "Aborted" })).toBeTruthy();
});
