// @vitest-environment jsdom
// The skirmish lobby on the menu's New Battle page: what it shows for the
// match being set up, which changes draw a new map, and that Deploy waits for
// the admitted map and then names exactly it. Preparation workers are fakes
// the test answers; everything above them is the real lobby.
import { createElement } from "react";
import { cleanup, fireEvent, waitFor, within, type RenderResult } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { MainMenu } from "@apps/battle-lab/src/MainMenu";
import { graphicsSettings } from "@apps/battle-lab/src/graphicsSettings";
import { askedBattle } from "@apps/battle-lab/src/battleLinks";
import { PreviewWorker } from "./support/previewWorker";
import { visitInRouter as visit } from "./support/router";

beforeEach(() => {
  PreviewWorker.all = [];
  vi.stubGlobal("Worker", PreviewWorker);
  vi.spyOn(console, "warn").mockImplementation(() => {});
  // A returning player: no first-visit graphics detection over the menu.
  graphicsSettings.set({ detectedMode: "medium" }, { persist: false });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  graphicsSettings.set({ detectedMode: null }, { persist: false });
  window.history.replaceState(null, "", "/");
});

/** The lobby, opened from the menu at `address`. */
async function openLobby(address = "/") {
  window.history.replaceState(null, "", address);
  const menu = await visit(createElement(MainMenu));
  if (address === "/") fireEvent.click(menu.getByRole("button", { name: "Skirmish" }));
  return menu;
}

/** The preparation started since `count` were. */
async function started(count: number): Promise<PreviewWorker> {
  // The first preview waits for the simulation module to load.
  await waitFor(() => expect(PreviewWorker.all.length).toBeGreaterThan(count), {
    timeout: 15_000,
  });
  expect(PreviewWorker.all).toHaveLength(count + 1);
  return PreviewWorker.all[count];
}

const side = (menu: RenderResult, name: "Blue" | "Red") =>
  within(menu.getByRole("group", { name: `${name} side` }));
const banners = (menu: RenderResult, name: "Blue" | "Red") =>
  side(menu, name)
    .getAllByRole("listitem")
    .map((li) => li.textContent);
const deploy = (menu: RenderResult) => menu.getByTestId("menu-deploy");
/** The battle Deploy's link names, or null while Deploy waits. */
const deployed = (menu: RenderResult) => {
  const href = deploy(menu).getAttribute("href");
  if (href === null) return null;
  const asked = askedBattle(new URL(href, "http://game").search);
  if ("error" in asked || asked.kind !== "generated") throw new Error(JSON.stringify(asked));
  return asked;
};

test("the lobby opens on a 1v1 of the asked map, and Deploy waits for that map", async () => {
  const menu = await openLobby("/?type=open&seed=77");
  expect(banners(menu, "Blue")).toEqual(["1You"]);
  expect(banners(menu, "Red")).toEqual(["1AI"]);
  expect(menu.getByTestId("lobby-vs").textContent).toBe("1 VS 1");
  expect(menu.getByTestId("lobby-status").textContent).toBe("Generating map");
  expect(deploy(menu).getAttribute("aria-disabled")).toBe("true");
  expect(deployed(menu)).toBeNull();

  const preview = await started(0);
  expect(preview.message!.preview).toBe(true);
  expect(preview.request.map_source.request).toMatchObject({
    type: "open",
    size: "small",
    seed: "77",
  });
  expect(preview.request.map_source.request.entries_per_side).toBe(1);
  await preview.admit();
  expect(menu.getByTestId("lobby-status").textContent).toBe("Ready seed 77");
  expect(menu.getByTestId("lobby-map").querySelector("svg")?.dataset.seed).toBe("77");
  expect(deployed(menu)).toMatchObject({
    map: { type: "open", size: "small", seed: "77" },
    players: preview.request.players,
  });
});

