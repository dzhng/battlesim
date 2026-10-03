// @vitest-environment jsdom
import { LAB_FIXTURES } from "@apps/battle-lab/src/fixtures";
import { createElement } from "react";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import { LabRouter } from "@apps/battle-lab/src/router";

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
});

test("the main menu at / offers a new battle, the village, the saved battlefield and replay, the benchmark and labs behind its developer link; the lab index is /labs", async () => {
  vi.stubEnv("DEV", true);
  vi.resetModules();
  const { LabRouter: DevelopmentRouter } = await import("@apps/battle-lab/src/router");
  const menu = render(createElement(DevelopmentRouter, { path: "/" }));
  // The saved-file read finishes before its viewer link becomes available.
  await waitFor(() =>
    expect(menu.getByRole("link", { name: "Watch replay" }).getAttribute("href")).toBe(
      "/replay/village",
    ),
  );
  const links = () => menu.getAllByRole("link").map((a) => a.getAttribute("href"));
  expect(links()).toEqual([
    expect.stringMatching(/^\/battle\?type=mixed&size=small&seed=\d+$/),
    "/battle?map=market-town&recipe=assault",
    "/replay/village",
  ]);
  fireEvent.click(menu.getByRole("button", { name: "Developer" }));
  const entries = {
    "Play Market Town": "/battle?map=market-town&recipe=assault",
    "Watch replay": "/replay/village",
    Village: "/battle/village",
    "Mechanics editor": "/mechanics",
    Benchmark: "/benchmark",
    Labs: "/labs",
  };
  expect(links()).toHaveLength(Object.keys(entries).length + 1);
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

test("fixture ids and routes are unique and builds are known", () => {
  expect(new Set(LAB_FIXTURES.map((f) => f.id)).size).toBe(LAB_FIXTURES.length);
  expect(new Set(LAB_FIXTURES.map((f) => f.route)).size).toBe(LAB_FIXTURES.length);
  for (const f of fixtures as { build?: string }[])
    expect([undefined, "production"]).toContain(f.build);
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
