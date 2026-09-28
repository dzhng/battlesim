// @vitest-environment jsdom
import { createElement } from "react";
import { render } from "@testing-library/react";
import { expect, test } from "vitest";
import fixtures from "@apps/battle-lab/src/fixtures.json";
import { LAB_FIXTURES, LabRouter, ROUTES } from "@apps/battle-lab/src/router";

test("every registered fixture has a page and every page a fixture", () => {
  expect(Object.keys(ROUTES).sort()).toEqual(fixtures.map((f) => f.id).sort());
});

test("the main menu at / offers play, replay, benchmark and labs; the lab index is /labs", () => {
  const menu = render(createElement(LabRouter, { path: "/" }));
  const entries = {
    "Play village": "/battle/village",
    "Watch replay": "/replay/village",
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

test("fixture ids and routes are unique and builds are known", () => {
  expect(new Set(LAB_FIXTURES.map((f) => f.id)).size).toBe(LAB_FIXTURES.length);
  expect(new Set(LAB_FIXTURES.map((f) => f.route)).size).toBe(LAB_FIXTURES.length);
  for (const f of fixtures as { build?: string }[])
    expect([undefined, "production"]).toContain(f.build);
});
