import { visitInRouter as visit } from "./support/router";
// @vitest-environment jsdom
import { LAB_FIXTURES } from "@apps/battle-lab/src/fixtures";
import { createElement } from "react";
import { cleanup, fireEvent } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { LabRouter } from "@apps/battle-lab/src/router";

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  window.history.replaceState(null, "", "/");
});

test("the main menu at / opens each of its pages inside its plate, and Back returns to the list; the lab index is /labs", async () => {
  vi.stubEnv("DEV", true);
  vi.resetModules();
  const { LabRouter: DevelopmentRouter } = await import("@apps/battle-lab/src/router");
  const menu = await visit(createElement(DevelopmentRouter));
  const links = () => menu.queryAllByRole("link").map((a) => a.getAttribute("href"));
  const pages = ["Skirmish", "Watch replay", "Settings", "Developer"];
  expect(links()).toEqual([]);
  for (const page of pages) expect(menu.getByRole("button", { name: page })).toBeTruthy();
  expect(menu.queryByRole("button", { name: "Back" })).toBeNull();

  /** Open `page` from the list: the plate shows only it, under a Back button. */
  const open = (page: string) => {
    fireEvent.click(menu.getByRole("button", { name: page }));
    expect(menu.getByRole("heading", { name: page })).toBeTruthy();
    for (const other of pages) expect(menu.queryByRole("button", { name: other })).toBeNull();
  };
  const entries: Record<string, Record<string, string>> = {
    Skirmish: { Deploy: "/battle?play=1&type=mixed&size=small&faction=us" },
    // No battle saved on this browser: only the file to load one from.
    "Watch replay": {},
    Settings: { "Graphics test": "/graphics-test" },
    Developer: {
      "Mechanics editor": "/mechanics",
      "Map workbench": "/map-workbench",
      "Sound workbench": "/sound-workbench",
      Benchmark: "/benchmark",
      Labs: "/labs",
    },
  };
  for (const [page, expected] of Object.entries(entries)) {
    open(page);
    expect(links()).toEqual(Object.values(expected));
    for (const [name, href] of Object.entries(expected)) {
      expect(menu.getByRole("link", { name }).getAttribute("href")).toBe(href);
    }
    fireEvent.click(menu.getByRole("button", { name: "Back" }));
    expect(menu.getByRole("button", { name: page })).toBeTruthy();
  }
  open("Watch replay");
  expect(menu.getByTestId("replay-file")).toBeTruthy();
  fireEvent.click(menu.getByRole("button", { name: "Back" }));
  open("Settings");
  expect(menu.getByTestId("sound-controls")).toBeTruthy();
  expect(menu.getByRole("link", { name: "Graphics test" }).getAttribute("href")).toBe(
    "/graphics-test",
  );
  // Escape is Back, as in a game's menus.
  fireEvent.keyDown(window, { key: "Escape" });
  expect(menu.getByRole("button", { name: "Settings" })).toBeTruthy();
  menu.unmount();
  window.history.replaceState(null, "", "/labs");
  const index = await visit(createElement(LabRouter));
  const hrefs = index.getAllByRole("link").map((a) => a.getAttribute("href"));
  for (const f of LAB_FIXTURES) expect(hrefs).toContain(f.route);
  index.unmount();
});

test("the production menu does not offer source editing", async () => {
  vi.stubEnv("DEV", false);
  vi.resetModules();
  const { MainMenu } = await import("@apps/battle-lab/src/MainMenu");
  const menu = await visit(createElement(MainMenu));
  fireEvent.click(menu.getByRole("button", { name: "Developer" }));
  expect(menu.queryByRole("link", { name: "Mechanics editor" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Map workbench" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Sound workbench" })).toBeNull();
  expect(menu.getByRole("link", { name: "Benchmark" }).getAttribute("href")).toBe("/benchmark");
  menu.unmount();
});
