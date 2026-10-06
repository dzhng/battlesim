import { renderInRouter as render } from "./support/router";
// @vitest-environment jsdom
import { LAB_FIXTURES } from "@apps/battle-lab/src/fixtures";
import { createElement } from "react";
import { cleanup, fireEvent, waitFor } from "@testing-library/react";
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
  const menu = render(createElement(DevelopmentRouter));
  // The saved-file read finishes before its viewer link becomes available.
  await waitFor(() =>
    expect(menu.getByRole("link", { name: "Watch replay" }).getAttribute("href")).toBe(
      "/replay/village",
    ),
  );
  const links = () => menu.queryAllByRole("link").map((a) => a.getAttribute("href"));
  const pages = ["Skirmish", "Battlefields", "Settings", "Developer"];
  expect(links()).toEqual(["/replay/village"]);
  for (const page of pages) expect(menu.getByRole("button", { name: page })).toBeTruthy();
  expect(menu.queryByRole("button", { name: "Back" })).toBeNull();

  /** Open `page` from the list: the plate shows only it, under a Back button. */
  const open = (page: string) => {
    fireEvent.click(menu.getByRole("button", { name: page }));
    expect(menu.getByRole("heading", { name: page })).toBeTruthy();
    for (const other of pages) expect(menu.queryByRole("button", { name: other })).toBeNull();
  };
  const entries: Record<string, Record<string, string>> = {
    Skirmish: { Deploy: "/battle?play=1&type=mixed&size=small" },
    Battlefields: { "Play Market Town": "/battle?map=market-town&recipe=assault" },
    Settings: {},
    Developer: {
      "Mechanics editor": "/mechanics",
      "Map workbench": "/map-workbench",
      "Sound workbench": "/sound-workbench",
      Village: "/battle/village",
      Benchmark: "/benchmark",
      Labs: "/labs",
    },
  };
  for (const [page, expected] of Object.entries(entries)) {
    open(page);
    expect(links()).toEqual(Object.values(expected));
    for (const [name, href] of Object.entries(expected)) {
      // Each entry is one link named by its title; its description is inside
      // the same link, so a click on it navigates too.
      const link = menu.getByRole("link", { name });
      expect(link.getAttribute("href")).toBe(href);
      const note = document.getElementById(link.getAttribute("aria-describedby")!);
      expect(note?.textContent).toBeTruthy();
      expect(note?.closest("a")).toBe(link);
    }
    fireEvent.click(menu.getByRole("button", { name: "Back" }));
    expect(menu.getByRole("button", { name: page })).toBeTruthy();
  }
  open("Settings");
  expect(menu.getByTestId("sound-controls")).toBeTruthy();
  // Escape is Back, as in a game's menus.
  fireEvent.keyDown(window, { key: "Escape" });
  expect(menu.getByRole("button", { name: "Settings" })).toBeTruthy();
  menu.unmount();
  window.history.replaceState(null, "", "/labs");
  const index = render(createElement(LabRouter));
  const hrefs = index.getAllByRole("link").map((a) => a.getAttribute("href"));
  for (const f of LAB_FIXTURES) expect(hrefs).toContain(f.route);
  index.unmount();
});

test("the production menu does not offer source editing", async () => {
  vi.stubEnv("DEV", false);
  vi.resetModules();
  const { MainMenu } = await import("@apps/battle-lab/src/MainMenu");
  const menu = render(createElement(MainMenu));
  fireEvent.click(menu.getByRole("button", { name: "Developer" }));
  expect(menu.queryByRole("link", { name: "Mechanics editor" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Map workbench" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Sound workbench" })).toBeNull();
  expect(menu.getByRole("link", { name: "Benchmark" }).getAttribute("href")).toBe("/benchmark");
  menu.unmount();
});
