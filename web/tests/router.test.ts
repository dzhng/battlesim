// @vitest-environment jsdom
import { createElement } from "react";
import { fireEvent, render } from "@testing-library/react";
import { expect, test } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import { LAB_FIXTURES, LabRouter, ROUTES } from "@apps/battle-lab/src/router";

test("every registered fixture has a page and every page a fixture", () => {
  expect(Object.keys(ROUTES).sort()).toEqual(fixtures.map((f) => f.id).sort());
});

test("the main menu at / offers a new battle, the village, the saved battlefield and replay, the benchmark and labs behind its developer link; the lab index is /labs", () => {
  const menu = render(createElement(LabRouter, { path: "/" }));
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
