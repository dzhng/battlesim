import { renderInRouter as render } from "./support/router";
import { LAB_FIXTURES } from "@apps/battle-lab/src/fixtures";
import { createElement } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { LabRouter } from "@apps/battle-lab/src/router";
import { MainMenu } from "@apps/battle-lab/src/MainMenu";
import * as catalogue from "@web/maps/catalogue";

afterEach(() => vi.restoreAllMocks());

const entry = (id: string, status: catalogue.MapStatus): catalogue.MapEntry => ({
  id,
  category: "playable",
  status,
  label: `Battlefield ${id}`,
  character: "mixed",
  biome: "summer",
  size_m: [6000, 6000],
  tags: [],
  source: "authored",
  seed: null,
  encounters: ["assault"],
  benchmarks: [],
});

test("a catalogue draft is inspectable in labs by name and absent from player battles", () => {
  const entries = [
    entry("released-field", "released"),
    entry("draft-field", "draft"),
    entry("retired-field", "retired"),
    { ...entry("released-arena", "released"), category: "lab" as const },
    { ...entry("other-encounter", "released"), encounters: ["duel"] },
  ];
  vi.spyOn(catalogue, "listMaps").mockImplementation((filter) =>
    catalogue.filterMaps(entries, filter),
  );
  window.history.replaceState(null, "", "/labs");
  const labs = render(createElement(LabRouter));
  expect(labs.getByRole("link", { name: "Battlefield draft-field" }).getAttribute("href")).toBe(
    "/lab/geometry?map=draft-field",
  );
  expect(labs.getByRole("link", { name: "Battlefield released-field" }).getAttribute("href")).toBe(
    "/lab/geometry?map=released-field",
  );
  expect(labs.queryByRole("link", { name: "Battlefield retired-field" })).toBeNull();
  expect(labs.getByRole("link", { name: "workbench" }).getAttribute("href")).toBe("/workbench");
  labs.unmount();
  window.history.replaceState(null, "", "/");
  const menu = render(createElement(MainMenu));
  expect(
    menu.getByRole("link", { name: "Play Battlefield released-field" }).getAttribute("href"),
  ).toBe("/battle?map=released-field&recipe=assault");
  expect(menu.queryByRole("link", { name: "Play Battlefield draft-field" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Play Battlefield retired-field" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Play Battlefield released-arena" })).toBeNull();
  expect(menu.queryByRole("link", { name: "Play Battlefield other-encounter" })).toBeNull();
  menu.unmount();
});

test("every saved route names an existing catalogue map and every active map has an inspection link", () => {
  const maps = catalogue.listMaps();
  const ids = maps.map((map) => map.id);
  for (const fixture of LAB_FIXTURES) {
    if (fixture.map !== null) expect(ids, `${fixture.id}'s saved map`).toContain(fixture.map);
  }
  for (const map of maps)
    for (const benchmark of map.benchmarks)
      expect(LAB_FIXTURES.find((fixture) => fixture.id === benchmark)?.map).toBe(map.id);
  window.history.replaceState(null, "", "/labs");
  const labs = render(createElement(LabRouter));
  for (const map of maps.filter((map) => map.status !== "retired"))
    expect(labs.getByRole("link", { name: map.label }).getAttribute("href")).toBe(
      `/lab/geometry?map=${map.id}`,
    );
  labs.unmount();
});
