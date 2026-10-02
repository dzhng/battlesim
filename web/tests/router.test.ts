// @vitest-environment jsdom
import { createElement } from "react";
import { fireEvent, render } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import { LAB_FIXTURES, LabRouter, ROUTES } from "@apps/battle-lab/src/router";

afterEach(() => vi.unstubAllEnvs());

test("every registered fixture has a page and every page a fixture", () => {
  expect(Object.keys(ROUTES).sort()).toEqual(fixtures.map((f) => f.id).sort());
});

test("the main menu at / offers play and replay, development tools behind its developer link; the lab index is /labs", async () => {
  vi.stubEnv("DEV", true);
  vi.resetModules();
  const { LabRouter: DevelopmentRouter } = await import("@apps/battle-lab/src/router");
  const menu = render(createElement(DevelopmentRouter, { path: "/" }));
  expect(menu.getAllByRole("link").map((a) => a.getAttribute("href"))).toEqual([
    "/battle/village",
    "/replay/village",
  ]);
  fireEvent.click(menu.getByRole("button", { name: "Developer" }));
  const entries = {
    "Play village": "/battle/village",
    "Watch replay": "/replay/village",
    "Mechanics editor": "/mechanics",
    Benchmark: "/benchmark",
    Labs: "/labs",
  };
  expect(menu.getAllByRole("link")).toHaveLength(Object.keys(entries).length);
  for (const [name, href] of Object.entries(entries)) {
    // Each entry is one link named by its title; its description is inside
    // the same link, so a click on it navigates too.
    const link = menu.getByRole("link", { name });
    expect(link.getAttribute("href")).toBe(href);
    const note = document.getElementById(link.getAttribute("aria-describedby")!);
    expect(note?.textContent).toBeTruthy();
    expect(note?.closest("a")).toBe(link);
  }
  menu.unmount();
  const index = render(createElement(LabRouter, { path: "/labs" }));
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
  expect(menu.getByRole("link", { name: "Benchmark" }).getAttribute("href")).toBe("/benchmark");
  menu.unmount();
});

test("fixture ids and routes are unique and builds are known", () => {
  expect(new Set(LAB_FIXTURES.map((f) => f.id)).size).toBe(LAB_FIXTURES.length);
  expect(new Set(LAB_FIXTURES.map((f) => f.route)).size).toBe(LAB_FIXTURES.length);
  for (const f of fixtures as { build?: string }[])
    expect([undefined, "production"]).toContain(f.build);
});