test("presets and Add/Delete set up any match, and the size follows the larger side", async () => {
  const menu = await openLobby();
  const size = (name: string) => menu.getByRole("radio", { name });
  fireEvent.click(menu.getByRole("radio", { name: "3v3" }));
  expect(menu.getByTestId("lobby-vs").textContent).toBe("3 VS 3");
  expect(banners(menu, "Blue")).toEqual(["1You", "2AI", "3AI"]);
  expect(size("L").getAttribute("aria-checked")).toBe("true");
  expect([size("S"), size("M")].every((s) => (s as HTMLButtonElement).disabled)).toBe(true);
  expect(menu.getByTestId("lobby-size-reason").textContent).toBe("S · M too small for 3 a side");

  // 3v4: one more on Red.
  fireEvent.click(menu.getByTestId("lobby-add-red"));
  expect(menu.getByTestId("lobby-vs").textContent).toBe("3 VS 4");
  expect(menu.getByRole("radio", { name: "3v3" }).getAttribute("aria-checked")).toBe("false");

  // 2v3: one less on each side. You and a side's last player can't be removed.
  fireEvent.click(menu.getByRole("button", { name: "Remove Blue 3" }));
  fireEvent.click(menu.getByRole("button", { name: "Remove Red 4" }));
  expect(menu.getByTestId("lobby-vs").textContent).toBe("2 VS 3");
  expect(side(menu, "Blue").queryByRole("button", { name: "Remove Blue 1" })).toBeNull();
  fireEvent.click(menu.getByRole("button", { name: "Remove Blue 2" }));
  for (const red of ["Red 3", "Red 2"])
    fireEvent.click(menu.getByRole("button", { name: `Remove ${red}` }));
  expect(menu.getByTestId("lobby-vs").textContent).toBe("1 VS 1");
  expect(menu.queryByRole("button", { name: /^Remove / })).toBeNull();
  // Fewer players never shrink the map; smaller sizes open again.
  expect(size("L").getAttribute("aria-checked")).toBe("true");
  expect((size("S") as HTMLButtonElement).disabled).toBe(false);

  fireEvent.click(menu.getByRole("radio", { name: "5v5" }));
  expect(menu.getByTestId("lobby-vs").textContent).toBe("5 VS 5");
  expect(size("XL").getAttribute("aria-checked")).toBe("true");
  expect(menu.queryByTestId("lobby-add-blue")).toBeNull();
  fireEvent.click(menu.getByRole("radio", { name: "1v1" }));
  expect(size("S").getAttribute("aria-checked")).toBe("true");
});

test("only a change to the map's inputs draws a new map", async () => {
  const menu = await openLobby();
  await (await started(0)).admit();
  fireEvent.click(menu.getByRole("radio", { name: "3v3" }));
  const threes = await started(1);
  expect(threes.request.map_source.request.entries_per_side).toBe(3);
  await threes.admit();
  expect(deployed(menu)).not.toBeNull();

  // You take Blue 2's seat; Red becomes European. The map stands.
  fireEvent.click(menu.getByTestId("lobby-controller-blue-2"));
  fireEvent.click(menu.getByTestId("lobby-controller-blue-2-you"));
  expect(banners(menu, "Blue")).toEqual(["1AI", "2You", "3AI"]);
  fireEvent.click(menu.getByTestId("lobby-faction-red"));
  fireEvent.click(menu.getByTestId("lobby-faction-red-europe"));
  expect(side(menu, "Red").getByTestId("lobby-faction-red").textContent).toBe("Red · European");
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(PreviewWorker.all).toHaveLength(2);
  expect(deployed(menu)?.players).toEqual(
    expect.arrayContaining([
      expect.objectContaining({ id: "blue-2", controller: "human" }),
      expect.objectContaining({ id: "red-1", faction: "europe" }),
    ]),
  );

  // The map's type, its region and a reroll each draw one; Deploy waits again.
  fireEvent.click(menu.getByRole("radio", { name: "metro" }));
  expect(deploy(menu).getAttribute("aria-disabled")).toBe("true");
  expect(threes.terminated).toBe(true);
  expect((await started(2)).request.map_source.request).toMatchObject({
    type: "metro",
    seed: threes.seed,
  });
  fireEvent.click(menu.getByTestId("menu-region"));
  fireEvent.click(menu.getByTestId("menu-region-paris"));
  expect(menu.getByTestId("menu-region").textContent).toBe("paris");
  expect((await started(3)).request.map_source.request.region).toBe("paris");
  fireEvent.click(menu.getByTestId("lobby-reroll"));
  expect((await started(4)).seed).not.toBe(threes.seed);
});

test("a refused map is redrawn quietly, and Deploy names the seed that was admitted", async () => {
  const menu = await openLobby("/?seed=19");
  const refused = await started(0);
  expect(refused.seed).toBe("19");
  await refused.refuse();
  expect(menu.getByTestId("lobby-status").textContent).toBe("Redrawing map · try 2");
  const admitted = await started(1);
  expect(admitted.seed).not.toBe("19");
  await admitted.admit();
  expect(deployed(menu)?.map.seed).toBe(admitted.seed);
  // Taking the admitted seed is not a new map.
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(PreviewWorker.all).toHaveLength(2);
});

test("a map that cannot be made says so, keeps Deploy closed and offers a reroll", async () => {
  const menu = await openLobby();
  await (await started(0)).refuse("invalid_request", "request");
  expect(menu.getByTestId("lobby-status").textContent).toBe("No map");
  expect(deploy(menu).getAttribute("aria-disabled")).toBe("true");
  fireEvent.click(menu.getByRole("button", { name: "Reroll" }));
  await (await started(1)).admit();
  expect(deploy(menu).getAttribute("href")).toMatch(/^\/battle\?/);
});
