// The Armory page: a pick changes only the page's own state and the address
// it mirrors, never the visit; an unknown unit opens the first tile; and the
// tabs hold exactly what each faction fields, each family once, under its
// own group. Every card of the catalog, never one roster entry.
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, test } from "vitest";
import { FACTIONS } from "@packages/scene-assets/src/units";
import { ArmoryPage } from "@apps/battle-lab/src/armory/ArmoryPage";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { spoken } from "@apps/battle-lab/src/battleLinks";
import { UNIT_GROUPS, unitGroup } from "@web/battle/present/purchaseFilter";
import { UNITS, WithTestCatalog } from "./catalog";
import { visitApp } from "./support/router";
import { menuPage } from "@apps/battle-lab/src/menu/pages";

beforeEach(() => graphicsSettings.set({ detectedMode: "medium" }, { persist: false }));
afterEach(() => {
  cleanup();
  graphicsSettings.set({ detectedMode: null }, { persist: false });
  window.history.replaceState(null, "", "/");
});

const unit = () => new URLSearchParams(location.search).get("unit");
const list = () => screen.getByRole("region", { name: "Units" });
const tiles = () =>
  within(list()).getByRole("tabpanel").querySelectorAll<HTMLButtonElement>("button[data-family]");
const families = () => [...tiles()].map((t) => t.dataset.family!);

test("a pick replaces the address and never remounts the page", async () => {
  await visitApp(menuPage("armory").path);
  const page = await screen.findByRole("region", { name: "Units" });
  const length = history.length;
  const opened = unit();
  const other = [...tiles()].find((t) => t.getAttribute("aria-pressed") !== "true")!;
  await act(async () => fireEvent.click(other));
  expect(unit()).not.toBe(opened);
  expect(other.getAttribute("aria-pressed")).toBe("true");
  // A group tab and a faction tab, too.
  const groups = within(page).getByRole("tablist", { name: "Unit groups" });
  const next = within(groups)
    .getAllByRole("tab")
    .find((tab) => tab.getAttribute("aria-selected") !== "true" && !tab.hasAttribute("disabled"))!;
  await act(async () => fireEvent.click(next));
  expect(next.getAttribute("aria-selected")).toBe("true");
  expect(screen.getByRole("region", { name: "Units" })).toBe(page);
  expect(history.length).toBe(length);
  expect(location.pathname).toBe(menuPage("armory").path);
});

test("an unknown unit falls back to the first tile and rewrites the address", async () => {
  await visitApp(`${menuPage("armory").path}?unit=no_such_card`);
  await screen.findByRole("region", { name: "Units" });
  const first = tiles()[0];
  expect(first.getAttribute("aria-pressed")).toBe("true");
  const card = UNITS.cards.find((c) => c.id === unit());
  expect(card?.family).toBe(first.dataset.family);
  // The first group with a tile, under the first faction with one.
  const groups = within(list()).getByRole("tablist", { name: "Unit groups" });
  const open = within(groups)
    .getAllByRole("tab")
    .find((tab) => !tab.hasAttribute("disabled"));
  expect(open?.getAttribute("aria-selected")).toBe("true");
});

/** Every family each group tab of `faction` shows, by group. */
function browse(faction: (typeof FACTIONS)[number]) {
  fireEvent.click(screen.getByRole("tab", { name: spoken(faction) }));
  const shown = new Map<string, string[]>();
  for (const { id, label } of UNIT_GROUPS) {
    const tab = screen.getByRole("tab", { name: label });
    if (tab.hasAttribute("disabled")) continue;
    fireEvent.click(tab);
    shown.set(id, families());
  }
  return shown;
}

test("each faction tab lists exactly the cards that faction fields", () => {
  render(
    <WithTestCatalog>
      <ArmoryPage cards={UNITS.cards} />
    </WithTestCatalog>,
  );
  for (const faction of FACTIONS) {
    const fielded = new Set(
      UNITS.cards.filter((c) => c.roster.factions.includes(faction)).map((c) => c.family),
    );
    const shown = [...browse(faction).values()].flat();
    // Each family once, under one group: a family never splits across tabs.
    expect(shown.length, faction).toBe(new Set(shown).size);
    expect(new Set(shown), faction).toEqual(fielded);
  }
});

test("each group tab shows only its own group's cards", () => {
  render(
    <WithTestCatalog>
      <ArmoryPage cards={UNITS.cards} />
    </WithTestCatalog>,
  );
  for (const faction of FACTIONS)
    for (const [group, shown] of browse(faction)) {
      const own = UNIT_GROUPS.find((g) => g.id === group)!;
      for (const family of shown) {
        const variants = UNITS.cards.filter(
          (c) => c.family === family && c.roster.factions.includes(faction),
        );
        // Every variant is of the group's category, and at least one of
        // them browses under the group by its own type.
        for (const v of variants) expect(v.roster.category, v.id).toBe(own.category);
        expect(
          variants.some((v) => unitGroup(v, UNITS.has(v.id) ? UNITS.type(v.id) : null) === group),
          `${faction} ${group} ${family}`,
        ).toBe(true);
      }
    }
});

test("a sheet heads its weapons only when it lists one", () => {
  render(
    <WithTestCatalog>
      <ArmoryPage cards={UNITS.cards} />
    </WithTestCatalog>,
  );
  let unarmed = 0;
  for (const faction of FACTIONS) {
    fireEvent.click(screen.getByRole("tab", { name: spoken(faction) }));
    for (const { label } of UNIT_GROUPS) {
      const tab = screen.getByRole("tab", { name: label });
      if (tab.hasAttribute("disabled")) continue;
      fireEvent.click(tab);
      for (const family of families()) {
        fireEvent.click(list().querySelector(`button[data-family="${family}"]`)!);
        const sheet = document.querySelector<HTMLElement>(".armory-sheet")!;
        const heading = within(sheet).queryByRole("heading", { name: "Weapons" });
        const mounts = sheet.querySelectorAll(".armory-mount").length;
        expect(heading !== null, `${faction} ${family}: ${mounts} mounts`).toBe(mounts > 0);
        if (mounts === 0) unarmed++;
      }
    }
  }
  // The catalog fields an unarmed card, or this proves nothing.
  expect(unarmed).toBeGreaterThan(0);
});
